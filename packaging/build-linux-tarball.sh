#!/usr/bin/env bash
set -euo pipefail
umask 022

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

version="${MORSE_RUNNER_BUILD_VERSION:-}"
package="${1:-}"
if [[ ! "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "Invalid MorseRunner build version: $version" >&2
  exit 1
fi
if [[ -z "$package" || ! -f "$package" ]]; then
  echo "Usage: MORSE_RUNNER_BUILD_VERSION=$version $0 dist/morse-runner_${version}_amd64.deb" >&2
  exit 1
fi

expected_package="morse-runner_${version}_amd64.deb"
if [[ "$(basename -- "$package")" != "$expected_package" ]]; then
  echo "Expected Debian package $expected_package, got $(basename -- "$package")" >&2
  exit 1
fi
if [[ "$(dpkg-deb -f "$package" Architecture)" != amd64 ]]; then
  echo "Linux tarball packaging requires an amd64 Debian package" >&2
  exit 1
fi
if [[ "$(dpkg-deb -f "$package" Version)" != "$version" ]]; then
  echo "Debian package version does not match $version" >&2
  exit 1
fi

export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct HEAD 2>/dev/null || date +%s)}"
work_dir="$(mktemp -d "$project_dir/target/linux-tarball.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT
dpkg-deb --extract "$package" "$work_dir/package"

binary="$work_dir/package/usr/bin/morse-runner"
documentation="$work_dir/package/usr/share/doc/morse-runner"
if [[ ! -x "$binary" ]]; then
  echo "Debian package does not contain executable usr/bin/morse-runner" >&2
  exit 1
fi
if [[ ! -d "$documentation" ]] || ! find "$documentation" -type f -print -quit | grep -q .; then
  echo "Debian package does not contain project documentation" >&2
  exit 1
fi
if ! find "$documentation" -type f -name '*.md' -print -quit | grep -q .; then
  echo "Debian package does not contain its Markdown documentation" >&2
  exit 1
fi

archive_root="morse-runner_${version}_linux_amd64"
stage="$work_dir/$archive_root"
mkdir -p "$stage"
install -m 755 "$binary" "$stage/morse-runner"
cp -a "$documentation/." "$stage/"

if find "$stage" -type f -path '*/man/*' -print -quit | grep -q .; then
  echo "Linux tarball must not contain a man page" >&2
  exit 1
fi

mkdir -p dist
archive="dist/${archive_root}.tar.gz"
tar --sort=name --format=gnu --mtime="@$SOURCE_DATE_EPOCH" \
  --owner=0 --group=0 --numeric-owner -cf - -C "$work_dir" "$archive_root" \
  | gzip -n -9 > "$archive"
tar -tzf "$archive" > "$work_dir/archive.list"
grep -Fxq "$archive_root/morse-runner" "$work_dir/archive.list"
echo "Created $archive"


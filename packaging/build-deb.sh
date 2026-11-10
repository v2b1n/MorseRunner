#!/usr/bin/env bash
set -euo pipefail
umask 022
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct HEAD 2>/dev/null || date +%s)}"
version="${MORSE_RUNNER_BUILD_VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)}"
build_kind="${MORSE_RUNNER_BUILD_KIND:-release}"
changelog_file="${MORSE_RUNNER_CHANGELOG_FILE:-}"
if [[ ! "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "Invalid MorseRunner build version: $version" >&2
  exit 1
fi
if [[ "$build_kind" != ci && "$build_kind" != release ]]; then
  echo "Invalid MorseRunner build kind: $build_kind" >&2
  exit 1
fi
export MORSE_RUNNER_BUILD_VERSION="$version"
scratch_dir=$(mktemp -d)
trap 'rm -rf "$scratch_dir"' EXIT
if [[ -z "$changelog_file" ]]; then
  # No stamped changelog from CI (local build): stamp CHANGELOG.md here.
  changelog_file="$scratch_dir/CHANGELOG.md"
  packaging/stamp-changelog.sh "$version" \
    "$(date -u -d "@$SOURCE_DATE_EPOCH" +%F)" CHANGELOG.md "$changelog_file"
fi
cargo build --release --locked
cargo about generate --fail about.hbs > target/THIRD_PARTY-LICENSES.md
arch="$(dpkg --print-architecture)"
package_root="$(mktemp -d "$project_dir/target/deb-root.XXXXXX")"
chmod 755 "$package_root"
install -Dm755 target/release/morse-runner "$package_root/usr/bin/morse-runner"
strip --strip-unneeded "$package_root/usr/bin/morse-runner"
install -Dm644 packaging/morse-runner.desktop "$package_root/usr/share/applications/morse-runner.desktop"
install -Dm644 packaging/morse-runner.1 "$package_root/usr/share/man/man1/morse-runner.1"
gzip -n -9 "$package_root/usr/share/man/man1/morse-runner.1"
install -Dm644 packaging/copyright "$package_root/usr/share/doc/morse-runner/copyright"
install -Dm644 LICENSE "$package_root/usr/share/doc/morse-runner/MPL-2.0"
install -Dm644 README.md "$package_root/usr/share/doc/morse-runner/README.md"
install -Dm644 "$changelog_file" "$package_root/usr/share/doc/morse-runner/CHANGELOG.md"
install -Dm644 SOURCE-MAP.md "$package_root/usr/share/doc/morse-runner/SOURCE-MAP.md"
install -Dm644 THIRD_PARTY.md "$package_root/usr/share/doc/morse-runner/THIRD_PARTY.md"
install -Dm644 target/THIRD_PARTY-LICENSES.md "$package_root/usr/share/doc/morse-runner/THIRD_PARTY-LICENSES.md"
entries_file="$scratch_dir/entries"
awk -v version="$version" -v commit="$(git rev-parse --short HEAD)" '
  function flush() {
    if (current != "") print "  * " current
    current = ""
  }
  !inside {
    if (index($0, "## [" version "]") == 1) inside = 1
    next
  }
  /^## / { exit }
  /^### / {
    flush()
    category = $0
    sub(/^###[[:space:]]+/, "", category)
    next
  }
  /^[[:space:]]*[-*][[:space:]]+/ {
    flush()
    item = $0
    sub(/^[[:space:]]*[-*][[:space:]]+/, "", item)
    gsub(/`/, "", item)
    current = category == "" ? item : category ": " item
    count++
    next
  }
  current != "" && /[^[:space:]]/ {
    sub(/^[[:space:]]+/, "")
    sub(/[[:space:]]+$/, "")
    current = current " " $0
  }
  END {
    flush()
    if (!inside || count == 0)
      print "  * CI build on commit " commit
  }
' "$changelog_file" > "$entries_file"

distribution=stable
[[ "$build_kind" == ci ]] && distribution=trixie
date=$(date -Ru -d "@$SOURCE_DATE_EPOCH")
{
  printf 'morse-runner (%s) %s; urgency=medium\n\n' "$version" "$distribution"
  cat "$entries_file"
  printf '\n -- Vladimir Vecgailis <vladimir@vovka.de>  %s\n' "$date"
} > "$package_root/usr/share/doc/morse-runner/changelog"
gzip -n -9 "$package_root/usr/share/doc/morse-runner/changelog"
install -d "$package_root/DEBIAN" dist
cat > "$package_root/DEBIAN/control" <<CONTROL
Package: morse-runner
Version: $version
Architecture: $arch
Maintainer: Vladimir Vecgailis <vladimir@vovka.de>
Section: hamradio
Priority: optional
Homepage: https://github.com/v2b1n/MorseRunner
Installed-Size: $(du -sk "$package_root/usr" | cut -f1)
Depends: libasound2t64, libc6 (>= 2.39), libgcc-s1, libx11-6, libxkbcommon0, libwayland-client0, libgl1
Description: Native Rust CW contest training simulator
 Desktop CW contest practice simulator with single-call and pile-up modes.
 Includes simulated radio conditions,
 keyboard operation, verified logs and CSV export.
CONTROL
dpkg-deb --root-owner-group --build "$package_root" "dist/morse-runner_${version}_${arch}.deb"


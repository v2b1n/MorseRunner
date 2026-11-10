#!/usr/bin/env bash
set -euo pipefail

version=$MORSE_RUNNER_BUILD_VERSION
tag="v$version"
release_dir=dist/release
mkdir -p "$release_dir"

cp dist/downloads/morse-runner-debian/morse-runner_*.deb "$release_dir/"
cp dist/downloads/morse-runner-debian/morse-runner_*_linux_amd64.tar.gz "$release_dir/"
cp dist/downloads/morse-runner-windows/morse-runner_*_windows_x86_64.zip "$release_dir/"
cp dist/downloads/morse-runner-windows/morse-runner_*_windows_x86_64_setup.exe "$release_dir/"
cp dist/downloads/morse-runner-macos-arm64/morse-runner_*_macos_arm64.dmg "$release_dir/"
cp dist/downloads/morse-runner-macos-x86_64/morse-runner_*_macos_x86_64.dmg "$release_dir/"
cp dist/downloads/morse-runner-metadata/CHANGELOG.md "$release_dir/CHANGELOG.md"
cp dist/downloads/morse-runner-metadata/BUILD-INFO.json "$release_dir/"

cd "$release_dir"
packages=(morse-runner_*.deb morse-runner_*_linux_amd64.tar.gz \
  morse-runner_*_windows_x86_64.zip morse-runner_*_windows_x86_64_setup.exe \
  morse-runner_*_macos_arm64.dmg morse-runner_*_macos_x86_64.dmg)
sha256sum "${packages[@]}" BUILD-INFO.json CHANGELOG.md > SHA256SUMS

cd "$GITHUB_WORKSPACE"
if gh release view "$tag" >/dev/null 2>&1 || git ls-remote --exit-code origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "Release tag $tag already exists" >&2
  exit 1
fi

git -c tag.gpgSign=false tag "$tag" "$GITHUB_SHA"
git push origin "refs/tags/$tag"

previous_main_sha=$(jq -r '.before // empty' "$GITHUB_EVENT_PATH")
previous_changelog=$(mktemp)
release_notes_file=dist/release-notes.md

if [[ -n "$previous_main_sha" && ! "$previous_main_sha" =~ ^0+$ ]]; then
  git show "$previous_main_sha:CHANGELOG.md" > "$previous_changelog"
fi

diff --new-line-format='%L' \
  --old-line-format='' \
  --unchanged-line-format='' \
  "$previous_changelog" CHANGELOG.md > "$release_notes_file" || diff_status=$?

if [[ "${diff_status:-0}" -gt 1 ]]; then
  echo "Could not compare CHANGELOG.md with the previous main state" >&2
  exit "$diff_status"
fi

rm -f "$previous_changelog"

gh release create "$tag" --draft --verify-tag \
  --title "MorseRunner $version" \
  --notes-file "$release_notes_file"
gh release upload "$tag" "$release_dir/"*
echo "Staged $tag as a draft release."


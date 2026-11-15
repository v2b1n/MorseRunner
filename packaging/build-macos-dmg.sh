#!/usr/bin/env bash
set -euo pipefail
umask 022

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

binary="${1:-}"
arch="${2:-}"
version="${MORSE_RUNNER_BUILD_VERSION:-}"
if [[ ! "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "Invalid MorseRunner build version: $version" >&2
  exit 1
fi
if [[ "$arch" != arm64 && "$arch" != x86_64 ]]; then
  echo "Architecture must be arm64 or x86_64" >&2
  exit 1
fi
if [[ ! -x "$binary" ]]; then
  echo "Expected an executable Rust build at: $binary" >&2
  exit 1
fi
if [[ ! -s dist/metadata/BUILD-INFO.json ]]; then
  echo "This run's BUILD-INFO.json is required in dist/metadata" >&2
  exit 1
fi
if [[ ! -s dist/metadata/CHANGELOG.md ]]; then
  echo "This run's stamped CHANGELOG.md is required in dist/metadata" >&2
  exit 1
fi
if [[ ! -s target/THIRD_PARTY-LICENSES.md ]]; then
  echo "Generated target/THIRD_PARTY-LICENSES.md is required" >&2
  exit 1
fi

actual_arch="$(lipo -archs "$binary")"
if [[ "$actual_arch" != "$arch" ]]; then
  echo "Expected a $arch executable, but lipo reports: $actual_arch" >&2
  exit 1
fi

deployment_target="10.15"
if [[ "$arch" == arm64 ]]; then
  deployment_target="11.0"
fi
if [[ -n "${MACOSX_DEPLOYMENT_TARGET:-}" && "$MACOSX_DEPLOYMENT_TARGET" != "$deployment_target" ]]; then
  echo "MACOSX_DEPLOYMENT_TARGET must be $deployment_target for $arch" >&2
  exit 1
fi

compiled_deployment_target="$(otool -l "$binary" | awk '
  $1 == "cmd" && $2 == "LC_BUILD_VERSION" { current = "build"; next }
  $1 == "cmd" && $2 == "LC_VERSION_MIN_MACOSX" { current = "legacy"; next }
  current == "build" && $1 == "minos" { print $2; exit }
  current == "legacy" && $1 == "version" { print $2; exit }
')"
if [[ "$compiled_deployment_target" != "$deployment_target" ]]; then
  echo "Expected a $deployment_target minimum macOS binary, got: ${compiled_deployment_target:-unknown}" >&2
  exit 1
fi

work_dir="$(mktemp -d "$project_dir/target/macos-dmg.XXXXXX")"
mount_point="$work_dir/mount"
mounted=0
cleanup() {
  if ((mounted)); then
    hdiutil detach "$mount_point" -quiet || true
  fi
  rm -rf "$work_dir"
}
trap cleanup EXIT

app="$work_dir/stage/MorseRunner.app"
contents="$app/Contents"
resources="$contents/Resources"
mkdir -p "$contents/MacOS" "$resources" "$mount_point" dist
install -m 755 "$binary" "$contents/MacOS/morse-runner"
install -m 644 LICENSE README.md THIRD_PARTY.md \
  dist/metadata/CHANGELOG.md dist/metadata/BUILD-INFO.json \
  target/THIRD_PARTY-LICENSES.md "$resources/"

cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>morse-runner</string>
  <key>CFBundleIdentifier</key><string>de.vovka.morserunner</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>MorseRunner</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>LSMinimumSystemVersion</key><string>$deployment_target</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
PLIST
/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$contents/Info.plist" >/dev/null
plutil -lint "$contents/Info.plist"

cp README.md "$work_dir/stage/README.md"
ln -s /Applications "$work_dir/stage/Applications"

dmg="dist/morse-runner_${version}_macos_${arch}.dmg"
hdiutil create \
  -volname "MorseRunner $version $arch" \
  -srcfolder "$work_dir/stage" \
  -format UDZO \
  -imagekey zlib-level=9 \
  -ov "$dmg"
hdiutil verify "$dmg"
hdiutil attach "$dmg" -readonly -nobrowse -noverify -mountpoint "$mount_point" >/dev/null
mounted=1

# Each check must stop the build explicitly. Bash 3.2, the macOS system shell,
# does not apply "set -e" to a failed [[ ... ]] command.
verify() {
  local description=$1
  shift
  if ! "$@"; then
    echo "Disk image verification failed: $description" >&2
    exit 1
  fi
}

mounted_app="$mount_point/MorseRunner.app"
mounted_binary="$mounted_app/Contents/MacOS/morse-runner"
mounted_plist="$mounted_app/Contents/Info.plist"
verify "the application executable is present" test -x "$mounted_binary"
verify "the Applications entry is a symbolic link" test -L "$mount_point/Applications"
verify "the Applications link points to /Applications" \
  test "$(readlink "$mount_point/Applications")" = /Applications
verify "the executable architecture is $arch" \
  test "$(lipo -archs "$mounted_binary")" = "$arch"
for document in LICENSE README.md THIRD_PARTY.md CHANGELOG.md BUILD-INFO.json THIRD_PARTY-LICENSES.md; do
  verify "$document is present in the application resources" \
    test -s "$mounted_app/Contents/Resources/$document"
done
verify "the bundle version is $version" \
  test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$mounted_plist")" = "$version"
verify "the minimum system version is $deployment_target" \
  test "$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$mounted_plist")" = "$deployment_target"

echo "Created and verified $dmg ($arch, macOS $deployment_target minimum)"

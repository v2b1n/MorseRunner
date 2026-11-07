#!/usr/bin/env bash
set -euo pipefail
umask 022
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct HEAD 2>/dev/null || date +%s)}"
version="${MORSE_RUNNER_BUILD_VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)}"
build_kind="${MORSE_RUNNER_BUILD_KIND:-release}"
changelog_file="${MORSE_RUNNER_CHANGELOG_FILE:-CHANGELOG.md}"
if [[ ! "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "Invalid MorseRunner build version: $version" >&2
  exit 1
fi
if [[ "$build_kind" != ci && "$build_kind" != release ]]; then
  echo "Invalid MorseRunner build kind: $build_kind" >&2
  exit 1
fi
export MORSE_RUNNER_BUILD_VERSION="$version"
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
python3 - "$version" "$build_kind" "$package_root/usr/share/doc/morse-runner/changelog" <<'PY'
import re
import os
import subprocess
import sys
import textwrap
from pathlib import Path

version, build_kind, destination = sys.argv[1:]
changelog_file = Path(os.environ.get("MORSE_RUNNER_CHANGELOG_FILE", "CHANGELOG.md"))
text = changelog_file.read_text(encoding="utf-8")
if build_kind == "ci":
    match = re.search(rf"^## \[{re.escape(version)}\](?:\s+-.*)?\s*$", text, re.M)
    distribution = "trixie"
else:
    match = re.search(rf"^## \[{re.escape(version)}\](?:\s+-.*)?\s*$", text, re.M)
    distribution = "stable"
if not match:
    raise SystemExit(f"{changelog_file} has no section for {version}")
next_section = re.search(r"^## ", text[match.end():], re.M)
section = text[match.end():match.end() + next_section.start()] if next_section else text[match.end():]
entries = []
category = ""
current = None

def save_current():
    if current is not None:
        entries.append(current)

for line in section.splitlines():
    heading = re.match(r"^###\s+(.+?)\s*$", line)
    if heading:
        save_current()
        current = None
        category = heading.group(1).rstrip(":")
        continue
    bullet = re.match(r"^\s*[-*]\s+(.+?)\s*$", line)
    if bullet:
        save_current()
        item = re.sub(r"`([^`]+)`", r"\1", bullet.group(1))
        current = f"{category}: {item}" if category else item
    elif current is not None and line.strip():
        current += " " + line.strip()
save_current()

wrapped_entries = []
for entry in entries:
    wrapped_entries.extend(textwrap.wrap(
        entry,
        width=78,
        initial_indent="  * ",
        subsequent_indent="    ",
        break_long_words=False,
        break_on_hyphens=False,
    ))
if not wrapped_entries:
    raise SystemExit(f"{changelog_file} section {version} has no bullet entries")
date = subprocess.check_output(
    ["date", "-Ru", "-d", f"@{os.environ['SOURCE_DATE_EPOCH']}"], text=True
).strip()
Path(destination).write_text(
    f"morse-runner ({version}) {distribution}; urgency=medium\n\n"
    + "\n".join(wrapped_entries)
    + f"\n\n -- Vladimir Vecgailis <vladimir@vovka.de>  {date}\n"
)
PY
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

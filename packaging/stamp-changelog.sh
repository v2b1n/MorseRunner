#!/usr/bin/env bash
# Stamp the "## [Unreleased]" section of a Keep a Changelog file with a build
# version and date, so the changelog shipped inside a package names the exact
# version of that package.
#
# The committed CHANGELOG.md never contains the build number (the patch
# component is the CI run number, which is unknown until the main build runs).
# CI stamps it once in prepare-build-metadata.sh; every package then ships the
# same stamped copy from dist/metadata/CHANGELOG.md.
#
# Usage: stamp-changelog.sh VERSION DATE INPUT OUTPUT
#   VERSION  MAJOR.MINOR.PATCH
#   DATE     YYYY-MM-DD
#   INPUT    committed CHANGELOG.md (must contain one "## [Unreleased]")
#   OUTPUT   stamped copy; "## [Unreleased]" becomes "## [VERSION] - DATE".
#            An Unreleased section without bullet entries gets the line
#            "_No user-visible changes._" so the section is never empty.
set -euo pipefail

if [[ $# -ne 4 ]]; then
  sed -n '/^# Usage:/,/^#   OUTPUT/p' "$0" | sed 's/^# \{0,1\}//' >&2
  exit 2
fi
version=$1
date=$2
input=$3
output=$4

if [[ ! "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "stamp-changelog: invalid version: $version" >&2
  exit 1
fi
if [[ ! "$date" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "stamp-changelog: date must be YYYY-MM-DD, got: $date" >&2
  exit 1
fi
if [[ ! -f "$input" ]]; then
  echo "stamp-changelog: $input not found" >&2
  exit 1
fi
if [[ "$(grep -c -E '^## \[Unreleased\][[:space:]]*$' "$input" || true)" -ne 1 ]]; then
  echo "stamp-changelog: $input must contain exactly one '## [Unreleased]' heading" >&2
  exit 1
fi

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

awk -v version="$version" -v date="$date" '
  function finish_section() {
    if (!has_entries) {
      body = "\n_No user-visible changes._\n\n"
    }
    printf "%s", body
    inside = 0
  }
  /^## \[Unreleased\][[:space:]]*$/ {
    print "## [" version "] - " date
    inside = 1
    body = ""
    has_entries = 0
    next
  }
  inside && /^## / { finish_section() }
  inside {
    body = body $0 "\n"
    if ($0 ~ /^[[:space:]]*[-*][[:space:]]+[^[:space:]]/) has_entries = 1
    next
  }
  { print }
  END { if (inside) finish_section() }
' "$input" > "$tmp"

if [[ "$(grep -c -F "## [$version]" "$tmp" || true)" -gt 1 ]]; then
  echo "stamp-changelog: warning: $input already has a '## [$version]' section" >&2
fi
if ! grep -q -E '^[[:space:]]*[-*][[:space:]]+[^[:space:]]' <(awk -v v="$version" '
    !inside { if (index($0, "## [" v "]") == 1) inside = 1; next }
    /^## / { exit }
    { print }' "$tmp"); then
  if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
    echo "::warning title=Empty changelog::No entries under [Unreleased] in $input; $version has no user-visible changes"
  else
    echo "stamp-changelog: warning: no entries under [Unreleased]" >&2
  fi
fi

mkdir -p "$(dirname -- "$output")"
cp "$tmp" "$output"

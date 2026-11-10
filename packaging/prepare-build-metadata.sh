#!/usr/bin/env bash
set -euo pipefail

base_version=$(awk '
  /^\[package\]/ { package = 1; next }
  /^\[/ { package = 0 }
  package && /^version = / {
    sub(/^version = "/, "")
    sub(/"$/, "")
    print
    exit
  }
' Cargo.toml)
run_number=$GITHUB_RUN_NUMBER
version="${base_version%.*}.$run_number"
source_sha=$GITHUB_SHA
build_timestamp=$(date -u +'%Y-%m-%dT%H:%M:%SZ')

if [[ "$GITHUB_EVENT_NAME" == pull_request ]]; then
  source_ref=pull_request
elif [[ "$GITHUB_EVENT_NAME" == push ]]; then
  source_ref=${GITHUB_REF#refs/heads/}
else
  source_ref=$GITHUB_REF_NAME
fi

attestations=false
if [[ "$GITHUB_EVENT_NAME" == push && "$GITHUB_REF" == refs/heads/main ]]; then
  attestations=true
fi

mkdir -p dist/metadata
# Stamp [Unreleased] with this run's version once; every package ships this copy.
# The date is the build commit's UTC date, so it is reproducible.
changelog_date=$(date -u -d "@$(git log -1 --format=%ct "$source_sha")" +%F)
bash packaging/stamp-changelog.sh "$version" "$changelog_date" \
  CHANGELOG.md dist/metadata/CHANGELOG.md

jq -n \
  --arg version "$version" \
  --arg source_ref "$source_ref" \
  --arg source_sha "$source_sha" \
  --arg build_timestamp "$build_timestamp" \
  --argjson run_number "$run_number" \
  --argjson attestations "$attestations" \
  '{version:$version, kind:"ci", run_number:$run_number,
    source_ref:$source_ref, source_sha:$source_sha,
    build_timestamp:$build_timestamp,
    github_artifact_attestations:$attestations}' \
  > dist/metadata/BUILD-INFO.json

printf 'version=%s\n' "$version" >> "$GITHUB_OUTPUT"


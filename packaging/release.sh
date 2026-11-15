#!/bin/bash
# Sign the checksum manifest of a staged draft release and publish the release.
#
# Usage: release.sh VERSION
#   VERSION  MAJOR.MINOR.PATCH of the draft release, without the "v" prefix
set -euo pipefail

if [[ $# -ne 1 || ! "$1" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "Usage: $0 MAJOR.MINOR.PATCH" >&2
  exit 2
fi

RELEASE_VERSION=$1
TAG="v$RELEASE_VERSION"
REPO="v2b1n/MorseRunner"
SIGNING_KEY=C73C58F2A4662E48537C77CD17E233FD523708C0

OUT=$(mktemp -d -p /dev/shm)
trap 'rm -rf "$OUT"' EXIT
cd "$OUT"

gh release download "$TAG" -R "$REPO" -p SHA256SUMS
gpg --armor --local-user "$SIGNING_KEY" --detach-sign SHA256SUMS
gpg --verify SHA256SUMS.asc SHA256SUMS
gh release upload "$TAG" -R "$REPO" SHA256SUMS.asc
gh release edit "$TAG" -R "$REPO" --draft=false --prerelease=false --latest

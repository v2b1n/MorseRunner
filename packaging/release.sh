#!/bin/bash

RELEASE_VERSION=$1
REPO="v2b1n/MorseRunner"

OUT=$(mktemp -d -p /dev/shm)

cd $OUT && \
gh release download v${RELEASE_VERSION} -R $REPO -p SHA256SUMS && \
gpg --armor --local-user C73C58F2A4662E48537C77CD17E233FD523708C0 --detach-sign SHA256SUMS && \
gpg --verify SHA256SUMS.asc SHA256SUMS && \
gh release upload v${RELEASE_VERSION} -R $REPO SHA256SUMS.asc && \
gh release edit "v${RELEASE_VERSION}" -R $REPO --draft=false --prerelease=false --latest

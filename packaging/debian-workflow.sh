#!/bin/bash
set -euo pipefail

DOCKER_IMAGE="v2b1n/morse-runner-builder:latest"

time docker run --rm \
  --env "MORSE_RUNNER_BUILD_VERSION=$MORSE_RUNNER_BUILD_VERSION" \
  --env "MORSE_RUNNER_BUILD_KIND=$MORSE_RUNNER_BUILD_KIND" \
  --env MORSE_RUNNER_CHANGELOG_FILE=/work/dist/metadata/CHANGELOG.md \
  --volume "$PWD:/work" \
  --workdir /work \
  $DOCKER_IMAGE \
  bash -euo pipefail -c '
    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked
    ./packaging/build-deb.sh
    lintian --tag-display-limit 0 --fail-on error dist/morse-runner_*_amd64.deb
    ./packaging/build-linux-tarball.sh "dist/morse-runner_${MORSE_RUNNER_BUILD_VERSION}_amd64.deb"
  '

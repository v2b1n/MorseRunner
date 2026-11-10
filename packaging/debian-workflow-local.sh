#!/bin/bash
set -euo pipefail

DOCKER_IMAGE="morse-runner-builder:latest"
MORSE_RUNNER_BUILD_VERSION=${MORSE_RUNNER_BUILD_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}
MORSE_RUNNER_BUILD_KIND=${MORSE_RUNNER_BUILD_KIND:-ci}

time docker run --rm \
  --env "MORSE_RUNNER_BUILD_VERSION=$MORSE_RUNNER_BUILD_VERSION" \
  --env "MORSE_RUNNER_BUILD_KIND=$MORSE_RUNNER_BUILD_KIND" \
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
    chown -R "$(stat -c '%u:%g' /work)" /work/target /work/dist
  '

#!/bin/bash
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install --yes --no-install-recommends \
  build-essential ca-certificates curl dpkg-dev git lintian \
  libasound2-dev libgl1-mesa-dev libwayland-dev \
  libx11-dev libxcursor-dev libxi-dev libxinerama-dev \
  libxkbcommon-dev libxrandr-dev pkg-config
git config --global --add safe.directory /work
curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain stable
export PATH="$HOME/.cargo/bin:$PATH"
rustup component add rustfmt clippy
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo install --locked --features cli cargo-about
./packaging/build-deb.sh
lintian --tag-display-limit 0 --fail-on error dist/morse-runner_*_amd64.deb
./packaging/build-linux-tarball.sh "dist/morse-runner_${MORSE_RUNNER_BUILD_VERSION}_amd64.deb"

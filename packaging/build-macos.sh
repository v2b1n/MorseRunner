#!/usr/bin/env bash
set -euo pipefail

rustup target add "$RUST_TARGET"
cargo fmt --all -- --check
cargo clippy --locked --all-targets --target "$RUST_TARGET" -- -D warnings
cargo test --locked --target "$RUST_TARGET"
cargo build --locked --release --target "$RUST_TARGET"
cargo install --locked --features cli cargo-about
cargo about generate --fail about.hbs --output-file target/THIRD_PARTY-LICENSES.md
./packaging/build-macos-dmg.sh \
  "target/$RUST_TARGET/release/morse-runner" \
  "$MORSE_RUNNER_MACOS_ARCH"


FROM debian:trixie-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install --yes --no-install-recommends \
      build-essential ca-certificates curl dpkg-dev git lintian python3 \
      libasound2-dev libgl1-mesa-dev libwayland-dev \
      libx11-dev libxcursor-dev libxi-dev libxinerama-dev \
      libxkbcommon-dev libxrandr-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN git config --global --add safe.directory /work

RUN curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
      -y --profile minimal --default-toolchain stable

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup component add rustfmt clippy \
    && cargo install --locked --features cli cargo-about

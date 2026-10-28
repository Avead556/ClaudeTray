FROM rust:1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev libdbus-1-dev \
      mingw-w64 \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-pc-windows-gnu && rustup component add clippy rustfmt
WORKDIR /src/desktop

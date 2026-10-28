#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

IMAGE=claudetray-rust
docker image inspect "$IMAGE" >/dev/null 2>&1 || docker build -t "$IMAGE" .

run() {
    docker run --rm \
        -v "$PWD/..:/src" \
        -v claudetray-cargo:/usr/local/cargo/registry \
        -v claudetray-target:/src/desktop/target \
        "$IMAGE" bash -c "$1"
}

case "${1:-all}" in
    check)
        run "cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo clippy --target x86_64-pc-windows-gnu -- -D warnings"
        ;;
    all)
        run "cargo build --release && cargo build --release --target x86_64-pc-windows-gnu \
            && mkdir -p dist \
            && cp target/release/claudetray dist/claudetray-linux-\$(uname -m) \
            && cp target/x86_64-pc-windows-gnu/release/claudetray.exe dist/claudetray-windows-x86_64.exe"
        ls -la dist
        ;;
    *)
        echo "usage: $0 [check | all]" >&2
        exit 2
        ;;
esac

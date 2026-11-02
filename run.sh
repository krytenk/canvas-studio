#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -x "$SCRIPT_DIR/canvas-studio" ]; then
    exec "$SCRIPT_DIR/canvas-studio" "$@"
elif [ -x "$SCRIPT_DIR/target/release/canvas-studio" ]; then
    exec "$SCRIPT_DIR/target/release/canvas-studio" "$@"
elif [ -x "$SCRIPT_DIR/target/debug/canvas-studio" ]; then
    exec "$SCRIPT_DIR/target/debug/canvas-studio" "$@"
else
    if ! command -v cargo >/dev/null 2>&1 && [ -n "${HOME:-}" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Install Rust to build from source, or download a binary from GitHub Releases." >&2
        exit 1
    fi
    cargo run --release --manifest-path "$SCRIPT_DIR/Cargo.toml" -- "$@"
fi

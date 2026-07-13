#!/usr/bin/env bash
set -e
cargo build --release
echo ""
echo "Build OK — target/release/pbscript"

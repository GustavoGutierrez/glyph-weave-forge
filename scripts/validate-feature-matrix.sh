#!/usr/bin/env bash
set -euo pipefail

cargo test
cargo test --doc
cargo test --no-default-features
cargo test --no-default-features --features renderer-minimal
cargo test --no-default-features --features renderer-typst
cargo test --no-default-features --features typst
cargo test --features renderer-typst
cargo test --all-features

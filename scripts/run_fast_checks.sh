#!/bin/sh
set -eu

cargo test --workspace
cargo clippy --workspace --all-targets

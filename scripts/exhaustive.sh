#!/bin/sh
# Every four-byte pattern on every target; no sampling or early-prefix skips.
set -eu
cargo build --release --example exhaustive
for target in 0 1 2 3 4; do
    target/release/examples/exhaustive "$target" 0 65536 "${H8_SWEEP_JOBS:-2}"
done

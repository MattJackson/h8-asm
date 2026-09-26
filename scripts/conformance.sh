#!/bin/sh
# All oracle tests are mandatory in this invocation. Missing tools fail tests.
set -eu
prefix=${1:-target/h8-binutils/install/bin}
prefix=$(cd "$prefix" && pwd)
export H8_AS="$prefix/h8300-elf-as"
export H8_LD="$prefix/h8300-elf-ld"
export H8_OBJCOPY="$prefix/h8300-elf-objcopy"
export H8_OBJDUMP="$prefix/h8300-elf-objdump"
cargo test --test binutils_assemble_back --test binutils_sx_assemble_back \
    --test binutils_first_word --test binutils_legacy --test binutils_planning --test binutils_shared --test binutils_sx_shared -- --ignored --nocapture

# Whole H8SX table provenance and raw-row oracle, including reviewed anomalies.
python3 scripts/compile-h8sx-table.py --check
python3 scripts/probe-h8sx-table.py --tools "$prefix" --output target/h8sx-row-probe.json

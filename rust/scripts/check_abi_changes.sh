#!/usr/bin/env bash
set -euo pipefail

################################################################################
# Purpose
#
# Verify that the `BINARYEN_FFI_ABI_VERSION` in `include/binaryen_ffi.h` and the
# Rust constant `BINARYEN_FFI_ABI_VERSION` in `rust/binaryen-ffi/src/lib.rs` are
# the same. This prevents accidental ABI mismatches between the golden header
# and the compiled runtime library.
#
# This script should be run as part of pull-request checks and locally before
# submitting changes that affect the FFI contract.
#
################################################################################
# How this script sits alongside other tooling
#
# - Use `rust/scripts/check_cbindgen.sh` to confirm the SPINE of the generated
#   header is consistent with the golden file. Combined with the ABI check,
#   these scripts ensure the header and runtime both agree on ABI versions.
# - The script is intended to be invoked by CI (see `.github/workflows/rust-abi-check.yml`)
#   as well as used by maintainers locally.
################################################################################

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

HEADER_PATH="include/binaryen_ffi.h"
RUST_SRC="rust/binaryen-ffi/src/lib.rs"

if [[ ! -f "$HEADER_PATH" ]]; then
  echo "ABI header not found: $HEADER_PATH" >&2
  exit 2
fi
if [[ ! -f "$RUST_SRC" ]]; then
  echo "Rust lib source not found: $RUST_SRC" >&2
  exit 2
fi

header_val=$(grep -E '^#define BINARYEN_FFI_ABI_VERSION' "$HEADER_PATH" | awk '{print $3}' | tr -d '\r') || header_val=""
rust_val=$(grep -E '^pub const BINARYEN_FFI_ABI_VERSION' "$RUST_SRC" | sed -E 's/.*= *([0-9]+).*/\1/' | tr -d '\r') || rust_val=""

if [[ -z "$header_val" ]]; then
  echo "Unable to parse BINARYEN_FFI_ABI_VERSION from $HEADER_PATH" >&2
  exit 2
fi
if [[ -z "$rust_val" ]]; then
  echo "Unable to parse BINARYEN_FFI_ABI_VERSION from $RUST_SRC" >&2
  exit 2
fi

if [[ "$header_val" != "$rust_val" ]]; then
  echo "ABI mismatch: header ($HEADER_PATH) = $header_val, rust runtime value ($RUST_SRC) = $rust_val" >&2
  echo "If you changed exported symbols or types, bump both the `BINARYEN_FFI_ABI_VERSION` in header and the rust `BINARYEN_FFI_ABI_VERSION` constant, and update the changelog and PR reviewers." >&2
  exit 1
fi

echo "ABI check passed: header and rust ABI version both = $header_val"

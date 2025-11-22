#!/usr/bin/env bash
set -euo pipefail

################################################################################
# Purpose
#
# This script validates that `cbindgen` output for the `binaryen-ffi` crate
# matches the committed golden header `include/binaryen_ffi.h`. It is a safeguard
# that ensures any changes to the exported FFI surface are intentional and
# reviewed before being merged.
#
# The script is intended for contributors and CI flows. Contributors should run
# this script locally after changing any `#[repr(C)]` types or `extern "C"`
# functions, and CI runs this script to enforce the golden file in pull requests.
#
################################################################################
# How this script sits alongside other tooling
#
# - `rust/scripts/update_cbindgen.sh` should be used when intentionally
#   updating the golden header. It regenerates the header and instructs the
#   maintainer to commit the result. This script (check) is the counterpart for
#   verifying that the generated header matches the golden file and will fail
#   the CI if it doesn't.
# - `rust/scripts/check_abi_changes.sh` complements this check by ensuring the
#   runtime ABI constant in Rust (`BINARYEN_FFI_ABI_VERSION`) is in sync with
#   the header macro `BINARYEN_FFI_ABI_VERSION` to avoid accidental ABI
#   mismatches.
################################################################################

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# Ensure cbindgen is available
if ! command -v cbindgen > /dev/null; then
  echo "cbindgen not found, install it with 'cargo install --locked cbindgen'"
  exit 1
fi

TMP_HEADER=$(mktemp)
cd rust/binaryen-ffi
cbindgen --config cbindgen.toml --crate binaryen-ffi --output "$TMP_HEADER"

cd "$ROOT"
if ! git diff --no-index --exit-code include/binaryen_ffi.h "$TMP_HEADER"; then
  echo "Generated header differs from committed golden header. If intentional, update include/binaryen_ffi.h after review."
  exit 1
fi

echo "cbindgen match: include/binaryen_ffi.h matches generated output"

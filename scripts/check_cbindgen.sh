#!/usr/bin/env bash
set -euo pipefail

# Generate header via cbindgen and compare to the committed golden header
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

# Ensure cbindgen is available
if ! command -v cbindgen > /dev/null; then
  echo "cbindgen not found, install it with 'cargo install --locked cbindgen'"
  exit 1
fi

# Generate header to a temp file
TMP_HEADER=$(mktemp)
cd rust/binaryen-ffi
cbindgen --config cbindgen.toml --crate binaryen-ffi --output "$TMP_HEADER"

# Compare with golden header
cd "$ROOT"
if ! git diff --no-index --exit-code include/binaryen_ffi.h "$TMP_HEADER"; then
  echo "Generated header differs from committed golden header. If intentional, update include/binaryen_ffi.h after review."
  exit 1
fi

echo "cbindgen match: include/binaryen_ffi.h matches generated output"

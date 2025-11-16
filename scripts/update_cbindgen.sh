#!/usr/bin/env bash
set -euo pipefail

# Regenerate the header from the rust crate and replace the golden header
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

if ! command -v cbindgen > /dev/null; then
  echo "cbindgen not found. Install with: cargo install --locked cbindgen"
  exit 1
fi

TMP=$(mktemp)
cd rust/binaryen-ffi
cbindgen --config cbindgen.toml --crate binaryen-ffi --output "$TMP"

echo "Comparing new header with committed golden header..."
if git diff --no-index --quiet include/binaryen_ffi.h "$TMP"; then
  echo "No changes to golden header"
  rm -f "$TMP"
  exit 0
fi

echo "Header differs from golden header. Updating include/binaryen_ffi.h"
mv "$TMP" include/binaryen_ffi.h
echo "Updated include/binaryen_ffi.h — please review and commit the change."

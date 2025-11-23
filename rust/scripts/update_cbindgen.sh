#!/usr/bin/env bash
set -euo pipefail

################################################################################
# Purpose
#
# Regenerate the `include/binaryen_ffi.h` golden header using `cbindgen` for the
# `binaryen-ffi` crate and then replace the committed golden header. This is a
# convenience script for maintainers who intentionally change the FFI surface.
#
# It is the complement to `rust/scripts/check_cbindgen.sh` which verifies the
# header matches the generated output; use `update` when you want to adopt a
# new ABI or header changes and commit them.
#
################################################################################
# How this script sits alongside other tooling
#
# - After edits to the Rust FFI crate, run `rust/scripts/update_cbindgen.sh` to
#   regenerate `include/binaryen_ffi.h`. Review the output manually, and if the
#   change is intended, commit the updated header along with a bump to ABI and
#   changelog notes as necessary.
# - Use `rust/scripts/check_cbindgen.sh` on CI to validate that contributors
#   didn't forget to update the golden header when editing the Rust FFI.
################################################################################

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if ! command -v cbindgen > /dev/null; then
  echo "cbindgen not found. Install with: cargo install --locked cbindgen"
  exit 1
fi

TMP=$(mktemp)
cd rust/binaryen-ffi
cbindgen --config cbindgen.toml --crate binaryen-ffi --output "$TMP"

cd "$ROOT"
echo "Comparing new header with committed golden header..."
if git diff --no-index --quiet include/binaryen_ffi.h "$TMP"; then
  echo "No changes to golden header"
  rm -f "$TMP"
  exit 0
fi

echo "Header differs from golden header. Updating include/binaryen_ffi.h"
# If the generated header doesn't contain the INTERNAL-ONLY warning, prepend
# our internal-only guidance to discourage external usage.
if ! grep -q "INTERNAL-ONLY: WARNING" "$TMP"; then
  echo "Detected cbindgen output without internal header notice. Prepending internal notice."
  CAT_TMP=$(mktemp)
  cat "$ROOT/rust/scripts/internal_header_notice.txt" "$TMP" > "$CAT_TMP"
  mv "$CAT_TMP" include/binaryen_ffi.h
else
  mv "$TMP" include/binaryen_ffi.h
fi
echo "Updated include/binaryen_ffi.h — please review and commit the change."

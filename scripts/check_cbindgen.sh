#!/usr/bin/env bash
set -euo pipefail

# Deprecated shim for backward compatibility. See rust/scripts/check_cbindgen.sh
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

echo "Deprecated: use 'rust/scripts/check_cbindgen.sh' instead of 'scripts/check_cbindgen.sh'" >&2
exec bash "$ROOT/rust/scripts/check_cbindgen.sh" "$@"

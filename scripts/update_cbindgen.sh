#!/usr/bin/env bash
set -euo pipefail

# Deprecated shim for backward compatibility. Forward to canonical script in rust/scripts.
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
echo "Deprecated: use 'rust/scripts/update_cbindgen.sh' instead of 'scripts/update_cbindgen.sh'" >&2
exec bash "$ROOT/rust/scripts/update_cbindgen.sh" "$@"

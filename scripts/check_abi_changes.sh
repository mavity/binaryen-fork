#!/usr/bin/env bash
set -euo pipefail

# Backward-compatibility shim
# This file used to contain the canonical ABI check implementation. The
# canonical copy is now in `rust/scripts/check_abi_changes.sh`. Keep this
# shim to avoid breaking scripts that call `scripts/check_abi_changes.sh`.

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

echo "Deprecated: use 'rust/scripts/check_abi_changes.sh' instead of 'scripts/check_abi_changes.sh'" >&2
exec bash "$ROOT_DIR/rust/scripts/check_abi_changes.sh" "$@"

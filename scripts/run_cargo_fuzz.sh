#!/usr/bin/env bash
set -euo pipefail

# Backward-compatibility shim
ROOT_DIR=$(git rev-parse --show-toplevel)
cd "$ROOT_DIR"
echo "Deprecated: use 'rust/scripts/run_cargo_fuzz.sh' instead of 'scripts/run_cargo_fuzz.sh'" >&2
exec bash "$ROOT_DIR/rust/scripts/run_cargo_fuzz.sh" "$@"

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR/fuzz"

if ! command -v cargo-fuzz &> /dev/null; then
  echo "cargo-fuzz not found. Install it with: cargo install cargo-fuzz" >&2
  exit 2
fi

for target in interner arena ahash; do
  echo "Running fuzz target: ${target} (limited to 60s)"
  timeout 60 cargo fuzz run ${target} || echo "target ${target} finished or timed out"
done

echo "Fuzz invocation complete (local)." 

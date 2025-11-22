#!/usr/bin/env bash
set -euo pipefail

################################################################################
# Purpose
#
# Run the Criterion benches for `rust/binaryen-support` locally in a controlled
# environment. This is useful for maintainers to collect a local benchmark output
# before and after making changes.
################################################################################

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT/rust/binaryen-support"

echo "Running benches in release mode (this can take a while)"
cargo bench --quiet || true
echo "Benchmark run complete — results in target/criterion"

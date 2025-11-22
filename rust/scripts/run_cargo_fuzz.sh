#!/usr/bin/env bash
set -euo pipefail

################################################################################
# Purpose
#
# Wrapper script to run `cargo-fuzz` targets for the rust support crates. This
# convenience script enables local developers to quickly run fuzz targets for
# `interner`, `arena`, and `ahash` with a short time limit. It is not intended
# to replace production fuzzing infrastructure, but to provide a low-friction
# starting point for manual testing.
#
# The script is intended to be used locally by developers while working on the
# rust crates and by maintainers during quick validation. For long-running
# fuzzing with persistent corpus handling, see the `fuzz/` workspace and CI
# workflows.
#
################################################################################
# How this script sits alongside other tooling
#
# - The `fuzz/` folder contains integrated `cargo-fuzz` targets and corpus
#   artifacts. This script wraps `cargo fuzz run` with a bounded timeout for
#   quick local validation.
# - For CI usage, we provide `.github/workflows/rust-fuzz.yml` which can be
#   used ad-hoc (`workflow_dispatch`) for manual fuzz runs to avoid heavy
#   compute costs on every push.
################################################################################

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT/fuzz"

if ! command -v cargo-fuzz &> /dev/null; then
  echo "cargo-fuzz not found. Install it with: cargo install cargo-fuzz" >&2
  exit 2
fi

for target in interner arena ahash fastmap; do
  echo "Running fuzz target: ${target} (limited to 60s)"
  timeout 60 cargo +nightly fuzz run ${target} || echo "target ${target} finished or timed out"
done

echo "Fuzz invocation complete (local)."

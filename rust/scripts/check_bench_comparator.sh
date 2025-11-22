#!/usr/bin/env bash
set -euo pipefail

# Small helper to compare Criterion bench results for a specific benchmark.
# It expects two directories: current and baseline. If default baseline is not
# provided, the script prints the current mean and exits 0.

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <current_bench_dir> [baseline_bench_dir]"
  exit 2
fi

CURRENT_DIR=$1
BASELINE_DIR=${2:-}

if [[ ! -d "$CURRENT_DIR" ]]; then
  echo "Current bench dir not found: $CURRENT_DIR" >&2
  exit 2
fi

function extract_mean() {
  local dir=$1
  jq -r '.[] | .[0].mean_estimates.mean' <<< "$(jq -s '.[].benchmarks' <(jq '.benchmarks' ${dir}/*/new/estimates.json))" 2>/dev/null | head -n 1
}

CURRENT_MEAN=$(extract_mean "$CURRENT_DIR" || echo "")
if [[ -z "$CURRENT_MEAN" ]]; then
  echo "Unable to extract mean from current bench dir: $CURRENT_DIR" >&2
  exit 2
fi

if [[ -z "$BASELINE_DIR" ]]; then
  echo "Current bench mean: $CURRENT_MEAN"
  exit 0
fi

if [[ ! -d "$BASELINE_DIR" ]]; then
  echo "Baseline bench dir not found: $BASELINE_DIR" >&2
  exit 2
fi

BASELINE_MEAN=$(extract_mean "$BASELINE_DIR" || echo "")
if [[ -z "$BASELINE_MEAN" ]]; then
  echo "Unable to extract mean from baseline bench dir: $BASELINE_DIR" >&2
  exit 2
fi

echo "Current mean: $CURRENT_MEAN"
echo "Baseline mean: $BASELINE_MEAN"

# Calculate relative change
awk -v a="$CURRENT_MEAN" -v b="$BASELINE_MEAN" 'BEGIN{ if (b == 0) { print "inf"; exit 0 } print (a - b) / b }'

exit 0

#!/usr/bin/env bash

set -euo pipefail

REPORT_ONLY=false
MIN_LINES="${IZEL_MIN_COVERAGE:-100}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --report-only)
            REPORT_ONLY=true
            shift
            ;;
        --min-lines)
            MIN_LINES="${2:-}"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Usage: $0 [--report-only] [--min-lines <percent>]"
            exit 2
            ;;
    esac
done

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "[missing] cargo-llvm-cov is not installed."
    echo "Install with: cargo install cargo-llvm-cov --locked"
    exit 1
fi

echo "== Izel Coverage Check =="
echo "Target minimum line coverage: ${MIN_LINES}%"

output="$(cargo llvm-cov --workspace --all-features --summary-only 2>&1)"
echo "$output"

total_line="$(grep -E '^TOTAL' <<<"$output" | tail -n1 || true)"
if [[ -z "$total_line" ]]; then
    echo "[error] Could not find TOTAL line in coverage output."
    exit 1
fi

line_percent_raw="$(grep -Eo '[0-9]+(\.[0-9]+)?%' <<<"$total_line" | tail -n1 || true)"
if [[ -z "$line_percent_raw" ]]; then
    echo "[error] Could not parse line coverage percentage from: $total_line"
    exit 1
fi

line_percent="${line_percent_raw%%%}"

if [[ "$REPORT_ONLY" == true ]]; then
    echo "Report-only mode: measured line coverage is ${line_percent}%"
    exit 0
fi

if awk -v got="$line_percent" -v want="$MIN_LINES" 'BEGIN { exit !(got + 0 >= want + 0) }'; then
    echo "[ok] line coverage ${line_percent}% >= ${MIN_LINES}%"
    exit 0
fi

echo "[low] line coverage ${line_percent}% < ${MIN_LINES}%"
exit 1

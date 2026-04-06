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

MIN_LINES="${MIN_LINES//%/}"
MIN_LINES="${MIN_LINES//[[:space:]]/}"

if ! awk -v want="$MIN_LINES" 'BEGIN { exit !(want ~ /^[0-9]+([.][0-9]+)?$/) }'; then
    echo "[error] --min-lines must be a numeric percentage (for example: 95 or 100)."
    exit 2
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "[missing] cargo-llvm-cov is not installed."
    echo "Install with: cargo install cargo-llvm-cov --locked"
    exit 1
fi

echo "== Izel Coverage Check =="
echo "Target minimum line coverage: ${MIN_LINES}%"

mkdir -p target/coverage
lcov_file="target/coverage/check_coverage.lcov"
uncovered_file="target/coverage/check_coverage.uncovered.txt"
output="$(cargo llvm-cov --workspace --all-features --lcov --output-path "$lcov_file" 2>&1)"
echo "$output"

if [[ ! -f "$lcov_file" ]]; then
    echo "[error] Expected LCOV output file was not generated: $lcov_file"
    exit 1
fi

: >"$uncovered_file"
read -r covered_lines total_lines uncovered_lines line_percent <<<"$(awk -F'[:,]' -v out="$uncovered_file" '
    /^SF:/ {
        file = substr($0, 4);
        next;
    }
    /^DA:/ {
        total += 1;
        line = $2 + 0;
        count = $3 + 0;
        if (count > 0) {
            covered += 1;
        } else {
            uncovered += 1;
            if (file != "") {
                printf "%s:%d\n", file, line >> out;
            }
        }
    }
    END {
        if (total == 0) {
            printf "0 0 0 0.00";
        } else {
            printf "%d %d %d %.2f", covered, total, uncovered, (covered * 100.0) / total;
        }
    }
' "$lcov_file")"

if [[ -z "$line_percent" ]]; then
    echo "[error] Could not compute line coverage percentage from LCOV data."
    exit 1
fi

echo "LCOV covered lines: ${covered_lines}/${total_lines}"
echo "LCOV uncovered lines: ${uncovered_lines}"

if [[ "$REPORT_ONLY" == true ]]; then
    echo "Report-only mode: measured line coverage is ${line_percent}%"
    exit 0
fi

if awk -v want="$MIN_LINES" 'BEGIN { exit !((want + 0) >= 100) }' && [[ "$uncovered_lines" != "0" ]]; then
    echo "[low] uncovered lines remain (${uncovered_lines}); see $uncovered_file"
    exit 1
fi

if awk -v covered="$covered_lines" -v total="$total_lines" -v want="$MIN_LINES" 'BEGIN {
    if (total == 0) {
        exit !(0 >= want + 0)
    }
    pct = (covered * 100.0) / total;
    exit !(pct >= want + 0)
}'; then
    echo "[ok] line coverage ${line_percent}% >= ${MIN_LINES}%"
    exit 0
fi

echo "[low] line coverage ${line_percent}% < ${MIN_LINES}%"
exit 1

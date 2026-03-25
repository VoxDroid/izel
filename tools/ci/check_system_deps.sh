#!/usr/bin/env bash

set -euo pipefail

REPORT_ONLY=false
if [[ "${1:-}" == "--report-only" ]]; then
    REPORT_ONLY=true
fi

failures=0

version_ge() {
    local got="$1"
    local want="$2"
    if [[ "$(printf '%s\n%s\n' "$want" "$got" | sort -V | head -n1)" == "$want" ]]; then
        return 0
    fi
    return 1
}

extract_first_version() {
    local text="$1"
    grep -Eo '[0-9]+(\.[0-9]+){1,2}' <<<"$text" | head -n1
}

check_tool_version() {
    local label="$1"
    local cmd="$2"
    local min="$3"

    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "[missing] ${label} (${cmd}) >= ${min}"
        failures=$((failures + 1))
        return
    fi

    local raw
    raw="$($cmd --version 2>/dev/null || $cmd -version 2>/dev/null || true)"
    local got
    got="$(extract_first_version "$raw")"

    if [[ -z "$got" ]]; then
        echo "[warn] ${label}: could not parse version from '$raw'"
        return
    fi

    if version_ge "$got" "$min"; then
        echo "[ok] ${label} ${got} (>= ${min})"
    else
        echo "[low] ${label} ${got} (< ${min})"
        failures=$((failures + 1))
    fi
}

check_zlib() {
    local min="1.2"
    local got=""

    if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists zlib; then
        got="$(pkg-config --modversion zlib 2>/dev/null || true)"
    fi

    if [[ -z "$got" ]] && command -v zlib-flate >/dev/null 2>&1; then
        got="$(zlib-flate -version 2>&1 | grep -Eo '[0-9]+(\.[0-9]+){1,2}' | head -n1 || true)"
    fi

    if [[ -z "$got" ]]; then
        echo "[warn] zlib: version not detected (expected >= ${min}); install dev package if LLVM build fails"
        return
    fi

    if version_ge "$got" "$min"; then
        echo "[ok] zlib ${got} (>= ${min})"
    else
        echo "[low] zlib ${got} (< ${min})"
        failures=$((failures + 1))
    fi
}

echo "== Izel System Dependency Check =="
check_tool_version "LLVM" "llvm-config" "17.0"
check_tool_version "lld" "ld.lld" "17.0"
check_tool_version "clang" "clang" "17.0"
check_tool_version "cmake" "cmake" "3.20"
check_zlib

if [[ "$REPORT_ONLY" == true ]]; then
    echo "Report-only mode: skipping failure exit status."
    exit 0
fi

if [[ "$failures" -gt 0 ]]; then
    echo "Dependency check failed with ${failures} issue(s)."
    exit 1
fi

echo "All required system dependencies look good."

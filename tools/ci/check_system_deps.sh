#!/usr/bin/env bash

set -euo pipefail

REPORT_ONLY=false
if [[ "${1:-}" == "--report-only" ]]; then
    REPORT_ONLY=true
fi

failures=0

detect_llvm_prefix() {
    local candidates=()

    if [[ -n "${LLVM_SYS_170_PREFIX:-}" ]]; then
        candidates+=("${LLVM_SYS_170_PREFIX}")
    fi

    if command -v llvm-config >/dev/null 2>&1; then
        local llvm_bin
        llvm_bin="$(command -v llvm-config)"
        candidates+=("$(cd "$(dirname "${llvm_bin}")/.." && pwd)")
    fi

    if command -v llvm-config-17 >/dev/null 2>&1; then
        local llvm17_bin
        llvm17_bin="$(command -v llvm-config-17)"
        candidates+=("$(cd "$(dirname "${llvm17_bin}")/.." && pwd)")
    fi

    if command -v brew >/dev/null 2>&1; then
        local brew_prefix
        brew_prefix="$(brew --prefix llvm@17 2>/dev/null || true)"
        if [[ -n "${brew_prefix}" ]]; then
            candidates+=("${brew_prefix}")
        fi
    fi

    candidates+=(
        "/opt/homebrew/opt/llvm@17"
        "/usr/local/opt/llvm@17"
        "/usr/lib/llvm-17"
        "/usr/lib64/llvm17"
    )

    local candidate
    for candidate in "${candidates[@]}"; do
        if [[ -n "${candidate}" && -x "${candidate}/bin/llvm-config" ]]; then
            echo "${candidate}"
            return 0
        fi
    done

    return 1
}

command_available() {
    local cmd="$1"
    if [[ "${cmd}" == */* ]]; then
        [[ -x "${cmd}" ]]
        return
    fi
    command -v "${cmd}" >/dev/null 2>&1
}

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

    if ! command_available "$cmd"; then
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

llvm_prefix="$(detect_llvm_prefix || true)"
llvm_config_cmd="llvm-config"
lld_cmd="ld.lld"

if [[ -n "${llvm_prefix}" ]]; then
    export LLVM_SYS_170_PREFIX="${llvm_prefix}"
    export PATH="${llvm_prefix}/bin:${PATH}"
    llvm_config_cmd="${llvm_prefix}/bin/llvm-config"
    if [[ -x "${llvm_prefix}/bin/ld.lld" ]]; then
        lld_cmd="${llvm_prefix}/bin/ld.lld"
    elif [[ -x "${llvm_prefix}/bin/lld" ]]; then
        lld_cmd="${llvm_prefix}/bin/lld"
    fi
fi

echo "== Izel System Dependency Check =="
check_tool_version "LLVM" "${llvm_config_cmd}" "17.0"
check_tool_version "lld" "${lld_cmd}" "17.0"
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

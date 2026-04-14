#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -eq 0 ]]; then
    echo "usage: $0 <command> [args...]" >&2
    exit 1
fi

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

if [[ -z "${LLVM_SYS_170_PREFIX:-}" ]]; then
    LLVM_SYS_170_PREFIX="$(detect_llvm_prefix || true)"
    if [[ -z "${LLVM_SYS_170_PREFIX}" ]]; then
        echo "Unable to locate LLVM 17. Set LLVM_SYS_170_PREFIX explicitly." >&2
        exit 1
    fi
fi

if [[ ! -x "${LLVM_SYS_170_PREFIX}/bin/llvm-config" ]]; then
    echo "LLVM_SYS_170_PREFIX='${LLVM_SYS_170_PREFIX}' does not contain bin/llvm-config" >&2
    exit 1
fi

case ":${PATH}:" in
    *":${LLVM_SYS_170_PREFIX}/bin:"*)
        ;;
    *)
        export PATH="${LLVM_SYS_170_PREFIX}/bin:${PATH}"
        ;;
esac

export LLVM_SYS_170_PREFIX
exec "$@"

#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECKSUM_FILE="${ROOT_DIR}/tools/bootstrap/bootstrap_sources.sha256"
STAGE_DIR="${ROOT_DIR}/target/selfhost-bootstrap"
DRY_RUN=true

if [[ "${1:-}" == "--execute" ]]; then
    DRY_RUN=false
fi

mkdir -p "${STAGE_DIR}"

echo "[bootstrap] verifying bootstrap source checksums"
(
    cd "${ROOT_DIR}"
    sha256sum -c "${CHECKSUM_FILE}"
)

echo "[bootstrap] building stage1 (Rust izelc)"
(
    cd "${ROOT_DIR}"
    cargo build -p izel_driver
)

STAGE1_BIN="${ROOT_DIR}/target/debug/izelc"
if [[ ! -x "${STAGE1_BIN}" ]]; then
    echo "error: stage1 binary not found at ${STAGE1_BIN}" >&2
    exit 1
fi

echo "[bootstrap] stage1 binary: ${STAGE1_BIN}"
echo "[bootstrap] stage2 target source: compiler/izelc.iz"

if [[ "${DRY_RUN}" == true ]]; then
    echo "[bootstrap] dry-run mode: command preview only"
    echo "  ${STAGE1_BIN} compiler/izelc.iz"
    echo "  sha256sum compiler/izelc.iz > target/selfhost-bootstrap/stage2-input.sha256"
    exit 0
fi

echo "[bootstrap] running stage1 compile against self-hosted driver source"
(
    cd "${ROOT_DIR}"
    "${STAGE1_BIN}" compiler/izelc.iz > "${STAGE_DIR}/stage1-compile.log"
)

echo "[bootstrap] recording stage2 input digest"
(
    cd "${ROOT_DIR}"
    sha256sum compiler/izelc.iz > "${STAGE_DIR}/stage2-input.sha256"
)

echo "[bootstrap] bootstrap run complete"

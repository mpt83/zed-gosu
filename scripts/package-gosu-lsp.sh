#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-$(rustc -vV | awk '/host:/ { print $2 }')}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT_DIR}/dist"
BIN_NAME="gosu-lsp"
ARCHIVE_NAME="gosu-lsp-${TARGET}.tar.gz"

cd "${ROOT_DIR}/gosu-lsp"
cargo build --release --target "${TARGET}"

mkdir -p "${OUT_DIR}"
tar -C "target/${TARGET}/release" -czf "${OUT_DIR}/${ARCHIVE_NAME}" "${BIN_NAME}"

echo "Created ${OUT_DIR}/${ARCHIVE_NAME}"

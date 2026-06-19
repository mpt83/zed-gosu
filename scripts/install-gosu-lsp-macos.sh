#!/usr/bin/env bash
set -euo pipefail

REPO="mpt83/zed-gosu"
INSTALL_DIR="${HOME}/.local/bin"

case "$(uname -m)" in
  arm64)
    TARGET="aarch64-apple-darwin"
    ;;
  x86_64)
    TARGET="x86_64-apple-darwin"
    ;;
  *)
    echo "unsupported macOS architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

ASSET="gosu-lsp-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

mkdir -p "${INSTALL_DIR}"

echo "Installing ${ASSET} to ${INSTALL_DIR}"
curl --fail --location "${URL}" | tar -xz -C "${INSTALL_DIR}"
chmod +x "${INSTALL_DIR}/gosu-lsp"

echo
echo "Installed: ${INSTALL_DIR}/gosu-lsp"
echo
echo "Launch Zed with:"
echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
echo "  zed ."
echo
echo "Use this Zed setting:"
echo '{'
echo '  "languages": {'
echo '    "Gosu": {'
echo '      "formatter": "language_server",'
echo '      "format_on_save": "on"'
echo '    }'
echo '  }'
echo '}'

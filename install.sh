#!/usr/bin/env bash
set -euo pipefail

REPO="yourusername/tempest"
INSTALL_DIR="/usr/local/bin"
BINARY="tempest"

echo "Installing Tempest..."

# Detect arch
ARCH=$(uname -m)
case "$ARCH" in
    x86_64) ARCH_TAG="x86_64-unknown-linux-gnu" ;;
    aarch64) ARCH_TAG="aarch64-unknown-linux-gnu" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Download latest release
LATEST_URL="https://github.com/${REPO}/releases/latest/download/tempest-${ARCH_TAG}"
TMP=$(mktemp)
echo "Downloading from $LATEST_URL..."
curl -fsSL "$LATEST_URL" -o "$TMP"
chmod +x "$TMP"

# Install
if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP" "${INSTALL_DIR}/${BINARY}"
else
    sudo mv "$TMP" "${INSTALL_DIR}/${BINARY}"
fi

echo "Installed to ${INSTALL_DIR}/${BINARY}"
echo ""
echo "Run 'tempest setup' to get started."

#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${INSTALL_PREFIX:-/usr/local}"
BIN_DIR="$PREFIX/bin"

echo "Installing rtg..."
echo "  Binary  → $BIN_DIR/rtg"
echo ""

mkdir -p "$BIN_DIR"

install -m755 "$SCRIPT_DIR/bin/rtg" "$BIN_DIR/rtg"

echo "Done. Run: rtg --help"

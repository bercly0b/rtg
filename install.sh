#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${INSTALL_PREFIX:-/usr/local}"
BIN_DIR="$PREFIX/bin"
LIB_DIR="$PREFIX/lib"

echo "Installing rtg..."
echo "  Binary  → $BIN_DIR/rtg"
echo "  Library → $LIB_DIR/"
echo ""

mkdir -p "$BIN_DIR" "$LIB_DIR"

install -m755 "$SCRIPT_DIR/bin/rtg" "$BIN_DIR/rtg"

for lib in "$SCRIPT_DIR"/lib/*; do
    install -m755 "$lib" "$LIB_DIR/$(basename "$lib")"
done

if [[ "$(uname)" == "Linux" ]] && command -v ldconfig &>/dev/null; then
    ldconfig
fi

echo "Done. Run: rtg --help"

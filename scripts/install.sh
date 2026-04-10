#!/usr/bin/env bash
set -euo pipefail

# ── Rho installer ────────────────────────────────────────────────────────────
#
# Builds rho in release mode and installs the binary to a location on PATH.
#
# Usage:
#   bash scripts/install.sh              # installs to ~/.local/bin/rho
#   bash scripts/install.sh /usr/local   # installs to /usr/local/bin/rho
#
# After installation, just run: rho

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Determine install prefix ────────────────────────────────────────────────
PREFIX="${1:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

echo "==> Building rho (release mode)..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

BINARY="$PROJECT_DIR/target/release/rho"
if [ ! -f "$BINARY" ]; then
  echo "ERROR: Build failed — binary not found at $BINARY"
  exit 1
fi

echo "==> Installing to $BIN_DIR/rho"
mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/rho"
chmod +x "$BIN_DIR/rho"

# ── Check PATH ──────────────────────────────────────────────────────────────
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
  echo ""
  echo "WARNING: $BIN_DIR is not in your PATH."
  echo ""
  echo "Add it by appending one of these to your shell profile:"
  echo ""
  echo "  # For zsh (~/.zshrc):"
  echo "  export PATH=\"$BIN_DIR:\$PATH\""
  echo ""
  echo "  # For bash (~/.bashrc):"
  echo "  export PATH=\"$BIN_DIR:\$PATH\""
  echo ""
  echo "Then restart your shell or run: source ~/.zshrc"
fi

echo ""
echo "==> rho installed successfully!"
echo "    Binary: $BIN_DIR/rho"
echo "    Version: $(\"$BIN_DIR/rho\" --version 2>/dev/null || echo 'unknown')"
echo ""
echo "Run it with: rho"

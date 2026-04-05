#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_URL="https://github.com/OpenHands/software-agent-sdk.git"
REPO_DIR="$SCRIPT_DIR/software-agent-sdk"
OUTPUT_DIR="$SCRIPT_DIR/dist"
CLONED=false

# ── Find Python >= 3.12 ──────────────────────────────────────────────
PYTHON=""
for candidate in python3.13 python3.12; do
  if command -v "$candidate" &>/dev/null; then
    PYTHON="$(command -v "$candidate")"
    break
  fi
done
if [ -z "$PYTHON" ]; then
  echo "ERROR: Python 3.12+ is required but not found on PATH"
  exit 1
fi
echo "==> Using Python: $PYTHON ($("$PYTHON" --version))"

# ── Cleanup handler — removes cloned repo on exit ────────────────────
cleanup() {
  if [ "$CLONED" = true ] && [ -d "$REPO_DIR" ]; then
    echo "==> Cleaning up: removing cloned repo"
    rm -rf "$REPO_DIR"
  fi
}
trap cleanup EXIT

# ── Clone the latest release ─────────────────────────────────────────
echo "==> Fetching latest release tag from $REPO_URL"
LATEST_TAG=$(git ls-remote --tags --sort=-v:refname "$REPO_URL" \
  | sed -n 's|.*refs/tags/||p' \
  | grep -v '{}' \
  | head -1)

if [ -z "$LATEST_TAG" ]; then
  echo "WARN: No release tags found, falling back to default branch"
  git clone --depth 1 "$REPO_URL" "$REPO_DIR"
else
  echo "==> Cloning tag: $LATEST_TAG"
  git clone --depth 1 --branch "$LATEST_TAG" "$REPO_URL" "$REPO_DIR"
fi
CLONED=true

# ── Preflight: verify spec file exists ────────────────────────────────
SPEC_FILE="$REPO_DIR/openhands-agent-server/openhands/agent_server/agent-server.spec"
if [ ! -f "$SPEC_FILE" ]; then
  echo "ERROR: PyInstaller spec file not found at $SPEC_FILE"
  exit 1
fi

echo "==> Building openhands-agent-server binary"

# ── Create a venv so we don't pollute the system Python ───────────────
VENV_DIR="$REPO_DIR/.build-venv"
echo "==> Creating build virtualenv"
"$PYTHON" -m venv "$VENV_DIR"
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"

# ── Install PyInstaller + project packages in editable mode ───────────
echo "==> Installing dependencies"
pip install --upgrade pip
pip install pyinstaller

for pkg in openhands-sdk openhands-tools openhands-workspace openhands-agent-server; do
  PKG_DIR="$REPO_DIR/$pkg"
  if [ -d "$PKG_DIR" ]; then
    echo "    Installing $pkg"
    pip install -e "$PKG_DIR"
  else
    echo "    WARN: $PKG_DIR not found, skipping"
  fi
done

# ── Build the binary ─────────────────────────────────────────────────
echo "==> Running PyInstaller (from repo root)"
pushd "$REPO_DIR" > /dev/null
pyinstaller --noconfirm --clean "$SPEC_FILE" --distpath "$OUTPUT_DIR"
popd > /dev/null

BINARY="$OUTPUT_DIR/openhands-agent-server"
if [ -f "$BINARY" ]; then
  echo "==> Build succeeded: $BINARY"
  ls -lh "$BINARY"
else
  echo "ERROR: Expected binary not found at $BINARY"
  exit 1
fi

# cleanup runs automatically via trap EXIT

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

# ── Patch spec: add missing hidden imports & data files ──────────────
echo "==> Patching spec file: adding missing modules"
"$PYTHON" -c "
import pathlib, sys
p = pathlib.Path(sys.argv[1])
txt = p.read_text()

# 1. Add hidden imports for jinja2 and binaryornot
old_hi = '*collect_submodules(\"fakeredis\"),'
new_hi = old_hi + '''
        *collect_submodules(\"jinja2\"),
        \"jinja2.debug\",
        *collect_submodules(\"binaryornot\"),'''
if old_hi in txt:
    txt = txt.replace(old_hi, new_hi, 1)

# 2. Add binaryornot data files (binaryornot.data needed by importlib.resources)
old_data = '*collect_data_files(\"fakeredis\"),'
new_data = old_data + '''
        *collect_data_files(\"binaryornot\"),'''
if old_data in txt:
    txt = txt.replace(old_data, new_data, 1)

# 3. Switch from one-file to one-dir (COLLECT step) for fast startup
#    Replace the EXE that bundles everything into a single binary with
#    an EXE + COLLECT that outputs a directory.
old_exe = '''exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name=\"openhands-agent-server\",'''
new_exe = '''exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name=\"openhands-agent-server\",'''
if old_exe in txt:
    txt = txt.replace(old_exe, new_exe, 1)

# Add COLLECT after the EXE closing paren
old_exe_end = '''    icon=None,
)'''
new_exe_end = '''    icon=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=True,
    upx=True,
    name=\"openhands-agent-server\",
)'''
if old_exe_end in txt:
    txt = txt.replace(old_exe_end, new_exe_end, 1)

p.write_text(txt)
print('Patched successfully')
" "$SPEC_FILE"

# ── Build the binary ─────────────────────────────────────────────────
echo "==> Running PyInstaller (from repo root)"
pushd "$REPO_DIR" > /dev/null
pyinstaller --noconfirm --clean "$SPEC_FILE" --distpath "$OUTPUT_DIR"
popd > /dev/null

BINARY="$OUTPUT_DIR/openhands-agent-server/openhands-agent-server"
if [ -f "$BINARY" ]; then
  echo "==> Build succeeded (onedir): $OUTPUT_DIR/openhands-agent-server/"
  ls -lh "$BINARY"
  du -sh "$OUTPUT_DIR/openhands-agent-server/"
else
  echo "ERROR: Expected binary not found at $BINARY"
  exit 1
fi

# cleanup runs automatically via trap EXIT

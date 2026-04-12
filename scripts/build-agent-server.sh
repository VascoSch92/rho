#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_URL="https://github.com/OpenHands/software-agent-sdk.git"
REPO_DIR="$SCRIPT_DIR/software-agent-sdk"
OUTPUT_DIR="$SCRIPT_DIR/dist"
CLONED=false

# ── Logging helpers ──────────────────────────────────────────────────
# Colors (disabled if not a TTY)
if [ -t 1 ]; then
  C_RESET=$'\033[0m'
  C_BLUE=$'\033[1;34m'
  C_GREEN=$'\033[1;32m'
  C_YELLOW=$'\033[1;33m'
  C_RED=$'\033[1;31m'
  C_DIM=$'\033[2m'
else
  C_RESET='' C_BLUE='' C_GREEN='' C_YELLOW='' C_RED='' C_DIM=''
fi

step() { echo "${C_BLUE}▸${C_RESET} $*"; }
ok()   { echo "${C_GREEN}✓${C_RESET} $*"; }
warn() { echo "${C_YELLOW}!${C_RESET} $*"; }
err()  { echo "${C_RED}✗${C_RESET} $*" >&2; }
info() { echo "  ${C_DIM}$*${C_RESET}"; }

# ── Preflight: Python ≥ 3.12 ─────────────────────────────────────────
step "Checking Python (requires 3.12+)"
PYTHON=""
for candidate in python3.13 python3.12; do
  if command -v "$candidate" &>/dev/null; then
    PYTHON="$(command -v "$candidate")"
    break
  fi
done

if [ -z "$PYTHON" ]; then
  err "Python 3.12+ is required but not found on PATH."
  err "Install via your package manager, e.g.:"
  err "  macOS:   brew install python@3.13"
  err "  Debian:  sudo apt install python3.13"
  exit 1
fi

PY_VERSION="$("$PYTHON" --version 2>&1 | awk '{print $2}')"
ok "Using $PYTHON ($PY_VERSION)"

# ── Preflight: git ───────────────────────────────────────────────────
if ! command -v git &>/dev/null; then
  err "git is required but not found on PATH."
  exit 1
fi

# ── Cleanup handler — removes cloned repo on exit ────────────────────
cleanup() {
  if [ "$CLONED" = true ] && [ -d "$REPO_DIR" ]; then
    info "Cleaning up cloned repo"
    rm -rf "$REPO_DIR"
  fi
}
trap cleanup EXIT

# ── Read pinned version from config.toml ─────────────────────────────
step "Resolving agent server version"
CONFIG_FILE="$SCRIPT_DIR/../config.toml"
PINNED_VERSION=""
if [ -f "$CONFIG_FILE" ]; then
  PINNED_VERSION=$(grep -A1 '^\[agent_server\]' "$CONFIG_FILE" \
    | grep '^version' \
    | sed 's/.*= *"\(.*\)"/\1/')
fi

if [ -n "$PINNED_VERSION" ]; then
  TAG="v${PINNED_VERSION}"
  ok "Pinned version from config.toml: $PINNED_VERSION (tag: $TAG)"
else
  warn "No pinned version in config.toml, fetching latest release"
  TAG=$(git ls-remote --tags --sort=-v:refname "$REPO_URL" \
    | sed -n 's|.*refs/tags/||p' \
    | grep -v '{}' \
    | head -1)
  if [ -z "$TAG" ]; then
    warn "No release tags found, using default branch"
    TAG=""
  else
    ok "Latest release tag: $TAG"
  fi
fi

# ── Clone the SDK ────────────────────────────────────────────────────
step "Cloning OpenHands SDK"
if [ -n "$TAG" ]; then
  git clone --quiet --depth 1 --branch "$TAG" "$REPO_URL" "$REPO_DIR"
else
  git clone --quiet --depth 1 "$REPO_URL" "$REPO_DIR"
fi
CLONED=true
ok "Cloned to $REPO_DIR"

# ── Preflight: verify spec file exists ────────────────────────────────
SPEC_FILE="$REPO_DIR/openhands-agent-server/openhands/agent_server/agent-server.spec"
if [ ! -f "$SPEC_FILE" ]; then
  err "PyInstaller spec file not found at:"
  err "  $SPEC_FILE"
  err "The SDK layout may have changed — please check the upstream repo."
  exit 1
fi

# ── Create a venv so we don't pollute the system Python ───────────────
step "Creating build virtualenv"
VENV_DIR="$REPO_DIR/.build-venv"
"$PYTHON" -m venv "$VENV_DIR"
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"
ok "Virtualenv ready: $VENV_DIR"

# ── Install PyInstaller + project packages ───────────────────────────
step "Installing build dependencies"
pip install --quiet --upgrade pip
pip install --quiet pyinstaller
ok "Installed pip + pyinstaller"

step "Installing OpenHands SDK packages"
for pkg in openhands-sdk openhands-tools openhands-workspace openhands-agent-server; do
  PKG_DIR="$REPO_DIR/$pkg"
  if [ -d "$PKG_DIR" ]; then
    info "$pkg"
    pip install --quiet -e "$PKG_DIR"
  else
    warn "$pkg not found at $PKG_DIR, skipping"
  fi
done
ok "All packages installed"

# ── Patch spec: add missing hidden imports & data files ──────────────
step "Patching PyInstaller spec"
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

# 2b. Exclude unused stdlib / 3rd-party modules to speed up startup & shrink size
import re
EXCLUDES = [
    # GUI toolkits — agent server is headless
    'tkinter', '_tkinter', 'turtle', 'turtledemo', 'idlelib',
    # Python's own test suite
    'test', 'tests', 'unittest', 'pydoc', 'pydoc_data',
    # Dev/build tooling
    'distutils', 'lib2to3', 'pip', 'setuptools', 'wheel',
    # Docs / notebooks (not needed at runtime)
    'IPython', 'jupyter', 'notebook', 'sphinx', 'docutils',
    # Scientific stack we don't use here
    'matplotlib', 'scipy', 'sklearn', 'pandas.tests', 'numpy.tests',
]
old_excl = 'excludes=[]'
new_excl = 'excludes=' + repr(EXCLUDES)
if old_excl in txt:
    txt = txt.replace(old_excl, new_excl, 1)
else:
    # Fallback: inject into Analysis(...) if excludes=[] isn't literal
    txt = re.sub(
        r'(Analysis\([^)]*?)hiddenimports=',
        lambda m: m.group(1) + 'excludes=' + repr(EXCLUDES) + ',\\n    hiddenimports=',
        txt, count=1, flags=re.DOTALL,
    )

# 3. Switch from one-file to one-dir (COLLECT step) for fast startup
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

old_exe_end = '''    icon=None,
)'''
new_exe_end = '''    icon=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=True,
    upx=False,
    name=\"openhands-agent-server\",
)'''
if old_exe_end in txt:
    txt = txt.replace(old_exe_end, new_exe_end, 1)

p.write_text(txt)
" "$SPEC_FILE"
ok "Spec patched"

# ── Build the binary ─────────────────────────────────────────────────
step "Running PyInstaller (this takes a few minutes)"
pushd "$REPO_DIR" > /dev/null
pyinstaller --noconfirm --clean --log-level=WARN "$SPEC_FILE" --distpath "$OUTPUT_DIR" > /dev/null
popd > /dev/null

BINARY="$OUTPUT_DIR/openhands-agent-server/openhands-agent-server"
if [ ! -f "$BINARY" ]; then
  err "Build completed but binary not found at:"
  err "  $BINARY"
  exit 1
fi

SIZE=$(du -sh "$OUTPUT_DIR/openhands-agent-server/" | awk '{print $1}')
ok "Build succeeded"
info "Location: $OUTPUT_DIR/openhands-agent-server/"
info "Size:     $SIZE"

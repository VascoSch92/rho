<pre>
 ▄▄▄▄▄▄                        ┃
█▀██▀▀▀█▄  █▄                  ┃  <b>Rho</b> v0.1.0
  ██▄▄▄█▀  ██                  ┃  Terminal UI for OpenHands
  ██▀▀█▄   ████▄ ▄███▄         ┃  Built with Rust + Ratatui
▄ ██  ██   ██ ██ ██ ██         ┃  License: MIT
▀██▀  ▀██▀▄██ ██▄▀███▀         ┃
</pre>

# Rho — OpenHands Agent Server TUI (Rust/Ratatui)

> **WARNING: This project is highly experimental and under active development.**
> APIs, commands, and behavior may change without notice. Use at your own risk.
> Bug reports and feedback are welcome on the [issue tracker](https://github.com/VascoSch92/rho/issues).

A terminal UI for OpenHands, built with [Ratatui](https://ratatui.rs/), that connects to the [OpenHands Agent Server](https://docs.openhands.dev/sdk/guides/agent-server/overview).

## Installation

### Prerequisites

- **Rust toolchain** (edition 2021) — install from [rustup.rs](https://rustup.rs)
- **Python 3.12+** — required to build the Agent Server
- An **LLM API key** (configured after install via `/settings` or `--override-with-envs`)

### Install

```bash
git clone https://github.com/VascoSch92/rho.git
cd rho
make install
```

This will:
1. Build the OpenHands Agent Server from source (pinned version from `config.toml`)
2. Compile Rho in release mode
3. Install both to `~/.local/bin/`

To install to a different location:

```bash
make install PREFIX=/usr/local
```

Make sure the install directory is in your `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Uninstall

```bash
make uninstall
```

### Configure LLM settings

On first run, use `/settings` inside the TUI to set your API key, model, and provider. Settings are persisted to `~/.rho/config.toml`.

Alternatively, use environment variables with the `--override-with-envs` flag:

```bash
export LLM_API_KEY="..."
export LLM_MODEL="openai/gpt-4o"      # optional (default: anthropic/claude-sonnet-4-5-20250929)
rho --override-with-envs
```

Environment variables are persisted to `~/.rho/config.toml` when `--override-with-envs` is used, so you only need the flag once.

### Run

```bash
rho
```

Rho automatically launches the bundled Agent Server. If you pass `--debug`, logs are written to `~/.rho/rho.log`.

## Features

- **Real-time event streaming** via WebSocket
- **Action confirmation** policies: always / only-risky / never
- **Message queue** — send multiple messages while the agent is busy; they execute in order
- **Slash commands** (`/help`, `/new`, `/settings`, `/theme`, `/resume`, `/rename`, `/btw`, ...)
- **Local shell shortcuts** — run a command by typing `!<cmd>` (e.g. `!ls`)
- **One-shot questions** — ask the agent without affecting the conversation via `/btw <question>`
- **Collapsible actions** + `Ctrl+E` expand/collapse all
- **Status bar** with timer, model, context usage, cost, and token counts
- **Themes** — 8 built-in themes (rho, dracula, catppuccin, tokyonight, solarized, gruvbox, github, custom), persisted across sessions
- **Markdown rendering** with headings, lists, code blocks, tables, and inline code
- **Web mode** — access the TUI from a browser via `rho web`
- **Headless mode** — run tasks without the TUI via `rho headless`, with JSON output for scripting

## Usage

### Connect to an existing Agent Server

```bash
rho --server http://192.168.1.100:8000
```

If an Agent Server is already running on the default port, Rho will detect and reuse it automatically (version is verified against the pinned version in `config.toml`).

### Web mode (browser access)

```bash
rho web                              # default: http://127.0.0.1:12000
rho web --host 0.0.0.0 --port 8080   # custom host/port
```

Open the printed URL in your browser. Each tab gets its own independent session.

### Headless mode (scripting / CI)

```bash
# Inline task
rho headless --task "Fix the bug in main.py"

# Task from file
rho headless --file task.txt

# JSON Lines output for machine consumption
rho headless --json --task "Write tests" | jq '.type'

# With timeout and auto-approve (for CI/CD)
rho headless --task "Run linting" --timeout 300 --auto-approve
```

Exit codes: `0` success, `1` task error, `2` timeout, `3` connection error.

### Common options

```bash
# Override LLM settings from environment variables (persisted to ~/.rho/config.toml)
rho --override-with-envs

# Server URL
rho --server http://192.168.1.100:8000

# Agent Server session auth (header: X-Session-API-Key)
rho --session-api-key your-session-key

# Workspace directory sent to the agent as the working directory
rho --workspace /path/to/repo

# Theme (also settable via /theme or ~/.rho/config.toml)
rho --theme dracula

# Skip exit confirmation
rho --exit-without-confirmation

# Enable debug logging (writes ~/.rho/rho.log)
rho --debug
```

## Configuration

Rho stores user configuration in `~/.rho/config.toml`. This file is created automatically when you change settings.

**Priority order** (highest to lowest):
1. CLI flags (`--theme`, `--override-with-envs`)
2. `~/.rho/config.toml` (persisted settings)
3. Embedded defaults (`config.toml` at build time)

Settings that can be configured:
- **LLM** — provider, model, API key, base URL (via `/settings` or `--override-with-envs`)
- **Theme** — active theme name (via `/theme` or `--theme`)

The full set of customizations (themes, spinners, keybindings, scroll speed, selector indicator) can be edited in the embedded `config.toml` or overridden in `~/.rho/config.toml`.

## Themes

8 built-in themes: **rho**, **dracula**, **catppuccin**, **tokyonight**, **solarized**, **gruvbox**, **github**.

Change theme with `/theme` (opens picker) or `/theme <name>`. The selection is persisted.

To add a custom theme, add a section to `~/.rho/config.toml`:

```toml
[theme.themes.my_theme]
primary    = "#e06c75"
accent     = "#61afef"
foreground = "#abb2bf"
background = "reset"
muted      = "#5c6370"
border     = "#3e4452"
error      = "#be5046"
success    = "#98c379"
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Alt+Enter` / `Shift+Enter` | Insert newline in input |
| `Esc` | Pause agent (when running) / close modals |
| `Ctrl+Q` / `Ctrl+C` | Quit (with confirmation unless `--exit-without-confirmation`) |
| `Ctrl+E` | Expand/collapse all actions |
| `Ctrl+T` | Toggle task list panel |
| `Up/Down` | Scroll messages |
| `PgUp/PgDn` | Scroll faster |

### Confirmation mode

When actions require confirmation:

| Key | Action |
|-----|--------|
| `Left/Right` | Select confirm option |
| `Enter` | Apply selected option |
| `Y` | Accept |
| `N` | Reject |
| `A` | Always accept (auto-approve future actions) |
| `Esc` | Defer (pause) |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show help modal |
| `/new` | Start a new conversation (clears UI state) |
| `/resume` | Resume a previous conversation |
| `/rename <name>` | Rename the current conversation |
| `/btw <question>` | Ask the agent a one-shot question (not part of the conversation) |
| `/pause` | Pause the agent |
| `/usage` | Show token usage details |
| `/settings` | Show/edit current settings |
| `/theme [name]` | Pick or set theme |
| `/confirm [always\|risky\|never]` | Show/change confirmation policy |
| `/exit` / `/quit` | Exit the application |

## Local shell commands

Prefix any input with `!` to run it locally and show output in the message log:

- `!pwd`
- `!ls -la`

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     Ratatui TUI (Rust)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Input Field │  │ Message Log │  │ Confirmation UI     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                           │                                  │
│           ┌───────────────┴───────────────┐                  │
│           │    Async Event Handler        │                  │
│           │  (tokio + WebSocket client)   │                  │
│           └───────────────┬───────────────┘                  │
└───────────────────────────┼──────────────────────────────────┘
                            │ HTTP/WebSocket
                            ▼
┌──────────────────────────────────────────────────────────────┐
│            Agent Server (OpenHands SDK)                       │
│  ┌─────────────────────────────────────────────────────┐     │
│  │  Conversation, Tools, Events                        │     │
│  └─────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘

Web mode (rho web):

┌──────────────────────────────────────────────────────────────┐
│                  Browser (xterm.js)                          │
│  ┌─────────────────────────────────────────────────────┐     │
│  │               WebSocket client                      │     │
│  └──────────────────────┬──────────────────────────────┘     │
└─────────────────────────┼────────────────────────────────────┘
                          │ WebSocket
┌─────────────────────────┼────────────────────────────────────┐
│                         ▼                                    │
│        Rho Web Server (axum + portable-pty)                  │
│  ┌─────────────────────────────────────────────────────┐     │
│  │  PTY  <->  spawns rho TUI as subprocess             │     │
│  └─────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘

Headless mode (rho headless):

┌──────────────────────────────────────────────────────────────┐
│                   rho headless                               │
│  ┌─────────────┐    ┌──────────────────────────────────┐     │
│  │  CLI Args   │--->│        HeadlessRunner            │     │
│  │  (cli.rs)   │    │  - Start conversation            │     │
│  └─────────────┘    │  - Stream events via WebSocket   │     │
│                     │  - Print to stdout (text / JSON)  │     │
│                     └──────────────┬───────────────────┘     │
│                                    │ HTTP/WebSocket          │
└────────────────────────────────────┼─────────────────────────┘
                                     ▼
                       ┌─────────────────────────┐
                       │    Agent Server          │
                       │  (existing, unchanged)   │
                       └─────────────────────────┘
```

## Data & Configuration

Rho uses `~/.rho/` for all persistent data:

| Path | Description |
|------|-------------|
| `~/.rho/config.toml` | User settings (LLM, theme) |
| `~/.rho/conversations/` | Agent Server conversation storage |
| `~/.rho/agent_settings.json` | LLM settings (shared with openhands-cli) |
| `~/.rho/rho.log` | Debug log (when `--debug` is enabled) |

## Project Structure

```
dist/
└── openhands-agent-server/  # Agent Server binary (onedir PyInstaller)
scripts/
└── build-agent-server.sh    # Builds the Agent Server from source
src/
├── main.rs              # Entry point, terminal setup, embedded server launcher
├── cli.rs               # CLI args and subcommands (clap)
├── client/              # HTTP + WebSocket client for Agent Server
├── handlers/            # Key handling, slash commands, settings edits, command execution
├── state/               # App state + message models
├── ui/                  # Ratatui UI (widgets, modals, markdown rendering)
├── config/              # Configuration loading, themes, keybindings
├── events/              # Event types (mirroring OpenHands SDK events)
├── headless/            # Headless runner (no TUI, stdout output)
└── web/                 # Web server (axum + PTY + xterm.js frontend)
web/
└── index.html           # xterm.js frontend (embedded at compile time)
config.toml              # Default configuration (embedded in binary)
```

## Development

```bash
cargo test
cargo clippy
cargo fmt

# Hot reloading (requires cargo-watch)
cargo watch -x run
```

## License

MIT (see `Cargo.toml`).

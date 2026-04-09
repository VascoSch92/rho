# Rho — OpenHands Agent Server TUI (Rust/Ratatui)

A terminal UI for OpenHands, built with [Ratatui](https://ratatui.rs/), that connects to the [OpenHands Agent Server](https://docs.openhands.dev/sdk/guides/agent-server/overview).

Rho can **automatically launch a local Agent Server** if the `dist/openhands-agent-server/` directory is present. Otherwise, it connects to whatever `--server` you provide.

## Quickstart

### 1) Configure LLM settings

On first run, use `/settings` inside the TUI to set your API key, model, and provider. Settings are persisted to `.rho/config.toml`.

Alternatively, use environment variables with the `--override-with-envs` flag:

```bash
export LLM_API_KEY="..."
export LLM_MODEL="openai/gpt-4o"      # optional (default: anthropic/claude-sonnet-4-5-20250929)
cargo run -- --override-with-envs
```

Environment variables are persisted to `.rho/config.toml` when `--override-with-envs` is used, so you only need the flag once.

### 2) Run

```bash
cargo run
```

If `dist/openhands-agent-server/` exists, Rho will launch it automatically. If you pass `--debug`, logs are written to `.rho/rho.log`.

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
│            Agent Server (dist/openhands-agent-server/)       │
│  ┌─────────────────────────────────────────────────────┐     │
│  │  OpenHands SDK: Conversation, Tools, Events         │     │
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

## Features

- **Real-time event streaming** via WebSocket
- **Action confirmation** policies: always / only-risky / never
- **Message queue** — send multiple messages while the agent is busy; they execute in order
- **Slash commands** (`/help`, `/new`, `/settings`, `/theme`, `/resume`, `/rename`, ...)
- **Local shell shortcuts** — run a command by typing `!<cmd>` (e.g. `!ls`)
- **Collapsible actions** + `Ctrl+E` expand/collapse all
- **Status bar** with timer, model, context usage, cost, and token counts
- **Themes** — 8 built-in themes (rho, dracula, catppuccin, tokyonight, solarized, gruvbox, github, custom), persisted across sessions
- **Markdown rendering** with headings, lists, code blocks, tables, and inline code
- **Web mode** — access the TUI from a browser via `rho web`
- **Headless mode** — run tasks without the TUI via `rho headless`, with JSON output for scripting

## Prerequisites

- **Rust toolchain** (edition 2021)
- **Agent Server** — either the bundled binary at `dist/openhands-agent-server/` or an external server
- An **LLM API key** (configured via `/settings` or `--override-with-envs`)

## Building

### Rho (TUI)

```bash
cargo build            # development
cargo build --release  # optimized
```

### Agent Server (optional, if not using an external server)

```bash
bash scripts/build-agent-server.sh
```

This clones the latest [OpenHands SDK](https://github.com/OpenHands/software-agent-sdk) release, builds a PyInstaller binary, and places it in `scripts/dist/openhands-agent-server/`. Copy it to `dist/`:

```bash
cp -R scripts/dist/openhands-agent-server dist/openhands-agent-server
```

## Usage

### Embedded server (default)

If `dist/openhands-agent-server/` exists, Rho launches it automatically:

```bash
cargo run
```

### Connect to an existing Agent Server

```bash
cargo run -- --server http://192.168.1.100:8000
```

### Web mode (browser access)

```bash
cargo run -- web                              # default: http://127.0.0.1:12000
cargo run -- web --host 0.0.0.0 --port 8080   # custom host/port
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
# Override LLM settings from environment variables (persisted to .rho/config.toml)
cargo run -- --override-with-envs

# Server URL
cargo run -- --server http://192.168.1.100:8000

# Agent Server session auth (header: X-Session-API-Key)
cargo run -- --session-api-key your-session-key

# Workspace directory sent to the agent as the working directory
cargo run -- --workspace /path/to/repo

# Theme (also settable via /theme or .rho/config.toml)
cargo run -- --theme dracula

# Skip exit confirmation
cargo run -- --exit-without-confirmation

# Enable debug logging (writes .rho/rho.log)
cargo run -- --debug
```

## Configuration

Rho stores user configuration in `.rho/config.toml` (next to the project root). This file is created automatically when you change settings.

**Priority order** (highest to lowest):
1. CLI flags (`--theme`, `--override-with-envs`)
2. `.rho/config.toml` (persisted settings)
3. Embedded defaults (`config.toml` at build time)

Settings that can be configured:
- **LLM** — provider, model, API key, base URL (via `/settings` or `--override-with-envs`)
- **Theme** — active theme name (via `/theme` or `--theme`)

The full set of customizations (themes, spinners, keybindings, scroll speed, selector indicator) can be edited in the embedded `config.toml` or overridden in `.rho/config.toml`.

## Themes

8 built-in themes: **rho**, **dracula**, **catppuccin**, **tokyonight**, **solarized**, **gruvbox**, **github**.

Change theme with `/theme` (opens picker) or `/theme <name>`. The selection is persisted.

To add a custom theme, add a section to `.rho/config.toml`:

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

## Data & Configuration

Rho uses a `.rho/` directory at the project root:

| Path | Description |
|------|-------------|
| `.rho/config.toml` | User settings (LLM, theme) |
| `.rho/conversations/` | Agent Server conversation storage |
| `.rho/bash_events/` | Agent Server bash event logs |
| `.rho/rho.log` | Debug log (when `--debug` is enabled) |

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

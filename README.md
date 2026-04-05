# Rho — OpenHands Agent Server TUI (Rust/Ratatui)

A terminal UI for OpenHands, built with [Ratatui](https://ratatui.rs/), that connects to the [OpenHands Agent Server](https://docs.openhands.dev/sdk/guides/agent-server/overview).

Rho can **optionally launch a local Agent Server automatically** if you’ve set up the repo’s Python `.venv` (see Quickstart). Otherwise, it will just connect to whatever `--server` you provide.

## Quickstart (embedded Agent Server)

### 1) Install the Python Agent Server into `.venv`

This repo uses [`uv`](https://docs.astral.sh/uv/) and `pyproject.toml` to install `openhands-agent-server` locally.

```bash
make build
```

### 2) Provide an LLM API key (required)

Rho must send an LLM configuration to the Agent Server when starting conversations.

```bash
export LLM_API_KEY="..."              # required
export LLM_MODEL="openai/gpt-4o"      # optional (default is an Anthropic model)
```

### 3) Run

```bash
cargo run
```

If you pass `--debug`, logs are written to `.rho/rho.log`.

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
│              Agent Server (Python)                           │
│  ┌─────────────────────────────────────────────────────┐     │
│  │  OpenHands SDK: Conversation, Tools, Events         │     │
│  └─────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘

Web mode (`rho web`):

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
│  │  PTY  ←→  spawns `rho` TUI as subprocess           │     │
│  └─────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘

Headless mode (`rho headless`):

┌──────────────────────────────────────────────────────────────┐
│                   rho headless                               │
│  ┌─────────────┐    ┌──────────────────────────────────┐     │
│  │  CLI Args   │───►│        HeadlessRunner            │     │
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
- **Slash commands** (`/help`, `/new`, `/settings`, `/theme`, …)
- **Local shell shortcuts**: run a command by typing `!<cmd>` (e.g. `!ls`)
- **Collapsible actions** + `Ctrl+E` expand/collapse all
- **Status indicators** for connection, execution status, and token usage
- **Themes** via `--theme` or `/theme`
- **Web mode** — access the TUI from a browser via `rho web`
- **Headless mode** — run tasks without the TUI via `rho headless`, with JSON output for scripting

## Prerequisites

- **Rust toolchain** (edition 2021)
- For the embedded server path: **Python 3.12** + **uv**
- An **LLM API key** (set `LLM_API_KEY`)

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
```

## Usage

### Embedded server (default if `.venv` exists)

If `.venv/bin/python` exists (created by `make build`), Rho will try to start:

- module: `openhands.agent_server`
- host/port: derived from `--server` (defaults to `http://127.0.0.1:8000`)
- data dir: `.rho/` (conversations, bash events, logs)

Run:

```bash
cargo run -- --llm-api-key "$LLM_API_KEY"
```

### Connect to an existing Agent Server

Start an Agent Server yourself (examples):

```bash
python -m openhands.agent_server --port 8000
# or
# docker run -p 8000:8000 ghcr.io/openhands/agent-server:latest
```

Then point Rho at it:

```bash
cargo run -- \
  --server http://127.0.0.1:8000 \
  --llm-api-key "$LLM_API_KEY"
```

Note: if you want to *prevent* Rho from attempting to launch an embedded server,
run without the repo’s `.venv/` present.

### Web mode (browser access)

Rho can serve the full TUI in a browser using xterm.js:

```bash
# Start the web server (default: http://127.0.0.1:12000)
cargo run -- web

# Custom host/port
cargo run -- web --host 0.0.0.0 --port 8080
```

Open the printed URL in your browser. Each browser tab gets its own independent session. Environment variables (`LLM_API_KEY`, etc.) are forwarded to each session automatically.

### Headless mode (scripting / CI)

Run a task without the TUI — output goes to stdout/stderr:

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
# Server URL
cargo run -- --server http://192.168.1.100:8000

# Agent Server session auth (header: X-Session-API-Key)
cargo run -- --session-api-key your-session-key

# Model selection ("provider/model" or just "model")
cargo run -- --model openai/gpt-4o

# Custom base URL (OpenAI-compatible endpoints)
cargo run -- --llm-base-url http://localhost:8080/v1

# Workspace directory sent to the agent as the working directory
cargo run -- --workspace /path/to/repo

# Auto-approve all actions (no confirmation prompts)
cargo run -- --always-approve

# Skip exit confirmation
cargo run -- --exit-without-confirmation

# Enable debug logging (writes .rho/rho.log)
cargo run -- --debug
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Alt+Enter` / `Shift+Enter` | Insert newline in input |
| `Esc` | Pause agent (when running) / close modals |
| `Ctrl+Q` / `Ctrl+C` | Quit (with confirmation unless `--exit-without-confirmation`) |
| `Ctrl+E` | Expand/collapse all actions |
| `↑/↓` | Scroll messages |
| `PgUp/PgDn` | Scroll faster |

### Confirmation mode

When actions require confirmation:

| Key | Action |
|-----|--------|
| `←/→` | Select confirm option |
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

## Data & logs

Rho creates a `.rho/` directory at the repository root and uses it for:

- `.rho/conversations/` (Agent Server conversation storage)
- `.rho/bash_events/` (Agent Server bash event logs)
- `.rho/rho.log` (when `--debug` is enabled)

## Project Structure

```
src/
├── main.rs              # Entry point, terminal setup, embedded server launcher
├── cli.rs               # CLI args and subcommands (clap)
├── client/              # HTTP + WebSocket client for Agent Server
├── handlers/            # Key handling, slash commands, settings edits, command execution
├── state/               # App state + message models
├── ui/                  # Ratatui UI (widgets, modals, markdown rendering)
├── config/              # Theme definitions
├── headless/            # Headless runner (no TUI, stdout output)
└── web/                 # Web server (axum + PTY + xterm.js frontend)
web/
└── index.html           # xterm.js frontend (embedded at compile time)
```

## Development

```bash
# Install/update the embedded Agent Server dependencies
make build

cargo test
cargo clippy
cargo fmt

# Hot reloading (requires cargo-watch)
cargo watch -x run
```

## License

MIT (see `Cargo.toml`).

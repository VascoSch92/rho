# OpenHands TUI (Rust/Ratatui)

A terminal user interface for OpenHands, built with [Ratatui](https://ratatui.rs/) and designed to connect to the [OpenHands Agent Server](https://docs.openhands.dev/sdk/guides/agent-server/overview).

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Ratatui TUI (Rust)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Input Field │  │ Message Log │  │ Confirmation Panel  │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                           │                                  │
│           ┌───────────────┴───────────────┐                  │
│           │    Async Event Handler        │                  │
│           │  (tokio + WebSocket client)   │                  │
│           └───────────────┬───────────────┘                  │
└───────────────────────────┼─────────────────────────────────┘
                            │ HTTP/WebSocket
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Agent Server (Python)                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  OpenHands SDK: Conversation, Tools, Events         │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Features

- **Real-time event streaming** via WebSocket
- **Action confirmation** with configurable policies (Always/Never/Risky)
- **Slash commands** (`/help`, `/new`, `/pause`, `/confirm`, `/exit`)
- **Collapsible messages** for actions and observations
- **Status indicators** for connection, execution state, and metrics
- **Notification popups** for important events

## Prerequisites

1. **Rust toolchain** (1.70+)
2. **OpenHands Agent Server** running (see below)

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
```

## Usage

### 1. Start the Agent Server

First, start the OpenHands Agent Server (Python):

```bash
# Using the OpenHands SDK
python -m openhands.agent_server --port 8000
```

Or using Docker:

```bash
docker run -p 8000:8000 ghcr.io/openhands/agent-server:latest
```

### 2. Run the TUI

```bash
# Connect to default server (localhost:8000)
cargo run

# Connect to a specific server
cargo run -- --server http://192.168.1.100:8000

# With API key
cargo run -- --api-key your-api-key

# Auto-approve all actions (no confirmation prompts)
cargo run -- --always-approve

# Skip exit confirmation
cargo run -- --exit-without-confirmation

# Enable debug logging
cargo run -- --debug
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Esc` | Pause agent / Cancel |
| `Ctrl+Q` / `Ctrl+C` | Quit (with confirmation) |
| `Up/Down` | Scroll messages |
| `PageUp/PageDown` | Scroll faster |

### Confirmation Mode

When actions require confirmation:

| Key | Action |
|-----|--------|
| `Y` | Approve action |
| `N` | Reject action |
| `A` | Approve all (change policy to Never Confirm) |
| `D` | Defer (pause agent) |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/new` | Start a new conversation |
| `/pause` | Pause the agent |
| `/confirm <policy>` | Set confirmation policy (always/never/risky) |
| `/exit` | Exit the application |

## Project Structure

```
src/
├── main.rs           # Application entry point and event loop
├── client/
│   ├── mod.rs        # Client module
│   ├── api.rs        # HTTP API client
│   └── websocket.rs  # WebSocket event streaming
├── events/
│   └── mod.rs        # Event types (mirrors SDK events)
├── state/
│   └── mod.rs        # Application state management
└── ui/
    ├── mod.rs        # UI module
    ├── layout.rs     # Main layout
    ├── input.rs      # Input field widget
    ├── messages.rs   # Message list widget
    ├── status.rs     # Status bar widgets
    └── confirmation.rs # Confirmation panel
```

## Comparison with Python TUI

| Aspect | Python (Textual) | Rust (Ratatui) |
|--------|------------------|----------------|
| **Runtime** | Requires Python | Single binary |
| **Performance** | Good | Excellent |
| **Memory** | ~50-100MB | ~5-10MB |
| **Startup** | ~1-2s | ~10ms |
| **Backend** | Direct SDK calls | Agent Server API |
| **Distribution** | pip/uv | Single binary |

## Development

```bash
# Run with hot reloading (requires cargo-watch)
cargo watch -x run

# Run tests
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

## License

MIT License - See [LICENSE](../LICENSE) for details.

//! Web server module — serves the Rho TUI in a browser via xterm.js + WebSocket + PTY.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use rust_embed::Embed;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::cli::WebArgs;

// ── Embedded static assets ─────────────────────────────────────────────────

#[derive(Embed)]
#[folder = "web/"]
struct Asset;

// ── Public entry point ─────────────────────────────────────────────────────

/// Start the web server that serves xterm.js and proxies to a PTY running `rho`.
pub async fn run_web_server(args: &WebArgs) -> Result<()> {
    tracing_subscriber::fmt::init();

    // Propagate --override-with-envs via env var for PTY subprocesses
    if args.override_with_envs {
        std::env::set_var("RHO_OVERRIDE_WITH_ENVS", "1");
    }

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(get(static_handler));

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!(
        "\x1b[1;33mRho web UI:\x1b[0m http://{}",
        listener.local_addr()?
    );
    println!("Press Ctrl+C to stop the server.");

    axum::serve(listener, app).await?;
    Ok(())
}

// ── Static file handler ────────────────────────────────────────────────────

async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => {
            // Fall back to index.html for SPA-style routing
            match Asset::get("index.html") {
                Some(content) => {
                    Html(String::from_utf8_lossy(&content.data).to_string()).into_response()
                }
                None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
            }
        }
    }
}

// ── WebSocket handler ──────────────────────────────────────────────────────

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    if let Err(e) = handle_socket_inner(socket).await {
        error!("WebSocket session error: {}", e);
    }
}

async fn handle_socket_inner(socket: WebSocket) -> Result<()> {
    let (mut ws_sink, mut ws_stream) = {
        use futures::StreamExt;
        socket.split()
    };

    // Spawn `rho` in a PTY
    let pty_system = native_pty_system();

    let pty_pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let exe = std::env::current_exe()?;
    let mut cmd = CommandBuilder::new(&exe);
    // Forward relevant environment variables
    for var in &[
        "LLM_API_KEY",
        "LLM_MODEL",
        "LLM_BASE_URL",
        "OPENHANDS_SESSION_API_KEY",
        "RHO_THEME",
        "TERM",
        "HOME",
        "PATH",
    ] {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, val);
        }
    }
    // Ensure TERM is set for proper TUI rendering
    cmd.env("TERM", "xterm-256color");

    // Forward --override-with-envs to the spawned TUI if set
    if std::env::var("RHO_OVERRIDE_WITH_ENVS").is_ok() {
        cmd.arg("--override-with-envs");
    }

    let child = pty_pair.slave.spawn_command(cmd)?;
    let child = Arc::new(std::sync::Mutex::new(child));

    // Get reader/writer from the master side
    let reader = pty_pair.master.try_clone_reader()?;
    let writer = pty_pair.master.take_writer()?;

    // Keep master alive so the PTY doesn't close
    let master = Arc::new(std::sync::Mutex::new(pty_pair.master));

    // Channel for PTY output -> WebSocket
    let (pty_tx, mut pty_rx) = mpsc::channel::<Vec<u8>>(256);

    // Spawn a blocking thread to read PTY output
    let pty_reader_handle = tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if pty_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Task: PTY output -> WebSocket
    let ws_send_handle = tokio::spawn(async move {
        use axum::extract::ws::Message;
        use futures::SinkExt;
        while let Some(data) = pty_rx.recv().await {
            if ws_sink.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });

    // Task: WebSocket input -> PTY
    let master_for_resize = Arc::clone(&master);
    let ws_recv_handle = tokio::spawn(async move {
        use futures::StreamExt;
        use std::io::Write;
        let mut writer = writer;
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Text(text) => {
                    // Check for resize control message
                    if let Ok(ctrl) = serde_json::from_str::<serde_json::Value>(&text) {
                        if ctrl.get("type").and_then(|t| t.as_str()) == Some("resize") {
                            let cols =
                                ctrl.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                            let rows =
                                ctrl.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                            if let Ok(master) = master_for_resize.lock() {
                                let _ = master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                });
                            }
                            continue;
                        }
                    }
                    // Regular text input
                    let _ = writer.write_all(text.as_bytes());
                    let _ = writer.flush();
                }
                Message::Binary(data) => {
                    let _ = writer.write_all(&data);
                    let _ = writer.flush();
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either side to finish
    tokio::select! {
        _ = ws_send_handle => {},
        _ = ws_recv_handle => {},
    }

    // Clean up: kill the child process
    pty_reader_handle.abort();
    if let Ok(mut child) = child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }

    info!("WebSocket session ended");
    Ok(())
}

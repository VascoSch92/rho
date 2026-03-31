//! Rho - Terminal User Interface
//!
//! A Ratatui-based TUI that connects to the OpenHands Agent Server.

mod cli;
mod client;
mod config;
mod events;
mod handlers;
mod state;
mod ui;

use std::io;
use std::process::{Child, Command};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::Args;
use client::{AgentServerClient, EventStream, ExecutionStatus, LLMConfig};
use handlers::{handle_key_event, process_command};
use state::{AppState, ConfirmationPolicy, DisplayMessage, Notification};

/// Ensure the .rho data directory exists and return its path.
/// The agent server creates `workspace/conversations/` inside this directory.
fn ensure_rho_dir() -> std::path::PathBuf {
    let rho_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".rho");
    std::fs::create_dir_all(&rho_dir).expect("Failed to create .rho/");
    rho_dir
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();
    let rho_dir = ensure_rho_dir();

    // Initialize logging - write to file when debug is enabled
    let log_level = if args.debug { "debug" } else { "warn" };

    if args.debug {
        // Write logs to .rho/rho.log so they're visible even with TUI
        let log_path = rho_dir.join("rho.log");
        let log_file = std::fs::File::create(&log_path).expect("Failed to create log file");
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(std::sync::Mutex::new(log_file)),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
            )
            .with(tracing_subscriber::fmt::layer().with_target(false))
            .init();
    }

    info!("Starting Rho TUI");
    info!("Server: {}", args.server);

    // Start the embedded agent server in the background
    let mut server_process = start_agent_server(&args.server, &rho_dir);

    // Run the TUI application (it will poll for server readiness)
    let result = run_app(args, server_process.is_some()).await;

    // Stop the agent server on exit (kill the whole process group)
    stop_agent_server(&mut server_process);

    result
}

/// Start the OpenHands agent server from the project's .venv.
/// Returns None if the venv or the module is not available.
fn start_agent_server(server_url: &str, rho_dir: &std::path::Path) -> Option<Child> {
    let parsed = url::Url::parse(server_url).ok()?;
    let host = parsed.host_str().unwrap_or("127.0.0.1").to_string();
    let port = parsed.port().unwrap_or(8000);

    let venv_python = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".venv")
        .join("bin")
        .join("python");

    if !venv_python.exists() {
        warn!(
            "Agent server venv not found at {}. Run `make build` first.",
            venv_python.display()
        );
        return None;
    }

    info!("Starting agent server on {}:{}", host, port);

    let mut cmd = Command::new(&venv_python);
    cmd.args([
        "-m",
        "openhands.agent_server",
        "--port",
        &port.to_string(),
        "--host",
        &host,
    ])
    .current_dir(rho_dir)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());

    // Start in its own process group so we can kill all sub-processes on exit
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    match cmd.spawn() {
        Ok(child) => {
            info!("Agent server started (pid={})", child.id());
            Some(child)
        }
        Err(e) => {
            warn!("Failed to start agent server: {}", e);
            None
        }
    }
}

/// Stop the agent server and all its child processes.
fn stop_agent_server(server_process: &mut Option<Child>) {
    if let Some(ref mut child) = server_process {
        let pid = child.id();
        info!("Stopping agent server (pid={})", pid);

        // Send SIGTERM to the entire process group so sub-processes (uvicorn workers, etc.) also exit
        #[cfg(unix)]
        {
            // Kill the process group (negative pid)
            unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            }
            // Give it a moment to shut down gracefully, then force-kill
            std::thread::sleep(Duration::from_secs(2));
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = child.kill();
        }

        let _ = child.wait();
    }
}

async fn run_app(args: Args, server_launched: bool) -> Result<()> {
    // Setup terminal
    // Note: We DON'T enable mouse capture so users can select/copy text with mouse
    // Use keyboard (arrows, Page Up/Down) for scrolling instead
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Enter alternate screen and clear scrollback buffer so mouse scroll doesn't show
    // previous terminal output
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        Clear(ClearType::Purge) // Clear scrollback buffer
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Hide the blinking terminal cursor - we render our own visual cursor
    terminal.hide_cursor()?;

    // Create application state
    let mut state = AppState::default();
    if args.always_approve {
        state.confirmation_policy = ConfirmationPolicy::NeverConfirm;
    }
    state.server_starting = server_launched;
    state.theme = args.theme.to_theme();
    state.theme_name = args.theme;

    // Set workspace path for display
    let workspace_path = args
        .workspace
        .clone()
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| ".".to_string());
    state.set_workspace(workspace_path);

    // Parse model argument (format: "provider/model" or just "model")
    let (provider, model) = cli::parse_model_arg(&args.model);
    state.llm_provider = provider;
    state.llm_model = model;

    // Validate LLM API key is provided
    let llm_api_key = match &args.llm_api_key {
        Some(key) => {
            state.llm_api_key = key.clone();
            key.clone()
        }
        None => {
            error!("LLM_API_KEY is required. Set via --llm-api-key or LLM_API_KEY environment variable.");
            return Err(anyhow::anyhow!("LLM_API_KEY is required"));
        }
    };

    // Set base URL if provided
    state.llm_base_url = args.llm_base_url.clone();

    // Create API client
    let client = AgentServerClient::new(&args.server, args.session_api_key.clone());

    // If we didn't launch the server ourselves, check health immediately
    if !server_launched {
        match client.health().await {
            Ok(_) => {
                state.connected = true;
                state.notify(Notification::info("Connected", "Connected to Agent Server"));
            }
            Err(e) => {
                warn!("Failed to connect to server: {}", e);
                state.notify(Notification::warning(
                    "Connection Failed",
                    format!("Could not connect to {}: {}", args.server, e),
                ));
            }
        }
    }

    // Build LLM config for conversations
    let llm_config = {
        let config = LLMConfig::new(&args.model, &llm_api_key);
        if let Some(ref base_url) = args.llm_base_url {
            config.with_base_url(base_url)
        } else {
            config
        }
    };

    // Event stream for WebSocket events
    let mut event_stream: Option<EventStream> = None;

    // Main event loop
    let tick_rate = Duration::from_millis(100);
    let notification_duration = Duration::from_secs(5);

    // Track ticks for animation timing
    let mut tick_count: u64 = 0;
    let spinner_interval = 1; // Update spinner every tick (100ms)
    let fun_fact_interval = 100; // Change fun fact every 10 seconds (100 ticks)

    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, &state))?;

        // Update elapsed time
        state.update_elapsed();

        // Cleanup old notifications
        state.cleanup_notifications(notification_duration);

        // Animation updates
        tick_count = tick_count.wrapping_add(1);
        if tick_count.is_multiple_of(spinner_interval) {
            state.tick_spinner();
        }
        if tick_count.is_multiple_of(fun_fact_interval) {
            state.next_fun_fact();
        }

        // Poll server health while starting up (every ~1s = 10 ticks)
        if state.server_starting {
            state.server_starting_tick = state.server_starting_tick.wrapping_add(1);
            if tick_count.is_multiple_of(10) {
                match client.health().await {
                    Ok(_) => {
                        state.server_starting = false;
                        state.connected = true;
                        state.notify(Notification::info(
                            "Connected",
                            "✨ Agent is awake and ready!",
                        ));
                        info!("Agent server ready");
                    }
                    Err(_) => {
                        debug!("Server not ready yet, will retry...");
                    }
                }
            }
        }

        // Poll for events (with timeout for tick rate)
        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key) => {
                    // Handle key events based on current mode
                    if let Some(cmd) = handle_key_event(&mut state, key, &args) {
                        if process_command(
                            &mut state,
                            &client,
                            &mut event_stream,
                            cmd,
                            &args,
                            &llm_config,
                        )
                        .await?
                        {
                            break; // Exit requested
                        }
                    }
                }
                Event::Mouse(_) => {
                    // Mouse capture is disabled to allow text selection/copy
                    // Use keyboard (arrows, Page Up/Down) for scrolling
                }
                _ => {} // Ignore other events
            }
        }

        // Process WebSocket events
        if let Some(ref mut stream) = event_stream {
            while let Some(event) = stream.try_recv() {
                debug!("Received event: {:?}", event.type_name());
                state.process_event(event);
            }

            // Check if stream is still connected - attempt reconnect if disconnected
            if !stream.is_connected() {
                warn!("WebSocket disconnected, attempting to reconnect...");

                // Try to reconnect if we have a conversation
                if let Some(conv_id) = state.conversation_id {
                    let ws_url = client.conversation_websocket_url(conv_id);
                    match EventStream::connect(&ws_url).await {
                        Ok(new_stream) => {
                            info!("WebSocket reconnected successfully");
                            event_stream = Some(new_stream);
                        }
                        Err(e) => {
                            error!("Failed to reconnect WebSocket: {}", e);
                            // Only show error if we're supposed to be running
                            if state.is_running() {
                                state.add_message(DisplayMessage::error(
                                    "WebSocket disconnected. Reconnect failed.",
                                ));
                                state.execution_status = ExecutionStatus::Error;
                            }
                            event_stream = None;
                        }
                    }
                } else {
                    event_stream = None;
                }
            }
        } else if let Some(conv_id) = state.conversation_id {
            if state.is_running() {
                // No stream but we have a conversation and it's running - try to connect
                let ws_url = client.conversation_websocket_url(conv_id);
                match EventStream::connect(&ws_url).await {
                    Ok(stream) => {
                        info!("WebSocket connected (was missing)");
                        event_stream = Some(stream);
                    }
                    Err(e) => {
                        debug!("Failed to connect missing WebSocket: {}", e);
                    }
                }
            }
        }

        // Fetch stats when execution finishes (server doesn't send metrics updates via WebSocket)
        if state.needs_stats_refresh {
            if let Some(conversation_id) = state.conversation_id {
                state.needs_stats_refresh = false;
                match client.get_conversation_state(conversation_id).await {
                    Ok(full_state) => {
                        if let Some(stats) = full_state.get("stats") {
                            state.parse_metrics(stats);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch conversation stats: {}", e);
                    }
                }
            }
        }

        // Check for exit - only when explicitly requested
        if state.should_exit {
            info!("Exit flag set, breaking main loop");
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    info!("Rho TUI exited");
    Ok(())
}

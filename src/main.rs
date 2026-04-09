//! Rho - Terminal User Interface
//!
//! A Ratatui-based TUI that connects to the OpenHands Agent Server.

mod cli;
mod client;
mod config;
mod events;
mod handlers;
mod headless;
mod state;
mod ui;
mod web;

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

use cli::{Args, Cli};
use client::{AgentServerClient, EventStream, ExecutionStatus, LLMConfig};
use config::RhoConfig;
use handlers::{handle_key_event, process_command};
use state::{AppState, DisplayMessage, Notification};

/// Ensure the .rho data directory exists and return its path.
/// The agent server creates `workspace/conversations/` inside this directory.
fn ensure_rho_dir() -> std::path::PathBuf {
    let rho_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".rho");
    std::fs::create_dir_all(&rho_dir).expect("Failed to create .rho/");
    rho_dir
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = <Cli as clap::Parser>::parse();

    // Dispatch to subcommands
    match cli.command {
        Some(cli::Command::Web(ref web_args)) => {
            return web::run_web_server(web_args).await;
        }
        Some(cli::Command::Headless(ref headless_args)) => {
            let code = headless::run_headless(headless_args).await?;
            std::process::exit(code);
        }
        None => {}
    }

    let args = cli.tui;
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

/// Start the OpenHands agent server from `dist/openhands-agent-server`.
/// Returns None if the binary is not found.
fn start_agent_server(server_url: &str, rho_dir: &std::path::Path) -> Option<Child> {
    let parsed = url::Url::parse(server_url).ok()?;
    let host = parsed.host_str().unwrap_or("127.0.0.1").to_string();
    let port = parsed.port().unwrap_or(8000);

    let dist_binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("dist")
        .join("openhands-agent-server")
        .join("openhands-agent-server");

    if !dist_binary.exists() {
        warn!(
            "Agent server binary not found at {}.",
            dist_binary.display()
        );
        return None;
    }

    info!("Starting agent server on {}:{}", host, port);

    let mut cmd = Command::new(&dist_binary);
    cmd.args(["--port", &port.to_string(), "--host", &host]);

    cmd.current_dir(rho_dir)
        .env("OH_CONVERSATIONS_PATH", "conversations")
        .env("OH_BASH_EVENTS_DIR", "bash_events")
        .env("OPENHANDS_SUPPRESS_BANNER", "1")
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

        // Send SIGTERM to the entire process group so sub-processes also exit
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

/// Print goodbye message with optional resume instructions.
fn print_goodbye(conversation_id: Option<uuid::Uuid>) {
    println!("\x1b[1;33mGoodbye! 👋\x1b[0m");
    if let Some(conv_id) = conversation_id {
        println!("Conversation ID: \x1b[1;34m{}\x1b[0m", conv_id.as_simple());
        println!(
            "\x1b[2mHint: run rho --resume {} to resume this conversation.\x1b[0m",
            conv_id
        );
    }
    println!();
}

/// Attempt to connect a WebSocket event stream for a conversation.
///
/// Centralizes URL building, connect, and structured logging for all three
/// connect call sites (resume, reconnect, lazy). Each call site decides what
/// to do with the result — this helper only connects and logs.
///
/// `context` is a short label included in log messages so users can tell
/// which call site the log came from.
async fn try_connect_event_stream(
    client: &AgentServerClient,
    conv_id: uuid::Uuid,
    context: &str,
) -> Option<EventStream> {
    let ws_url = client.conversation_websocket_url(conv_id);
    match EventStream::connect(&ws_url).await {
        Ok(stream) => {
            info!("WebSocket connected ({})", context);
            Some(stream)
        }
        Err(e) => {
            warn!("Failed to connect WebSocket ({}): {}", context, e);
            None
        }
    }
}

async fn run_app(args: Args, server_launched: bool) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Enter alternate screen, clear scrollback, enable mouse capture for scroll wheel.
    // Hold modifier key (Ctrl/Shift/Cmd depending on terminal) to select text.
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        Clear(ClearType::Purge),
        crossterm::event::EnableMouseCapture,
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Hide the blinking terminal cursor - we render our own visual cursor
    terminal.hide_cursor()?;

    // Load configuration (from ~/.config/rho/config.toml or defaults)
    let mut config = RhoConfig::load();
    // Priority: CLI flag > .rho/config.toml > embedded default
    if let Some(ref theme) = args.theme {
        config.theme_name = theme.clone();
    }
    // Always persist the resolved theme to .rho/config.toml
    if let Err(e) = crate::config::save_theme(&config.theme_name) {
        tracing::warn!("Failed to save theme: {}", e);
    }

    // Extract LLM settings before moving config into AppState
    let config_llm = config.llm.clone();

    // Apply env var overrides only with --override-with-envs
    let env_model = if args.override_with_envs {
        std::env::var("LLM_MODEL").ok()
    } else {
        None
    };
    let env_api_key = if args.override_with_envs {
        std::env::var("LLM_API_KEY").ok()
    } else {
        None
    };
    let env_base_url = if args.override_with_envs {
        std::env::var("LLM_BASE_URL").ok()
    } else {
        None
    };

    // Resolve LLM settings: env (if --override-with-envs) > config > defaults
    let effective_model = if let Some(m) = env_model {
        m
    } else if let Some(ref m) = config_llm.model {
        m.clone()
    } else {
        "anthropic/claude-sonnet-4-5-20250929".to_string()
    };
    let effective_base_url = env_base_url.or(config_llm.base_url.clone());

    // Create application state from config
    let mut state = AppState::with_config(config);
    state.confirmation_policy = args.permission_mode;
    state.server_starting = server_launched;

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

    // Apply LLM settings to state
    let (provider, model) = cli::parse_model_arg(&effective_model);
    state.llm.provider = provider;
    state.llm.model = model;
    state.llm.base_url = effective_base_url.clone();

    // API key: env (if --override-with-envs) > config
    let llm_api_key = if let Some(ref key) = env_api_key {
        state.llm.api_key = key.clone();
        key.clone()
    } else if let Some(ref key) = config_llm.api_key {
        if !key.is_empty() {
            state.llm.api_key = key.clone();
            key.clone()
        } else {
            error!("LLM_API_KEY is required. Set via --override-with-envs + LLM_API_KEY env, or /settings.");
            return Err(anyhow::anyhow!("LLM_API_KEY is required"));
        }
    } else {
        error!("LLM_API_KEY is required. Set via --override-with-envs + LLM_API_KEY env, or /settings.");
        return Err(anyhow::anyhow!("LLM_API_KEY is required"));
    };

    // Persist env overrides to .rho/config.toml so they survive restarts
    if args.override_with_envs {
        if let Err(e) = crate::config::save_llm(
            &effective_model,
            &llm_api_key,
            effective_base_url.as_deref(),
        ) {
            tracing::warn!("Failed to persist LLM settings: {}", e);
        }
    }

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
        let config = LLMConfig::new(&effective_model, &llm_api_key);
        if let Some(ref base_url) = effective_base_url {
            config.with_base_url(base_url)
        } else {
            config
        }
    };

    // Event stream for WebSocket events
    let mut event_stream: Option<EventStream> = None;

    // Handle --resume flag: replay events and connect WebSocket
    if let Some(conv_id) = args.resume {
        state.reset_conversation();
        state.conversation_id = Some(conv_id);

        let conv_id_str = conv_id.as_simple().to_string();
        let events = state::conversations::load_events(&conv_id_str);
        info!(
            "Resuming conversation {} ({} events)",
            conv_id,
            events.len()
        );
        state.replaying = true;
        for event in events {
            state.process_event(event);
        }
        state.replaying = false;
        state.execution_status = client::ExecutionStatus::Idle;

        // Connect WebSocket
        if let Some(stream) = try_connect_event_stream(&client, conv_id, "resume").await {
            event_stream = Some(stream);
            state.connected = true;
            // Fetch title/metrics
            if let Ok(full_state) = client.get_conversation_state(conv_id).await {
                if let Some(title) = full_state.get("title").and_then(|v| v.as_str()) {
                    state.conversation_title = Some(title.to_string());
                }
                if let Some(stats) = full_state.get("stats") {
                    state.parse_metrics(stats);
                }
            }
        }
    }

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
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        crossterm::event::MouseEventKind::ScrollUp => {
                            state.scroll_up(state.scroll_lines);
                        }
                        crossterm::event::MouseEventKind::ScrollDown => {
                            state.scroll_down(state.scroll_lines);
                        }
                        _ => {} // Ignore other mouse events
                    }
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
                    if let Some(new_stream) =
                        try_connect_event_stream(&client, conv_id, "reconnect").await
                    {
                        event_stream = Some(new_stream);
                    } else {
                        // Only show error if we're supposed to be running
                        if state.is_running() {
                            state.add_message(DisplayMessage::error(
                                "WebSocket disconnected. Reconnect failed.",
                            ));
                            state.execution_status = ExecutionStatus::Error;
                        }
                        event_stream = None;
                    }
                } else {
                    event_stream = None;
                }
            }
        } else if let Some(conv_id) = state.conversation_id {
            if state.is_running() {
                // No stream but we have a conversation and it's running - try to connect
                if let Some(stream) = try_connect_event_stream(&client, conv_id, "lazy").await {
                    event_stream = Some(stream);
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

        // Drain message queue — send next queued message when agent becomes idle
        if !state.message_queue.is_empty() && !state.is_running() && state.conversation_id.is_some()
        {
            if let Some(queued_msg) = state.message_queue.pop_front() {
                info!(
                    "Sending queued message ({} remaining)",
                    state.message_queue.len()
                );
                state.add_message(crate::state::DisplayMessage::user(&queued_msg));
                let conv_id = state.conversation_id.unwrap();
                state.start_timer();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
                if let Err(e) = client.send_message(conv_id, &queued_msg, true).await {
                    error!("Failed to send queued message: {}", e);
                    state.add_message(crate::state::DisplayMessage::error(format!(
                        "Failed to send: {}",
                        e
                    )));
                    state.execution_status = ExecutionStatus::Idle;
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
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    print_goodbye(state.conversation_id);

    info!("Rho TUI exited");
    Ok(())
}

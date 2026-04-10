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
use client::{
    try_connect_event_stream, AgentServerClient, EventStream, ExecutionStatus, LLMConfig,
};
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

    // `rho --resume` (no value) prints the list of recent conversations and exits
    // without starting the server. `--last` is handled later inside the TUI path.
    if matches!(args.resume.as_deref(), Some("")) && !args.last {
        print_recent_conversations();
        return Ok(());
    }

    let rho_dir = ensure_rho_dir();

    // Initialize logging - write to file when debug is enabled
    let log_level = if args.debug { "debug" } else { "warn" };

    // Always write logs to .rho/rho.log (never to stderr — it corrupts the TUI).
    // --debug enables debug-level logging, otherwise only warn+error are logged.
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

    info!("Starting Rho TUI");
    info!("Server: {}", args.server);

    // Start the embedded agent server in the background
    let mut server_process = start_agent_server(&args.server);

    // Run the TUI application (it will poll for server readiness)
    let result = run_app(args, server_process.is_some()).await;

    // Stop the agent server on exit (kill the whole process group)
    stop_agent_server(&mut server_process);

    result
}

/// Start the OpenHands agent server from `dist/openhands-agent-server`.
/// Returns None if the binary is not found.
fn start_agent_server(server_url: &str) -> Option<Child> {
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

    let server_data_dir = config::data_dir();
    // Ensure conversations dir exists so the agent server can write to it
    let _ = std::fs::create_dir_all(server_data_dir.join("conversations"));
    cmd.current_dir(&server_data_dir)
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

/// Format an ISO-8601 timestamp as a relative age string.
///
/// - `<` 1 hour → `"Xm ago"`
/// - `<` 1 day → `"Xh ago"`
/// - yesterday → `"yesterday"`
/// - `<` 7 days → `"X days ago"`
/// - older → `"YYYY-MM-DD"`
fn format_relative_time(iso: &str) -> String {
    use chrono::{DateTime, Local, NaiveDate, Utc};
    if iso.is_empty() {
        return "unknown".to_string();
    }
    let parsed = DateTime::parse_from_rfc3339(iso)
        .map(|d| d.with_timezone(&Utc))
        .or_else(|_| {
            DateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.f").map(|d| d.with_timezone(&Utc))
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        });
    let Ok(when) = parsed else {
        return iso.to_string();
    };

    let now = Utc::now();
    let delta = now.signed_duration_since(when);
    if delta.num_seconds() < 0 {
        return "just now".to_string();
    }
    let minutes = delta.num_minutes();
    let hours = delta.num_hours();

    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else {
        // Compare calendar dates (in local tz) for "yesterday"
        let local_when = when.with_timezone(&Local).date_naive();
        let local_today: NaiveDate = Local::now().date_naive();
        let diff_days = (local_today - local_when).num_days();
        if diff_days == 1 {
            "yesterday".to_string()
        } else if diff_days < 7 {
            format!("{} days ago", diff_days)
        } else {
            local_when.format("%Y-%m-%d").to_string()
        }
    }
}

/// Print a formatted list of recent conversations for `--resume` without an ID.
fn print_recent_conversations() {
    const MAX_ENTRIES: usize = 15;
    const RULE: &str =
        "────────────────────────────────────────────────────────────────────────────────";

    let entries = state::conversations::scan_conversations();
    if entries.is_empty() {
        println!("\x1b[1;33mNo conversations found.\x1b[0m");
        println!("Start a new one with: \x1b[1mrho\x1b[0m");
        println!();
        return;
    }

    println!("\x1b[1;33mRecent Conversations:\x1b[0m");
    println!("\x1b[2m{}\x1b[0m", RULE);

    for (i, conv) in entries.iter().take(MAX_ENTRIES).enumerate() {
        let age = format_relative_time(&conv.updated_at);
        let preview = if conv.first_message.trim().is_empty() {
            "(No user message)".to_string()
        } else {
            let line = conv.first_message.lines().next().unwrap_or("");
            if line.chars().count() > 72 {
                let trunc: String = line.chars().take(69).collect();
                format!("{}...", trunc)
            } else {
                line.to_string()
            }
        };

        println!(
            "{:>3}. \x1b[1;34m{}\x1b[0m \x1b[2m({})\x1b[0m",
            i + 1,
            conv.id,
            age
        );
        println!("     \x1b[2m{}\x1b[0m", preview);
        if i + 1 < entries.len().min(MAX_ENTRIES) {
            println!();
        }
    }

    println!("\x1b[2m{}\x1b[0m", RULE);
    println!("To resume a conversation, use: \x1b[1mrho --resume <conversation-id>\x1b[0m");
    if entries.len() > 1 {
        println!("Or resume the most recent with:  \x1b[1mrho --resume --last\x1b[0m");
    }
    println!();
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

    // Resolve the conversation to resume:
    // - `--last` (or `--resume --last`) picks the most recent conversation on disk
    // - `--resume <id>` uses the supplied id directly
    // - `--resume` with no value is equivalent to `--last`
    let resume_id: Option<uuid::Uuid> = {
        // Did the user pass `--resume <id>` with a non-empty value?
        let explicit_id = args.resume.as_deref().filter(|s| !s.is_empty());
        let wants_latest = args.last || matches!(args.resume.as_deref(), Some(""));

        if let Some(id_str) = explicit_id {
            match uuid::Uuid::parse_str(id_str) {
                Ok(id) => Some(id),
                Err(e) => {
                    error!("Invalid conversation id '{}': {}", id_str, e);
                    return Err(anyhow::anyhow!("Invalid conversation id: {}", e));
                }
            }
        } else if wants_latest {
            match state::conversations::scan_conversations().first() {
                Some(conv) => match uuid::Uuid::parse_str(&conv.id) {
                    Ok(id) => {
                        println!("Resuming latest conversation: {}", id);
                        Some(id)
                    }
                    Err(e) => {
                        error!("Invalid conversation id on disk: {}", e);
                        return Err(anyhow::anyhow!("Corrupted conversation id: {}", e));
                    }
                },
                None => {
                    error!("No conversations found to resume.");
                    return Err(anyhow::anyhow!("No conversations found to resume."));
                }
            }
        } else {
            None
        }
    };

    // Handle --resume / --last: replay events and connect WebSocket
    if let Some(conv_id) = resume_id {
        state.reset_conversation();
        info!("Resuming conversation {}", conv_id);
        handlers::resume_conversation(&mut state, &client, &mut event_stream, conv_id, &llm_config, &args).await;
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
        poll_server_startup(&mut state, &client, tick_count).await;

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

        // Drain WebSocket events and handle reconnect / lazy connect
        process_websocket_events(&mut state, &client, &mut event_stream).await;

        // Fetch stats when execution finishes (server doesn't push metrics via WebSocket)
        refresh_stats_if_needed(&mut state, &client).await;

        // Drain message queue — send next queued message when agent becomes idle
        drain_message_queue(&mut state, &client).await;

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

// ── Event loop helpers ─────────────────────────────────────────────────────
//
// These functions are called once per tick from `run_app`'s main loop. They
// take `&mut state` / `&client` and mutate in place, keeping the loop itself
// short and readable.

/// Poll `/health` every ~1s while the server is still starting.
async fn poll_server_startup(state: &mut AppState, client: &AgentServerClient, tick_count: u64) {
    if !state.server_starting {
        return;
    }
    state.server_starting_tick = state.server_starting_tick.wrapping_add(1);
    if !tick_count.is_multiple_of(10) {
        return;
    }
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

/// Drain events from the active WebSocket stream, attempt reconnect if it
/// has dropped, or lazily connect one if we don't have one yet.
async fn process_websocket_events(
    state: &mut AppState,
    client: &AgentServerClient,
    event_stream: &mut Option<EventStream>,
) {
    if let Some(ref mut stream) = event_stream {
        while let Some(event) = stream.try_recv() {
            debug!("Received event: {:?}", event.type_name());
            state.process_event(event);
        }

        // Check if stream is still connected — attempt reconnect only if running
        if !stream.is_connected() {
            if state.is_running() {
                warn!("WebSocket disconnected, attempting to reconnect...");
                if let Some(conv_id) = state.conversation_id {
                    if let Some(new_stream) =
                        try_connect_event_stream(client, conv_id, "reconnect").await
                    {
                        *event_stream = Some(new_stream);
                        return;
                    }
                    state.add_message(DisplayMessage::error(
                        "WebSocket disconnected. Reconnect failed.",
                    ));
                    state.execution_status = ExecutionStatus::Error;
                }
            }
            // Not running or reconnect failed — drop the stream silently
            *event_stream = None;
        }
    } else if let Some(conv_id) = state.conversation_id {
        if state.is_running() {
            // No stream but we have a running conversation — try to connect lazily
            if let Some(stream) = try_connect_event_stream(client, conv_id, "lazy").await {
                *event_stream = Some(stream);
            }
        }
    }
}

/// Fetch metrics/stats when execution has just finished.
async fn refresh_stats_if_needed(state: &mut AppState, client: &AgentServerClient) {
    if !state.needs_stats_refresh {
        return;
    }
    let Some(conversation_id) = state.conversation_id else {
        return;
    };
    state.needs_stats_refresh = false;
    match client.get_conversation_state(conversation_id).await {
        Ok(full_state) => {
            if let Some(stats) = full_state.get("stats") {
                state.parse_metrics(stats);
            }
        }
        Err(e) => {
            warn!("Failed to fetch conversation stats: {}", e);
        }
    }
}

/// If the agent just became idle and we have queued messages, send the next
/// one — its display entry is added at this moment so the visible order is
/// always `PROMPT_1, ANSWER_1, PROMPT_2, ANSWER_2`.
async fn drain_message_queue(state: &mut AppState, client: &AgentServerClient) {
    if state.message_queue.is_empty() || state.is_running() || state.conversation_id.is_none() {
        return;
    }
    let Some(queued_msg) = state.message_queue.pop_front() else {
        return;
    };
    info!(
        "Sending queued message ({} remaining)",
        state.message_queue.len()
    );
    state.add_message(DisplayMessage::user(&queued_msg));
    let conv_id = state.conversation_id.unwrap();
    state.start_timer();
    state.randomize_spinner();
    state.execution_status = ExecutionStatus::Running;
    if let Err(e) = client.send_message(conv_id, &queued_msg, true).await {
        error!("Failed to send queued message: {}", e);
        state.add_message(DisplayMessage::error(format!("Failed to send: {}", e)));
        state.execution_status = ExecutionStatus::Idle;
    }
}

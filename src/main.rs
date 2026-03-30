//! Rho - Terminal User Interface
//!
//! A Ratatui-based TUI that connects to the OpenHands Agent Server.

mod client;
mod events;
mod state;
mod ui;

use std::io;
use std::process::{Child, Command};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, 
        Clear, ClearType,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use client::{
    AgentServerClient, AgentConfig, EventStream, ExecutionStatus,
    LLMConfig, LocalWorkspace, SendMessageRequest, ServerConfirmationPolicy, StartConversationRequest,
};
use ui::ConfirmOption;
use ui::settings_modal::SETTINGS_FIELD_COUNT;
use ui::theme::Theme;
use state::{AppState, ConfirmationPolicy, DisplayMessage, InputMode, LlmProvider, Notification};

/// Rho - AI-powered coding assistant
#[derive(Parser, Debug)]
#[command(name = "rho")]
#[command(version)]
#[command(about = "Rho - Terminal UI for OpenHands Agent Server", long_about = None)]
struct Args {
    /// Agent Server URL
    #[arg(short, long, default_value = "http://127.0.0.1:8000")]
    server: String,

    /// Session API key for authentication (can also use OPENHANDS_SESSION_API_KEY env var)
    #[arg(long, env = "OPENHANDS_SESSION_API_KEY")]
    session_api_key: Option<String>,

    /// LLM model name (e.g., "anthropic/claude-sonnet-4-5-20250929", "openai/gpt-4o")
    #[arg(short, long, env = "LLM_MODEL", default_value = "anthropic/claude-sonnet-4-5-20250929")]
    model: String,

    /// LLM API key (can also use LLM_API_KEY env var)
    #[arg(long, env = "LLM_API_KEY")]
    llm_api_key: Option<String>,

    /// LLM base URL (optional, for custom endpoints)
    #[arg(long, env = "LLM_BASE_URL")]
    llm_base_url: Option<String>,

    /// Working directory for the agent
    #[arg(short, long)]
    workspace: Option<String>,

    /// Resume an existing conversation
    #[arg(short, long)]
    resume: Option<Uuid>,

    /// Auto-approve all actions (no confirmation)
    #[arg(long)]
    always_approve: bool,

    /// Skip exit confirmation
    #[arg(long)]
    exit_without_confirmation: bool,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Color theme (rho, dracula, catppuccin, tokyonight, solarized, gruvbox)
    #[arg(long, env = "RHO_THEME", default_value = "rho")]
    theme: String,
}

/// Application commands from user input
#[derive(Debug)]
enum AppCommand {
    SendMessage(String),
    RunBashCommand(String),
    NewConversation,
    Pause,
    ConfirmYes,
    ConfirmNo,
    ConfirmAll,
    ConfirmDefer,
    SetPolicy(ConfirmationPolicy),
    Quit,
    ForceQuit,
    CancelQuit,
}

/// Parse model argument in format "provider/model" or just "model"
fn parse_model_arg(model_arg: &str) -> (LlmProvider, String) {
    if let Some((provider_str, model)) = model_arg.split_once('/') {
        let provider = match provider_str.to_lowercase().as_str() {
            "openhands" => LlmProvider::OpenHands,
            "anthropic" => LlmProvider::Anthropic,
            "openai" => LlmProvider::OpenAI,
            "mistral" => LlmProvider::Mistral,
            "google" | "gemini" => LlmProvider::Google,
            "deepseek" => LlmProvider::DeepSeek,
            other => LlmProvider::Other(other.to_string()),
        };
        (provider, model.to_string())
    } else {
        // No provider prefix, try to guess from model name
        let model = model_arg.to_string();
        let provider = if model.contains("claude") {
            LlmProvider::Anthropic
        } else if model.contains("gpt") || model.starts_with("o1") || model.starts_with("o3") || model.starts_with("o4") {
            LlmProvider::OpenAI
        } else if model.contains("gemini") {
            LlmProvider::Google
        } else if model.contains("devstral") {
            LlmProvider::Mistral
        } else if model.contains("deepseek") {
            LlmProvider::DeepSeek
        } else {
            LlmProvider::Anthropic // Default
        };
        (provider, model)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging - write to file when debug is enabled
    let log_level = if args.debug { "debug" } else { "warn" };
    
    if args.debug {
        // Write logs to file so they're visible even with TUI
        let log_file = std::fs::File::create("rho.log")
            .expect("Failed to create log file");
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(std::sync::Mutex::new(log_file))
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
    let mut server_process = start_agent_server(&args.server);

    // Run the TUI application (it will poll for server readiness)
    let result = run_app(args, server_process.is_some()).await;

    // Stop the agent server on exit (kill the whole process group)
    stop_agent_server(&mut server_process);

    result
}

/// Start the OpenHands agent server from the project's .venv.
/// Returns None if the venv or the module is not available.
fn start_agent_server(server_url: &str) -> Option<Child> {
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
    cmd.args(["-m", "openhands.agent_server", "--port", &port.to_string(), "--host", &host])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Start in its own process group so we can kill all sub-processes on exit
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    match cmd.spawn()
    {
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
            use std::os::unix::process::ExitStatusExt;
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
        Clear(ClearType::Purge)  // Clear scrollback buffer
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Hide the blinking terminal cursor - we render our own visual cursor
    terminal.hide_cursor()?;

    // Create application state
    let mut state = AppState::new(args.server.clone());
    if args.always_approve {
        state.confirmation_policy = ConfirmationPolicy::NeverConfirm;
    }
    state.server_starting = server_launched;
    state.theme = Theme::by_name(&args.theme);
    state.theme_name = args.theme.to_lowercase();

    // Set workspace path for display
    let workspace_path = args.workspace.clone()
        .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".to_string());
    state.set_workspace(workspace_path);

    // Parse model argument (format: "provider/model" or just "model")
    let (provider, model) = parse_model_arg(&args.model);
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

    // Channel for commands from event loop (reserved for future use)
    let (_cmd_tx, _cmd_rx) = mpsc::unbounded_channel::<AppCommand>();

    // Event stream for WebSocket events
    let mut event_stream: Option<EventStream> = None;

    // Main event loop
    let tick_rate = Duration::from_millis(100);
    let notification_duration = Duration::from_secs(5);

    // Track ticks for animation timing
    let mut tick_count: u64 = 0;
    let spinner_interval = 1;      // Update spinner every tick (100ms)
    let fun_fact_interval = 100;   // Change fun fact every 10 seconds (100 ticks)

    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, &state))?;

        // Update elapsed time
        state.update_elapsed();

        // Cleanup old notifications
        state.cleanup_notifications(notification_duration);

        // Animation updates
        tick_count = tick_count.wrapping_add(1);
        if tick_count % spinner_interval == 0 {
            state.tick_spinner();
        }
        if tick_count % fun_fact_interval == 0 {
            state.next_fun_fact();
        }

        // Poll server health while starting up (every ~1s = 10 ticks)
        if state.server_starting {
            state.server_starting_tick = state.server_starting_tick.wrapping_add(1);
            if tick_count % 10 == 0 {
                match client.health().await {
                    Ok(_) => {
                        state.server_starting = false;
                        state.connected = true;
                        state.notify(Notification::info("Connected", "Agent Server is ready"));
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
                    match handle_key_event(&mut state, key, &args) {
                        Some(cmd) => {
                            if process_command(&mut state, &client, &mut event_stream, cmd, &args, &llm_config)
                                .await?
                            {
                                break; // Exit requested
                            }
                        }
                        None => {}
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
                                state.add_message(DisplayMessage::error("WebSocket disconnected. Reconnect failed."));
                                state.execution_status = ExecutionStatus::Error;
                            }
                            event_stream = None;
                        }
                    }
                } else {
                    event_stream = None;
                }
            }
        } else if state.conversation_id.is_some() && state.is_running() {
            // No stream but we have a conversation and it's running - try to connect
            let conv_id = state.conversation_id.unwrap();
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    info!("Rho TUI exited");
    Ok(())
}

/// Handle key events and return an optional command
fn handle_key_event(
    state: &mut AppState,
    key: event::KeyEvent,
    args: &Args,
) -> Option<AppCommand> {
    // Global key bindings
    match (key.code, key.modifiers) {
        // Quit shortcuts
        (KeyCode::Char('q'), KeyModifiers::CONTROL) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if args.exit_without_confirmation {
                return Some(AppCommand::ForceQuit);
            } else if state.exit_confirmation_pending {
                return Some(AppCommand::CancelQuit);
            } else {
                state.exit_confirmation_pending = true;
                return None;
            }
        }
        // Expand/collapse all actions
        (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
            state.toggle_all_actions();
            return None;
        }
        _ => {}
    }

    // Handle token modal
    if state.show_token_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                state.show_token_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle help modal
    if state.show_help_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                state.show_help_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle policy modal
    if state.show_policy_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                state.show_policy_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle settings modal
    if state.show_settings_modal {
        return handle_settings_modal_input(state, key);
    }

    // Handle notification modal - any key dismisses
    if !state.notifications.is_empty() {
        state.notifications.clear();
        return None;
    }

    // Handle exit confirmation mode
    if state.exit_confirmation_pending {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Some(AppCommand::ForceQuit),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                return Some(AppCommand::CancelQuit)
            }
            _ => return None,
        }
    }

    // Handle confirmation mode with arrow key navigation
    if state.input_mode == InputMode::Confirmation {
        let num_options = ConfirmOption::all().len();
        match key.code {
            // Arrow key navigation
            KeyCode::Left => {
                state.confirmation_selected = state.confirmation_selected.saturating_sub(1);
                return None;
            }
            KeyCode::Right => {
                state.confirmation_selected = (state.confirmation_selected + 1).min(num_options - 1);
                return None;
            }
            // Enter confirms the selected option
            KeyCode::Enter => {
                let selected = ConfirmOption::all()[state.confirmation_selected];
                state.confirmation_selected = 0; // Reset for next time
                return match selected {
                    ConfirmOption::Accept => Some(AppCommand::ConfirmYes),
                    ConfirmOption::AlwaysAccept => Some(AppCommand::ConfirmAll),
                    ConfirmOption::Reject => Some(AppCommand::ConfirmNo),
                };
            }
            // Legacy single-key shortcuts still work
            KeyCode::Char('y') | KeyCode::Char('Y') => return Some(AppCommand::ConfirmYes),
            KeyCode::Char('n') | KeyCode::Char('N') => return Some(AppCommand::ConfirmNo),
            KeyCode::Char('a') | KeyCode::Char('A') => return Some(AppCommand::ConfirmAll),
            KeyCode::Esc => return Some(AppCommand::ConfirmDefer),
            _ => return None,
        }
    }

    // Handle command menu navigation
    if state.show_command_menu {
        match key.code {
            KeyCode::Up => {
                let count = crate::ui::command_menu::command_count(state);
                if count > 0 {
                    state.command_menu_selected = state.command_menu_selected.saturating_sub(1);
                }
                return None;
            }
            KeyCode::Down => {
                let count = crate::ui::command_menu::command_count(state);
                if count > 0 {
                    state.command_menu_selected = (state.command_menu_selected + 1) % count;
                }
                return None;
            }
            KeyCode::Tab => {
                // Autocomplete the selected command
                if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                    state.input_buffer = format!("/{}", cmd);
                    state.cursor_position = state.input_buffer.len();
                    state.show_command_menu = false;
                }
                return None;
            }
            KeyCode::Enter => {
                // Execute the selected command
                if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                    state.input_buffer = format!("/{}", cmd);
                    state.cursor_position = state.input_buffer.len();
                    state.show_command_menu = false;
                    let input = state.take_input();
                    return handle_slash_command(&input[1..], state);
                }
                return None;
            }
            KeyCode::Esc => {
                state.show_command_menu = false;
                return None;
            }
            _ => {}
        }
    }

    // Normal input mode
    match key.code {
        KeyCode::Enter => {
            // Alt+Enter or Shift+Enter: add newline
            if key.modifiers.contains(KeyModifiers::ALT) 
                || key.modifiers.contains(KeyModifiers::SHIFT) 
            {
                state.input_buffer.insert(state.cursor_position, '\n');
                state.cursor_position += 1;
                return None;
            }
            
            // Regular Enter: submit
            state.show_command_menu = false;
            
            let input = state.take_input();
            if input.is_empty() {
                return None;
            }

            // Check for slash commands
            if input.starts_with('/') {
                return handle_slash_command(&input[1..], state);
            }

            // Check for bash commands (starts with !)
            if input.starts_with('!') {
                let cmd = input[1..].to_string();
                if !cmd.is_empty() {
                    return Some(AppCommand::RunBashCommand(cmd));
                }
            }

            return Some(AppCommand::SendMessage(input));
        }
        KeyCode::Char(c) => {
            state.handle_char(c);
            // Show command menu when typing /
            if state.input_buffer.starts_with('/') && state.input_buffer.len() <= 10 {
                state.show_command_menu = true;
                state.command_menu_selected = 0;
            } else {
                state.show_command_menu = false;
            }
        }
        KeyCode::Backspace => {
            state.handle_backspace();
            // Update command menu visibility
            if state.input_buffer.starts_with('/') && state.input_buffer.len() <= 10 {
                state.show_command_menu = true;
            } else {
                state.show_command_menu = false;
            }
        }
        KeyCode::Delete => {
            state.handle_delete();
        }
        KeyCode::Left => {
            state.cursor_left();
        }
        KeyCode::Right => {
            state.cursor_right();
        }
        KeyCode::Home => {
            state.cursor_home();
        }
        KeyCode::End => {
            state.cursor_end();
        }
        KeyCode::Up => {
            state.scroll_up(3);
        }
        KeyCode::Down => {
            state.scroll_down(3);
        }
        KeyCode::PageUp => {
            state.scroll_up(10);
        }
        KeyCode::PageDown => {
            state.scroll_down(10);
        }
        KeyCode::Esc => {
            if state.show_command_menu {
                state.show_command_menu = false;
            } else if state.is_running() {
                return Some(AppCommand::Pause);
            }
        }
        _ => {}
    }

    None
}

/// Handle settings modal input
fn handle_settings_modal_input(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    let providers = LlmProvider::all();
    let models = state.llm_provider.models();
    
    if state.settings_editing {
        // In editing mode for text fields (API key, base URL)
        match key.code {
            KeyCode::Esc => {
                // Cancel editing
                state.settings_editing = false;
                state.settings_edit_buffer.clear();
            }
            KeyCode::Enter => {
                // Save the edited value
                match state.settings_field {
                    2 => {
                        // API Key
                        state.llm_api_key = state.settings_edit_buffer.clone();
                    }
                    3 => {
                        // Base URL
                        if state.settings_edit_buffer.is_empty() {
                            state.llm_base_url = None;
                        } else {
                            state.llm_base_url = Some(state.settings_edit_buffer.clone());
                        }
                    }
                    _ => {}
                }
                state.settings_editing = false;
                state.settings_edit_buffer.clear();
            }
            KeyCode::Backspace => {
                state.settings_edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                state.settings_edit_buffer.push(c);
            }
            _ => {}
        }
        return None;
    }
    
    // Normal navigation mode
    match key.code {
        KeyCode::Esc => {
            state.show_settings_modal = false;
            state.settings_field = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings_field > 0 {
                state.settings_field -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.settings_field < SETTINGS_FIELD_COUNT - 1 {
                state.settings_field += 1;
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            match state.settings_field {
                0 => {
                    // Provider - cycle backward
                    if let Some(idx) = providers.iter().position(|p| *p == state.llm_provider) {
                        let new_idx = if idx == 0 { providers.len() - 1 } else { idx - 1 };
                        state.llm_provider = providers[new_idx].clone();
                        // Reset model to first available for new provider
                        let new_models = state.llm_provider.models();
                        state.llm_model = new_models.first().map(|s| s.to_string()).unwrap_or_default();
                    }
                }
                1 => {
                    // Model - cycle backward
                    if let Some(idx) = models.iter().position(|m| *m == state.llm_model) {
                        let new_idx = if idx == 0 { models.len() - 1 } else { idx - 1 };
                        state.llm_model = models[new_idx].to_string();
                    }
                }
                _ => {}
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            match state.settings_field {
                0 => {
                    // Provider - cycle forward
                    if let Some(idx) = providers.iter().position(|p| *p == state.llm_provider) {
                        let new_idx = (idx + 1) % providers.len();
                        state.llm_provider = providers[new_idx].clone();
                        // Reset model to first available for new provider
                        let new_models = state.llm_provider.models();
                        state.llm_model = new_models.first().map(|s| s.to_string()).unwrap_or_default();
                    }
                }
                1 => {
                    // Model - cycle forward
                    if !models.is_empty() {
                        if let Some(idx) = models.iter().position(|m| *m == state.llm_model) {
                            let new_idx = (idx + 1) % models.len();
                            state.llm_model = models[new_idx].to_string();
                        } else {
                            // Current model not in list, select first
                            state.llm_model = models[0].to_string();
                        }
                    }
                }
                _ => {}
            }
        }
        KeyCode::Enter => {
            match state.settings_field {
                0 | 1 => {
                    // Provider/Model fields cycle on Enter too
                    match state.settings_field {
                        0 => {
                            if let Some(idx) = providers.iter().position(|p| *p == state.llm_provider) {
                                let new_idx = (idx + 1) % providers.len();
                                state.llm_provider = providers[new_idx].clone();
                                let new_models = state.llm_provider.models();
                                state.llm_model = new_models.first().map(|s| s.to_string()).unwrap_or_default();
                            }
                        }
                        1 => {
                            if !models.is_empty() {
                                if let Some(idx) = models.iter().position(|m| *m == state.llm_model) {
                                    let new_idx = (idx + 1) % models.len();
                                    state.llm_model = models[new_idx].to_string();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                2 => {
                    // API Key - enter edit mode
                    state.settings_editing = true;
                    state.settings_edit_buffer = state.llm_api_key.clone();
                }
                3 => {
                    // Base URL - enter edit mode
                    state.settings_editing = true;
                    state.settings_edit_buffer = state.llm_base_url.clone().unwrap_or_default();
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

/// Handle slash commands
fn handle_slash_command(command: &str, state: &mut AppState) -> Option<AppCommand> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let cmd = parts.first().map(|s| s.to_lowercase());

    match cmd.as_deref() {
        Some("help") => {
            state.show_help_modal = true;
            None
        }
        Some("new") => Some(AppCommand::NewConversation),
        Some("usage") => {
            state.show_token_modal = true;
            None
        }
        Some("settings") => {
            state.show_settings_modal = true;
            None
        }
        Some("pause") => Some(AppCommand::Pause),
        Some("theme") => {
            if let Some(name) = parts.get(1) {
                state.theme = Theme::by_name(name);
                state.theme_name = name.to_lowercase();
                state.notify(Notification::info(
                    "Theme Changed",
                    format!("Switched to {} theme", name),
                ));
            } else {
                let available = Theme::available().join(", ");
                state.add_message(DisplayMessage::system(format!(
                    "Current theme: {}. Available: {}", state.theme_name, available,
                )));
            }
            None
        }
        Some("confirm") => {
            if let Some(policy) = parts.get(1) {
                match policy.to_lowercase().as_str() {
                    "always" => Some(AppCommand::SetPolicy(ConfirmationPolicy::AlwaysConfirm)),
                    "never" => Some(AppCommand::SetPolicy(ConfirmationPolicy::NeverConfirm)),
                    "risky" => Some(AppCommand::SetPolicy(ConfirmationPolicy::ConfirmRisky)),
                    _ => {
                        // Invalid policy - show modal with options
                        state.show_policy_modal = true;
                        None
                    }
                }
            } else {
                // No argument - show policy modal
                state.show_policy_modal = true;
                None
            }
        }
        Some("exit") | Some("quit") => {
            state.exit_confirmation_pending = true;
            None
        }
        _ => {
            state.add_message(DisplayMessage::error(format!(
                "Unknown command: /{}. Type /help for available commands.",
                command
            )));
            None
        }
    }
}

/// Process a command and return true if should exit
async fn process_command(
    state: &mut AppState,
    client: &AgentServerClient,
    event_stream: &mut Option<EventStream>,
    command: AppCommand,
    args: &Args,
    llm_config: &LLMConfig,
) -> Result<bool> {
    match command {
        AppCommand::SendMessage(message) => {
            // Add user message to display
            state.add_message(DisplayMessage::user(&message));

            // Ensure we have a conversation
            if state.conversation_id.is_none() {
                // Build workspace config
                let workspace_dir = args.workspace.clone()
                    .unwrap_or_else(|| std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| ".".to_string()));

                // Build conversation request with default development tools
                // Convert client-side policy to server-side policy
                let server_policy = match state.confirmation_policy {
                    ConfirmationPolicy::NeverConfirm => ServerConfirmationPolicy::NeverConfirm,
                    ConfirmationPolicy::AlwaysConfirm => ServerConfirmationPolicy::AlwaysConfirm,
                    ConfirmationPolicy::ConfirmRisky => ServerConfirmationPolicy::ConfirmRisky,
                };
                
                let request = StartConversationRequest {
                    agent: AgentConfig::with_default_tools(llm_config.clone()),
                    workspace: LocalWorkspace::new(workspace_dir),
                    initial_message: Some(SendMessageRequest::user(&message).with_run()),
                    conversation_id: None,
                    confirmation_policy: Some(server_policy),
                };

                match client.start_conversation(request).await {
                    Ok(info) => {
                        state.conversation_id = Some(info.id);
                        state.conversation_title = info.title;
                        info!("Started conversation: {}", info.id);

                        // Connect to WebSocket for events
                        let ws_url = client.conversation_websocket_url(info.id);
                        match EventStream::connect(&ws_url).await {
                            Ok(stream) => {
                                *event_stream = Some(stream);
                                info!("Connected to WebSocket");
                            }
                            Err(e) => {
                                error!("Failed to connect WebSocket: {}", e);
                                state.notify(Notification::error(
                                    "WebSocket Error",
                                    "Failed to connect for real-time updates",
                                ));
                            }
                        }

                        // Conversation starts running automatically with initial_message
                        state.start_timer();
                        state.randomize_spinner();
                        state.execution_status = ExecutionStatus::Running;
                    }
                    Err(e) => {
                        error!("Failed to start conversation: {}", e);
                        state.add_message(DisplayMessage::error(format!(
                            "Failed to start conversation: {}",
                            e
                        )));
                        return Ok(false);
                    }
                }
            } else {
                // Existing conversation - send message with run=true
                let conv_id = state.conversation_id.unwrap();

                // Send message with run=true to start processing
                state.start_timer();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
                if let Err(e) = client.send_message(conv_id, &message, true).await {
                    error!("Failed to send message: {}", e);
                    state.add_message(DisplayMessage::error(format!("Failed to send: {}", e)));
                    state.execution_status = ExecutionStatus::Idle;
                    return Ok(false);
                }
            }
        }

        AppCommand::RunBashCommand(cmd) => {
            // Run bash command locally and display output
            state.add_message(DisplayMessage::system(format!("$ {}", cmd)));
            
            // Execute the command
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    if !stdout.is_empty() {
                        // Trim trailing newlines and display
                        let stdout_trimmed = stdout.trim_end();
                        state.add_message(DisplayMessage::system(stdout_trimmed.to_string()));
                    }
                    if !stderr.is_empty() {
                        let stderr_trimmed = stderr.trim_end();
                        state.add_message(DisplayMessage::error(stderr_trimmed.to_string()));
                    }
                    if !output.status.success() {
                        state.add_message(DisplayMessage::system(format!(
                            "Exit code: {}",
                            output.status.code().unwrap_or(-1)
                        )));
                    }
                }
                Err(e) => {
                    state.add_message(DisplayMessage::error(format!("Failed to run command: {}", e)));
                }
            }
        }

        AppCommand::NewConversation => {
            // Disconnect existing stream
            *event_stream = None;
            state.conversation_id = None;
            state.messages.clear();
            state.pending_actions.clear();
            state.execution_status = ExecutionStatus::Idle;
            state.conversation_title = None;
            state.notify(Notification::info("New Conversation", "Starting fresh"));
        }

        AppCommand::Pause => {
            if let Some(conv_id) = state.conversation_id {
                if let Err(e) = client.pause_conversation(conv_id).await {
                    error!("Failed to pause: {}", e);
                    state.notify(Notification::error("Pause Failed", e.to_string()));
                } else {
                    state.execution_status = ExecutionStatus::Paused;
                    state.notify(Notification::info("Paused", "Conversation paused"));
                }
            }
        }

        AppCommand::ConfirmYes => {
            // Accept: tell the server to accept the pending actions
            if let Some(conv_id) = state.conversation_id {
                info!("User accepted pending actions");
                if let Err(e) = client.accept_pending_actions(conv_id).await {
                    error!("Failed to accept actions: {}", e);
                    state.notify(Notification::error("Accept Failed", e.to_string()));
                }
                state.clear_pending_actions();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
            }
        }

        AppCommand::ConfirmNo => {
            // Reject: tell the server to reject the pending actions
            if let Some(conv_id) = state.conversation_id {
                info!("User rejected pending actions - calling reject API");
                if let Err(e) = client
                    .reject_pending_actions(conv_id, Some("User rejected the action"))
                    .await
                {
                    // If reject API fails, try to just run to clear the state
                    warn!("Reject API failed ({}), trying to continue anyway", e);
                }
                state.add_message(DisplayMessage::system("Action rejected"));
                state.clear_pending_actions();
                // Set to idle - the server should handle the rejection
                state.execution_status = ExecutionStatus::Idle;
            }
        }

        AppCommand::ConfirmAll => {
            // Always accept: change policy and accept current pending actions
            state.confirmation_policy = ConfirmationPolicy::NeverConfirm;
            state.notify(Notification::info(
                "Policy Changed",
                "Auto-approving all future actions",
            ));

            // Accept current pending actions
            if let Some(conv_id) = state.conversation_id {
                if let Err(e) = client.accept_pending_actions(conv_id).await {
                    warn!("Failed to accept actions: {}", e);
                }
                state.clear_pending_actions();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
            }
        }

        AppCommand::ConfirmDefer => {
            state.clear_pending_actions();
            state.execution_status = ExecutionStatus::Paused;
            state.notify(Notification::info("Deferred", "Actions deferred, agent paused"));
        }

        AppCommand::SetPolicy(policy) => {
            state.confirmation_policy = policy;
            state.notify(Notification::info(
                "Policy Changed",
                format!("Confirmation policy: {}", policy),
            ));
        }

        AppCommand::Quit => {
            state.exit_confirmation_pending = true;
        }

        AppCommand::ForceQuit => {
            state.should_exit = true;
            return Ok(true);
        }

        AppCommand::CancelQuit => {
            state.exit_confirmation_pending = false;
        }
    }

    Ok(false)
}



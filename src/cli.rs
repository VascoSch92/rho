//! Command-line argument parsing.

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use uuid::Uuid;

use crate::state::LlmProvider;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Green.on_default())
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Red.on_default().effects(Effects::BOLD))
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD));

const HELP_TEMPLATE: &str = "\
{before-help}\
\x1b[1;33m ▄▄▄▄▄▄\x1b[0m
\x1b[1;33m█▀██▀▀▀█▄  █▄\x1b[0m
\x1b[1;33m  ██▄▄▄█▀  ██\x1b[0m\x1b[0m v{version}
\x1b[1;33m  ██▀▀█▄   ████▄ ▄███▄\x1b[0m
\x1b[1;33m▄ ██  ██   ██ ██ ██ ██\x1b[0m
\x1b[1;33m▀██▀  ▀██▀▄██ ██▄▀███▀\x1b[0m

{about}

{usage-heading} {usage}

{all-args}{after-help}";

/// Rho - AI-powered coding assistant
#[derive(Parser, Debug)]
#[command(name = "rho")]
#[command(version)]
#[command(about = "Terminal UI for OpenHands Agent Server")]
#[command(styles = STYLES)]
#[command(help_template = HELP_TEMPLATE)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// All existing TUI args, flattened so `rho --server X` still works
    #[command(flatten)]
    pub tui: Args,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Launch a web server that serves the TUI in a browser via xterm.js
    #[command(version, help_template = HELP_TEMPLATE)]
    Web(WebArgs),

    /// Run a task headlessly (no TUI) — useful for scripting and CI/CD
    #[command(version, help_template = HELP_TEMPLATE)]
    Headless(HeadlessArgs),
}

#[derive(clap::Args, Debug)]
pub struct WebArgs {
    /// Host to bind the web server to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port for the web server
    #[arg(long, default_value_t = 12000)]
    pub port: u16,

    /// Override LLM settings with environment variables (LLM_API_KEY, LLM_BASE_URL, LLM_MODEL)
    #[arg(long)]
    pub override_with_envs: bool,
}

#[derive(clap::Args, Debug)]
pub struct HeadlessArgs {
    /// Task to execute (inline)
    #[arg(short, long, conflicts_with = "file")]
    pub task: Option<String>,

    /// Read task from a file
    #[arg(short, long, conflicts_with = "task")]
    pub file: Option<std::path::PathBuf>,

    /// Output format: JSON Lines for machine consumption
    #[arg(long)]
    pub json: bool,

    /// Timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 0)]
    pub timeout: u64,

    /// Agent Server URL
    #[arg(short, long, default_value = "http://127.0.0.1:8000")]
    pub server: String,

    /// Session API key for authentication
    #[arg(long, env = "OPENHANDS_SESSION_API_KEY")]
    pub session_api_key: Option<String>,

    /// Working directory for the agent
    #[arg(short, long)]
    pub workspace: Option<String>,

    /// Auto-approve all actions (no confirmation prompts)
    #[arg(long)]
    pub auto_approve: bool,

    /// Override LLM settings with environment variables (LLM_API_KEY, LLM_BASE_URL, LLM_MODEL)
    #[arg(long)]
    pub override_with_envs: bool,
}

/// Rho TUI arguments (default mode)
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Agent Server URL
    #[arg(short, long, default_value = "http://127.0.0.1:8000")]
    pub server: String,

    /// Session API key for authentication (can also use OPENHANDS_SESSION_API_KEY env var)
    #[arg(long, env = "OPENHANDS_SESSION_API_KEY")]
    pub session_api_key: Option<String>,

    /// Working directory for the agent
    #[arg(short, long)]
    pub workspace: Option<String>,

    /// Resume an existing conversation
    #[arg(short, long)]
    pub resume: Option<Uuid>,

    /// Permission mode for action confirmation
    #[arg(long, value_enum, default_value_t = crate::state::ConfirmationPolicy::AlwaysConfirm)]
    pub permission_mode: crate::state::ConfirmationPolicy,

    /// Skip exit confirmation
    #[arg(long)]
    pub exit_without_confirmation: bool,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,

    /// Color theme (rho, dracula, catppuccin, tokyonight, solarized, gruvbox, or custom)
    #[arg(long, env = "RHO_THEME")]
    pub theme: Option<String>,

    /// Override LLM settings with environment variables (LLM_API_KEY, LLM_BASE_URL, LLM_MODEL)
    #[arg(long)]
    pub override_with_envs: bool,
}

/// Parse model argument in format "provider/model" or just "model"
pub fn parse_model_arg(model_arg: &str) -> (LlmProvider, String) {
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
        } else if model.contains("gpt")
            || model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("o4")
        {
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

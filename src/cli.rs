//! Command-line argument parsing.

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::Parser;
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
pub struct Args {
    /// Agent Server URL
    #[arg(short, long, default_value = "http://127.0.0.1:8000")]
    pub server: String,

    /// Session API key for authentication (can also use OPENHANDS_SESSION_API_KEY env var)
    #[arg(long, env = "OPENHANDS_SESSION_API_KEY")]
    pub session_api_key: Option<String>,

    /// LLM model name (e.g., "anthropic/claude-sonnet-4-5-20250929", "openai/gpt-4o")
    #[arg(
        short,
        long,
        env = "LLM_MODEL",
        default_value = "anthropic/claude-sonnet-4-5-20250929"
    )]
    pub model: String,

    /// LLM API key (can also use LLM_API_KEY env var)
    #[arg(long, env = "LLM_API_KEY")]
    pub llm_api_key: Option<String>,

    /// LLM base URL (optional, for custom endpoints)
    #[arg(long, env = "LLM_BASE_URL")]
    pub llm_base_url: Option<String>,

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

    /// Color theme
    #[arg(long, env = "RHO_THEME", value_enum, default_value_t = crate::config::theme::ThemeName::Rho)]
    pub theme: crate::config::theme::ThemeName,
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

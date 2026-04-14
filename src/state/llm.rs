//! LLM provider, model lists, and runtime LLM configuration.

/// LLM Provider
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LlmProvider {
    OpenHands,
    #[default]
    Anthropic,
    OpenAI,
    Mistral,
    Google,
    DeepSeek,
    Other(String),
}

impl LlmProvider {
    pub fn display_name(&self) -> &str {
        match self {
            LlmProvider::OpenHands => "OpenHands",
            LlmProvider::Anthropic => "Anthropic",
            LlmProvider::OpenAI => "OpenAI",
            LlmProvider::Mistral => "Mistral",
            LlmProvider::Google => "Google",
            LlmProvider::DeepSeek => "DeepSeek",
            LlmProvider::Other(s) => s,
        }
    }

    /// Provider prefix for the model string (e.g. "anthropic", "openai").
    pub fn provider_prefix(&self) -> &str {
        match self {
            LlmProvider::OpenHands => "openhands",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::OpenAI => "openai",
            LlmProvider::Mistral => "mistral",
            LlmProvider::Google => "google",
            LlmProvider::DeepSeek => "deepseek",
            LlmProvider::Other(s) => s,
        }
    }

    pub fn all() -> Vec<LlmProvider> {
        vec![
            LlmProvider::OpenHands,
            LlmProvider::Anthropic,
            LlmProvider::OpenAI,
            LlmProvider::Mistral,
            LlmProvider::Google,
            LlmProvider::DeepSeek,
        ]
    }

    pub fn models(&self) -> Vec<&'static str> {
        match self {
            LlmProvider::OpenHands => vec![
                "claude-sonnet-4-5-20250929",
                "claude-opus-4-6",
                "gpt-5.2",
                "gpt-5.1",
                "deepseek-chat",
            ],
            LlmProvider::Anthropic => vec![
                "claude-sonnet-4-5-20250929",
                "claude-opus-4-6",
                "claude-sonnet-4-6",
                "claude-3-5-sonnet-20241022",
                "claude-3-opus-20240229",
                "claude-3-haiku-20240307",
            ],
            LlmProvider::OpenAI => vec![
                "gpt-5.2",
                "gpt-5.1",
                "gpt-4o",
                "gpt-4o-mini",
                "o4-mini",
                "o3",
            ],
            LlmProvider::Mistral => vec![
                "devstral-medium-2512",
                "devstral-2512",
                "devstral-small-2507",
            ],
            LlmProvider::Google => vec!["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
            LlmProvider::DeepSeek => vec!["deepseek-chat", "deepseek-reasoner"],
            LlmProvider::Other(_) => vec![],
        }
    }
}

/// LLM provider/model/key configuration (runtime).
#[derive(Debug, Clone)]
pub struct LlmState {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    /// Free-text override; when non-empty it's sent to the server instead of
    /// the preset `model`.
    pub custom_model: String,
    /// Request timeout in seconds for LLM calls.
    pub llm_timeout_seconds: u32,
    /// Maximum input tokens per request, if set.
    pub llm_max_input_tokens: Option<u64>,
    /// Condenser max size (tokens / turns), if set.
    pub condenser_max_size: Option<u64>,
    /// Whether memory condensation is enabled.
    pub memory_condensation: bool,
}

impl Default for LlmState {
    fn default() -> Self {
        let provider = LlmProvider::Anthropic;
        let model = provider
            .models()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            provider,
            model,
            api_key: String::new(),
            base_url: None,
            custom_model: String::new(),
            llm_timeout_seconds: 600,
            llm_max_input_tokens: None,
            condenser_max_size: None,
            memory_condensation: true,
        }
    }
}

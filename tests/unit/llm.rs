use rho::state::llm::{LlmProvider, LlmState};

// ── display_name ────────────────────────────────────────────────────

#[test]
fn display_name_known_providers() {
    assert_eq!(LlmProvider::Anthropic.display_name(), "Anthropic");
    assert_eq!(LlmProvider::OpenAI.display_name(), "OpenAI");
    assert_eq!(LlmProvider::Google.display_name(), "Google");
    assert_eq!(LlmProvider::Mistral.display_name(), "Mistral");
    assert_eq!(LlmProvider::DeepSeek.display_name(), "DeepSeek");
    assert_eq!(LlmProvider::OpenHands.display_name(), "OpenHands");
}

#[test]
fn display_name_other() {
    let p = LlmProvider::Other("custom".into());
    assert_eq!(p.display_name(), "custom");
}

// ── provider_prefix ─────────────────────────────────────────────────

#[test]
fn provider_prefix_known() {
    assert_eq!(LlmProvider::Anthropic.provider_prefix(), "anthropic");
    assert_eq!(LlmProvider::OpenAI.provider_prefix(), "openai");
    assert_eq!(LlmProvider::Google.provider_prefix(), "google");
}

#[test]
fn provider_prefix_other() {
    let p = LlmProvider::Other("myprefix".into());
    assert_eq!(p.provider_prefix(), "myprefix");
}

// ── models ──────────────────────────────────────────────────────────

#[test]
fn anthropic_has_models() {
    let models = LlmProvider::Anthropic.models();
    assert!(!models.is_empty());
    assert!(models.iter().any(|m| m.contains("claude")));
}

#[test]
fn openai_has_models() {
    let models = LlmProvider::OpenAI.models();
    assert!(!models.is_empty());
    assert!(models.iter().any(|m| m.contains("gpt")));
}

#[test]
fn other_provider_has_no_models() {
    let models = LlmProvider::Other("x".into()).models();
    assert!(models.is_empty());
}

// ── all ─────────────────────────────────────────────────────────────

#[test]
fn all_excludes_other() {
    let all = LlmProvider::all();
    assert!(all.len() >= 5);
    assert!(!all.iter().any(|p| matches!(p, LlmProvider::Other(_))));
}

// ── Default ─────────────────────────────────────────────────────────

#[test]
fn default_provider_is_anthropic() {
    assert_eq!(LlmProvider::default(), LlmProvider::Anthropic);
}

#[test]
fn llm_state_default_has_anthropic_model() {
    let state = LlmState::default();
    assert_eq!(state.provider, LlmProvider::Anthropic);
    assert!(!state.model.is_empty());
    assert!(state.api_key.is_empty());
    assert!(state.base_url.is_none());
}

use rho::state::metrics::MetricsState;
use serde_json::json;

// ── Direct format ───────────────────────────────────────────────────

#[test]
fn parse_direct_cost() {
    let mut m = MetricsState::default();
    m.parse(&json!({ "accumulated_cost": 1.23 }));
    assert!((m.total_cost - 1.23).abs() < f64::EPSILON);
}

#[test]
fn parse_direct_usage() {
    let mut m = MetricsState::default();
    m.parse(&json!({
        "accumulated_cost": 0.5,
        "accumulated_token_usage": {
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "cache_read_tokens": 10,
            "cache_write_tokens": 5,
            "reasoning_tokens": 3,
            "per_turn_token": 42,
            "context_window": 200000
        }
    }));
    assert_eq!(m.total_tokens, 150);
    assert_eq!(m.prompt_tokens, 100);
    assert_eq!(m.completion_tokens, 50);
    assert_eq!(m.cache_read_tokens, 10);
    assert_eq!(m.cache_write_tokens, 5);
    assert_eq!(m.reasoning_tokens, 3);
    assert_eq!(m.per_turn_tokens, 42);
    assert_eq!(m.context_window, 200000);
}

// ── usage_to_metrics format ─────────────────────────────────────────

#[test]
fn parse_usage_to_metrics_single() {
    let mut m = MetricsState::default();
    m.parse(&json!({
        "usage_to_metrics": {
            "rho": {
                "accumulated_cost": 0.05,
                "accumulated_token_usage": {
                    "prompt_tokens": 200,
                    "completion_tokens": 100
                }
            }
        }
    }));
    assert!((m.total_cost - 0.05).abs() < f64::EPSILON);
    assert_eq!(m.total_tokens, 300);
}

#[test]
fn parse_usage_to_metrics_multiple_accumulates() {
    let mut m = MetricsState::default();
    m.parse(&json!({
        "usage_to_metrics": {
            "llm_a": {
                "accumulated_cost": 0.10,
                "accumulated_token_usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 50
                }
            },
            "llm_b": {
                "accumulated_cost": 0.20,
                "accumulated_token_usage": {
                    "prompt_tokens": 200,
                    "completion_tokens": 75
                }
            }
        }
    }));
    assert!((m.total_cost - 0.30).abs() < f64::EPSILON);
    assert_eq!(m.prompt_tokens, 300);
    assert_eq!(m.completion_tokens, 125);
    assert_eq!(m.total_tokens, 425);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn parse_empty_object() {
    let mut m = MetricsState::default();
    m.parse(&json!({}));
    assert_eq!(m.total_tokens, 0);
    assert!((m.total_cost - 0.0).abs() < f64::EPSILON);
}

#[test]
fn parse_missing_fields_no_panic() {
    let mut m = MetricsState::default();
    m.parse(&json!({
        "accumulated_token_usage": {
            "prompt_tokens": 10
        }
    }));
    assert_eq!(m.prompt_tokens, 10);
    assert_eq!(m.completion_tokens, 0);
    assert_eq!(m.total_tokens, 10);
}

#[test]
fn context_window_zero_not_stored() {
    let mut m = MetricsState {
        context_window: 128000,
        ..MetricsState::default()
    };
    m.parse(&json!({
        "accumulated_token_usage": {
            "context_window": 0
        }
    }));
    // Should keep previous value because 0 is skipped
    assert_eq!(m.context_window, 128000);
}

#[test]
fn usage_to_metrics_resets_before_accumulate() {
    let mut m = MetricsState {
        prompt_tokens: 999,
        ..MetricsState::default()
    };
    m.parse(&json!({
        "usage_to_metrics": {
            "x": {
                "accumulated_token_usage": {
                    "prompt_tokens": 10
                }
            }
        }
    }));
    // Should have been reset to 0 then accumulated to 10
    assert_eq!(m.prompt_tokens, 10);
}

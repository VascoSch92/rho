//! Token and cost metrics tracking.

use std::time::Instant;

/// Token and cost metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsState {
    pub elapsed_seconds: u64,
    pub elapsed_base: u64,
    pub start_time: Option<Instant>,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    pub per_turn_tokens: u64,
    pub total_cost: f64,
    pub context_window: u64,
}

impl MetricsState {
    /// Parse metrics from a JSON value.
    ///
    /// Supports multiple formats including:
    /// - `{"usage_to_metrics": {"usage_id": {"accumulated_cost": 0.01, ...}}}`
    /// - `{"accumulated_cost": 0.01, "accumulated_token_usage": {...}}`
    pub fn parse(&mut self, value: &serde_json::Value) {
        if let Some(usage_map) = value.get("usage_to_metrics").and_then(|v| v.as_object()) {
            // Reset and accumulate across all usage entries
            self.total_cost = 0.0;
            self.prompt_tokens = 0;
            self.completion_tokens = 0;
            self.cache_read_tokens = 0;
            self.cache_write_tokens = 0;
            self.reasoning_tokens = 0;
            self.per_turn_tokens = 0;

            for (_usage_id, m) in usage_map {
                if let Some(c) = m.get("accumulated_cost").and_then(|v| v.as_f64()) {
                    self.total_cost += c;
                }
                if let Some(usage) = m.get("accumulated_token_usage") {
                    self.accumulate_usage(usage);
                }
            }
            self.total_tokens = self.prompt_tokens + self.completion_tokens;
            self.log_if_nonzero();
            return;
        }

        // Direct format
        if let Some(cost) = value.get("accumulated_cost").and_then(|v| v.as_f64()) {
            self.total_cost = cost;
        }
        if let Some(usage) = value.get("accumulated_token_usage") {
            self.accumulate_usage(usage);
            self.total_tokens = self.prompt_tokens + self.completion_tokens;
            self.log_if_nonzero();
        }
    }

    /// Add token counts from a usage JSON object.
    fn accumulate_usage(&mut self, usage: &serde_json::Value) {
        if let Some(v) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
            self.prompt_tokens += v;
        }
        if let Some(v) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
            self.completion_tokens += v;
        }
        if let Some(v) = usage.get("cache_read_tokens").and_then(|v| v.as_u64()) {
            self.cache_read_tokens += v;
        }
        if let Some(v) = usage.get("cache_write_tokens").and_then(|v| v.as_u64()) {
            self.cache_write_tokens += v;
        }
        if let Some(v) = usage.get("reasoning_tokens").and_then(|v| v.as_u64()) {
            self.reasoning_tokens += v;
        }
        if let Some(v) = usage.get("per_turn_token").and_then(|v| v.as_u64()) {
            self.per_turn_tokens = v;
        }
        if let Some(ctx) = usage.get("context_window").and_then(|v| v.as_u64()) {
            if ctx > 0 {
                self.context_window = ctx;
            }
        }
    }

    fn log_if_nonzero(&self) {
        if self.total_tokens > 0 || self.total_cost > 0.0 {
            tracing::info!(
                "Updated metrics: tokens={} (prompt={}, completion={}), cost={}, context={}",
                self.total_tokens,
                self.prompt_tokens,
                self.completion_tokens,
                self.total_cost,
                self.context_window
            );
        }
    }
}

//! Shared formatting helpers for tokens, cost, duration, and paths.

/// Format a token count as a human-readable string (e.g. "1.2k", "3.5M").
pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Format a token count with more precision (for modal detail view).
pub fn format_tokens_detailed(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.2}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Format a cost value as a dollar string.
pub fn format_cost(cost: f64) -> String {
    if cost < 0.001 {
        format!("${:.6}", cost)
    } else if cost < 0.01 {
        format!("${:.4}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

/// Format a duration in seconds as "Xm Ys" or "Ys".
pub fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Shorten a path to its last two components: `/a/b/c/d` → `.../c/d`
pub fn truncate_path(path: &str) -> String {
    let parts: Vec<&str> = path.rsplitn(3, '/').collect();
    match parts.len() {
        0 => path.to_string(),
        1 => parts[0].to_string(),
        2 => format!("{}/{}", parts[1], parts[0]),
        _ => format!(".../{}/{}", parts[1], parts[0]),
    }
}

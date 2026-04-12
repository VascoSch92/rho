use rho::ui::formatting::*;

// ── format_tokens ───────────────────────────────────────────────────

#[test]
fn format_tokens_zero() {
    assert_eq!(format_tokens(0), "0");
}

#[test]
fn format_tokens_small() {
    assert_eq!(format_tokens(1), "1");
    assert_eq!(format_tokens(999), "999");
}

#[test]
fn format_tokens_thousands_boundary() {
    assert_eq!(format_tokens(1_000), "1.0k");
    assert_eq!(format_tokens(1_500), "1.5k");
    assert_eq!(format_tokens(999_999), "1000.0k");
}

#[test]
fn format_tokens_millions() {
    assert_eq!(format_tokens(1_000_000), "1.0M");
    assert_eq!(format_tokens(2_500_000), "2.5M");
    assert_eq!(format_tokens(100_000_000), "100.0M");
}

// ── format_tokens_detailed ──────────────────────────────────────────

#[test]
fn format_tokens_detailed_small() {
    assert_eq!(format_tokens_detailed(42), "42");
    assert_eq!(format_tokens_detailed(0), "0");
}

#[test]
fn format_tokens_detailed_thousands() {
    assert_eq!(format_tokens_detailed(1_000), "1.0k");
    assert_eq!(format_tokens_detailed(1_234), "1.2k");
}

#[test]
fn format_tokens_detailed_millions_extra_precision() {
    assert_eq!(format_tokens_detailed(1_000_000), "1.00M");
    assert_eq!(format_tokens_detailed(1_234_567), "1.23M");
}

// ── format_cost ─────────────────────────────────────────────────────

#[test]
fn format_cost_zero() {
    assert_eq!(format_cost(0.0), "$0.0");
}

#[test]
fn format_cost_very_small() {
    assert_eq!(format_cost(0.0001), "$0.0001");
    assert_eq!(format_cost(0.00099), "$0.0010");
}

#[test]
fn format_cost_small() {
    assert_eq!(format_cost(0.005), "$0.005");
    assert_eq!(format_cost(0.0099), "$0.010");
}

#[test]
fn format_cost_normal() {
    assert_eq!(format_cost(0.01), "$0.01");
    assert_eq!(format_cost(1.23), "$1.23");
    assert_eq!(format_cost(99.99), "$99.99");
}

// ── format_duration ─────────────────────────────────────────────────

#[test]
fn format_duration_zero() {
    assert_eq!(format_duration(0), "0s");
}

#[test]
fn format_duration_seconds_only() {
    assert_eq!(format_duration(1), "1s");
    assert_eq!(format_duration(59), "59s");
}

#[test]
fn format_duration_with_minutes() {
    assert_eq!(format_duration(60), "1m 0s");
    assert_eq!(format_duration(61), "1m 1s");
    assert_eq!(format_duration(3661), "61m 1s");
}

// ── selector_prefix ─────────────────────────────────────────────────

#[test]
fn selector_prefix_selected() {
    assert_eq!(selector_prefix(true, ">"), "> ");
    assert_eq!(selector_prefix(true, ">>"), ">> ");
}

#[test]
fn selector_prefix_not_selected() {
    assert_eq!(selector_prefix(false, ">"), "  ");
    assert_eq!(selector_prefix(false, ">>"), "   ");
}

// ── truncate_path ───────────────────────────────────────────────────

#[test]
fn truncate_path_with_home() {
    // This test depends on $HOME being set; skip if not.
    if let Ok(home) = std::env::var("HOME") {
        let p = format!("{}/projects/rho", home);
        assert_eq!(truncate_path(&p), "~/projects/rho");
    }
}

#[test]
fn truncate_path_no_home_match() {
    // Path that doesn't start with $HOME falls back to last two components.
    let p = "/some/deep/nested/path";
    assert_eq!(truncate_path(p), "~/nested/path");
}

#[test]
fn truncate_path_single_component() {
    assert_eq!(truncate_path("foo"), "foo");
}

#[test]
fn truncate_path_two_components() {
    assert_eq!(truncate_path("foo/bar"), "foo/bar");
}

use std::time::Duration;

use rho::state::types::*;

// ── DisplayMessage constructors ─────────────────────────────────────

#[test]
fn user_message_fields() {
    let msg = DisplayMessage::user("hello");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.role, MessageRole::User);
    assert!(!msg.collapsed);
    assert!(msg.id.is_none());
    assert!(msg.tool_name.is_none());
    assert!(msg.security_risk.is_none());
    assert!(!msg.accepted);
    assert!(msg.thought.is_none());
    assert!(msg.activated_skills.is_empty());
}

#[test]
fn assistant_message_fields() {
    let msg = DisplayMessage::assistant("response");
    assert_eq!(msg.content, "response");
    assert_eq!(msg.role, MessageRole::Assistant);
    assert!(!msg.collapsed);
}

#[test]
fn system_message_is_collapsed() {
    let msg = DisplayMessage::system("info");
    assert_eq!(msg.role, MessageRole::System);
    assert!(msg.collapsed);
}

#[test]
fn error_message_fields() {
    let msg = DisplayMessage::error("oops");
    assert_eq!(msg.role, MessageRole::Error);
    assert_eq!(msg.content, "oops");
    assert!(!msg.collapsed);
}

#[test]
fn terminal_message_format() {
    let msg = DisplayMessage::terminal("ls -la", "file1\nfile2");
    assert_eq!(msg.role, MessageRole::Terminal);
    assert!(msg.content.starts_with("$ ls -la\n"));
    assert!(msg.content.contains("file1\nfile2"));
}

#[test]
fn btw_message_format() {
    let msg = DisplayMessage::btw("question?", "answer");
    assert_eq!(msg.role, MessageRole::Btw);
    assert_eq!(msg.content, "question?\nanswer");
}

#[test]
fn user_message_from_string_type() {
    let msg = DisplayMessage::user(String::from("owned"));
    assert_eq!(msg.content, "owned");
}

#[test]
fn empty_content_allowed() {
    let msg = DisplayMessage::user("");
    assert_eq!(msg.content, "");
}

// ── ConfirmationPolicy display ──────────────────────────────────────

#[test]
fn confirmation_policy_display() {
    assert!(format!("{}", ConfirmationPolicy::AlwaysConfirm).contains("Always Confirm"));
    assert!(format!("{}", ConfirmationPolicy::NeverConfirm).contains("Auto-Approve"));
    assert!(format!("{}", ConfirmationPolicy::ConfirmRisky).contains("Confirm Risky"));
}

// ── Notification ────────────────────────────────────────────────────

#[test]
fn notification_info() {
    let n = Notification::info("title", "message");
    assert_eq!(n.title, "title");
    assert_eq!(n.message, "message");
    assert_eq!(n.severity, NotificationSeverity::Info);
}

#[test]
fn notification_warning() {
    let n = Notification::warning("w", "m");
    assert_eq!(n.severity, NotificationSeverity::Warning);
}

#[test]
fn notification_error() {
    let n = Notification::error("e", "m");
    assert_eq!(n.severity, NotificationSeverity::Error);
}

#[test]
fn notification_not_expired_immediately() {
    let n = Notification::info("t", "m");
    assert!(!n.is_expired(Duration::from_secs(5)));
}

#[test]
fn notification_expired_with_zero_duration() {
    let n = Notification::info("t", "m");
    // Sleep a tiny bit to ensure elapsed > 0
    std::thread::sleep(Duration::from_millis(2));
    assert!(n.is_expired(Duration::from_nanos(1)));
}

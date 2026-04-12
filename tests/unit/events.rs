use rho::events::*;

// ── SecurityRisk display ────────────────────────────────────────────

#[test]
fn security_risk_display() {
    assert_eq!(format!("{}", SecurityRisk::Unknown), "UNKNOWN");
    assert_eq!(format!("{}", SecurityRisk::Low), "LOW");
    assert_eq!(format!("{}", SecurityRisk::Medium), "MEDIUM");
    assert_eq!(format!("{}", SecurityRisk::High), "HIGH");
}

#[test]
fn security_risk_default_is_unknown() {
    assert_eq!(SecurityRisk::default(), SecurityRisk::Unknown);
}

// ── ActionEvent::effective_risk ─────────────────────────────────────

fn make_action(top_risk: Option<SecurityRisk>, args_json: Option<&str>) -> ActionEvent {
    ActionEvent {
        base: EventBase {
            id: None,
            timestamp: None,
            source: None,
        },
        tool_call_id: "tc-1".into(),
        tool_name: "terminal".into(),
        action: serde_json::json!({}),
        tool_call: args_json.map(|a| ToolCall {
            id: None,
            name: None,
            arguments: Some(a.to_string()),
        }),
        summary: None,
        thought: None,
        reasoning_content: None,
        security_risk: top_risk,
    }
}

#[test]
fn effective_risk_uses_top_level_when_meaningful() {
    let a = make_action(Some(SecurityRisk::High), None);
    assert_eq!(a.effective_risk(), SecurityRisk::High);
}

#[test]
fn effective_risk_falls_back_to_args() {
    let a = make_action(
        None,
        Some(r#"{"security_risk": "MEDIUM", "command": "ls"}"#),
    );
    assert_eq!(a.effective_risk(), SecurityRisk::Medium);
}

#[test]
fn effective_risk_args_low() {
    let a = make_action(
        Some(SecurityRisk::Unknown),
        Some(r#"{"security_risk": "low"}"#),
    );
    assert_eq!(a.effective_risk(), SecurityRisk::Low);
}

#[test]
fn effective_risk_no_args_returns_unknown() {
    let a = make_action(None, None);
    assert_eq!(a.effective_risk(), SecurityRisk::Unknown);
}

#[test]
fn effective_risk_invalid_args_json() {
    let a = make_action(None, Some("not json"));
    assert_eq!(a.effective_risk(), SecurityRisk::Unknown);
}

#[test]
fn effective_risk_args_missing_field() {
    let a = make_action(None, Some(r#"{"command": "ls"}"#));
    assert_eq!(a.effective_risk(), SecurityRisk::Unknown);
}

#[test]
fn effective_risk_args_unrecognized_value() {
    let a = make_action(None, Some(r#"{"security_risk": "CRITICAL"}"#));
    assert_eq!(a.effective_risk(), SecurityRisk::Unknown);
}

#[test]
fn effective_risk_top_level_takes_precedence() {
    // Top-level is Medium, args say High — top-level wins when meaningful.
    let a = make_action(
        Some(SecurityRisk::Medium),
        Some(r#"{"security_risk": "HIGH"}"#),
    );
    assert_eq!(a.effective_risk(), SecurityRisk::Medium);
}

// ── Event deserialization ───────────────────────────────────────────

#[test]
fn deserialize_message_event() {
    let json = r#"{
        "kind": "MessageEvent",
        "llm_message": {
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello"}]
        }
    }"#;
    let event: Event = serde_json::from_str(json).unwrap();
    assert!(matches!(event, Event::MessageEvent(_)));
    if let Event::MessageEvent(msg) = event {
        assert_eq!(msg.get_text().unwrap(), "Hello");
    }
}

#[test]
fn deserialize_action_event() {
    let json = r#"{
        "kind": "ActionEvent",
        "tool_call_id": "tc-1",
        "tool_name": "terminal",
        "action": {"command": "ls"},
        "security_risk": "LOW"
    }"#;
    let event: Event = serde_json::from_str(json).unwrap();
    assert!(matches!(event, Event::ActionEvent(_)));
    if let Event::ActionEvent(a) = event {
        assert_eq!(a.security_risk, Some(SecurityRisk::Low));
    }
}

#[test]
fn deserialize_unknown_event() {
    let json = r#"{"kind": "FutureEventType", "id": "abc"}"#;
    let event: Event = serde_json::from_str(json).unwrap();
    assert!(matches!(event, Event::Unknown));
}

#[test]
fn deserialize_activated_skills() {
    let json = r#"{
        "kind": "MessageEvent",
        "llm_message": {"role": "user", "content": [{"type": "text", "text": "test"}]},
        "activated_skills": ["uv", "github"]
    }"#;
    let event: Event = serde_json::from_str(json).unwrap();
    if let Event::MessageEvent(msg) = event {
        assert_eq!(msg.activated_skills, vec!["uv", "github"]);
    } else {
        panic!("Expected MessageEvent");
    }
}

#[test]
fn deserialize_empty_activated_skills() {
    let json = r#"{
        "kind": "MessageEvent",
        "llm_message": {"role": "user", "content": [{"type": "text", "text": "hi"}]}
    }"#;
    let event: Event = serde_json::from_str(json).unwrap();
    if let Event::MessageEvent(msg) = event {
        assert!(msg.activated_skills.is_empty());
    }
}

// ── Event::type_name ────────────────────────────────────────────────

#[test]
fn event_type_names() {
    assert_eq!(Event::Unknown.type_name(), "Unknown");

    let json = r#"{"kind":"PauseEvent"}"#;
    let e: Event = serde_json::from_str(json).unwrap();
    assert_eq!(e.type_name(), "Pause");
}

// ── MessageEvent::get_text ──────────────────────────────────────────

#[test]
fn message_event_get_text_none_without_llm_message() {
    let msg = MessageEvent {
        base: EventBase {
            id: None,
            timestamp: None,
            source: None,
        },
        llm_message: None,
        sender: None,
        activated_skills: vec![],
    };
    assert!(msg.get_text().is_none());
}

#[test]
fn message_event_get_text_skips_other_content() {
    let json = r#"{
        "kind": "MessageEvent",
        "llm_message": {
            "role": "assistant",
            "content": [{"type": "image", "url": "x"}, {"type": "text", "text": "found it"}]
        }
    }"#;
    let event: Event = serde_json::from_str(json).unwrap();
    if let Event::MessageEvent(msg) = event {
        assert_eq!(msg.get_text().unwrap(), "found it");
    }
}

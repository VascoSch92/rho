use rho::state::types::{DisplayMessage, InputMode, MessageRole};
use rho::state::AppState;

fn new_state() -> AppState {
    AppState::default()
}

// ── Input handling ──────────────────────────────────────────────────

#[test]
fn handle_char_inserts_at_cursor() {
    let mut s = new_state();
    s.handle_char('a');
    s.handle_char('b');
    assert_eq!(s.input_buffer, "ab");
    assert_eq!(s.cursor_position, 2);
}

#[test]
fn handle_char_inserts_in_middle() {
    let mut s = new_state();
    s.input_buffer = "ac".into();
    s.cursor_position = 1;
    s.handle_char('b');
    assert_eq!(s.input_buffer, "abc");
    assert_eq!(s.cursor_position, 2);
}

#[test]
fn handle_backspace_removes_before_cursor() {
    let mut s = new_state();
    s.input_buffer = "abc".into();
    s.cursor_position = 3;
    s.handle_backspace();
    assert_eq!(s.input_buffer, "ab");
    assert_eq!(s.cursor_position, 2);
}

#[test]
fn handle_backspace_at_start_does_nothing() {
    let mut s = new_state();
    s.input_buffer = "abc".into();
    s.cursor_position = 0;
    s.handle_backspace();
    assert_eq!(s.input_buffer, "abc");
    assert_eq!(s.cursor_position, 0);
}

#[test]
fn handle_delete_removes_at_cursor() {
    let mut s = new_state();
    s.input_buffer = "abc".into();
    s.cursor_position = 1;
    s.handle_delete();
    assert_eq!(s.input_buffer, "ac");
    assert_eq!(s.cursor_position, 1);
}

#[test]
fn handle_delete_at_end_does_nothing() {
    let mut s = new_state();
    s.input_buffer = "abc".into();
    s.cursor_position = 3;
    s.handle_delete();
    assert_eq!(s.input_buffer, "abc");
}

// ── Cursor movement ─────────────────────────────────────────────────

#[test]
fn cursor_left_decrements() {
    let mut s = new_state();
    s.input_buffer = "ab".into();
    s.cursor_position = 2;
    s.cursor_left();
    assert_eq!(s.cursor_position, 1);
}

#[test]
fn cursor_left_at_zero_stays() {
    let mut s = new_state();
    s.cursor_left();
    assert_eq!(s.cursor_position, 0);
}

#[test]
fn cursor_right_increments() {
    let mut s = new_state();
    s.input_buffer = "ab".into();
    s.cursor_position = 0;
    s.cursor_right();
    assert_eq!(s.cursor_position, 1);
}

#[test]
fn cursor_right_at_end_stays() {
    let mut s = new_state();
    s.input_buffer = "ab".into();
    s.cursor_position = 2;
    s.cursor_right();
    assert_eq!(s.cursor_position, 2);
}

#[test]
fn cursor_home_goes_to_zero() {
    let mut s = new_state();
    s.cursor_position = 5;
    s.cursor_home();
    assert_eq!(s.cursor_position, 0);
}

#[test]
fn cursor_end_goes_to_len() {
    let mut s = new_state();
    s.input_buffer = "hello".into();
    s.cursor_position = 0;
    s.cursor_end();
    assert_eq!(s.cursor_position, 5);
}

// ── take_input ──────────────────────────────────────────────────────

#[test]
fn take_input_returns_and_clears() {
    let mut s = new_state();
    s.input_buffer = "hello".into();
    s.cursor_position = 5;
    let taken = s.take_input();
    assert_eq!(taken, "hello");
    assert_eq!(s.input_buffer, "");
    assert_eq!(s.cursor_position, 0);
}

#[test]
fn take_input_empty() {
    let mut s = new_state();
    let taken = s.take_input();
    assert_eq!(taken, "");
}

// ── Scrolling ───────────────────────────────────────────────────────

#[test]
fn scroll_up_adds() {
    let mut s = new_state();
    s.scroll_up(5);
    assert_eq!(s.scroll_offset, 5);
    s.scroll_up(3);
    assert_eq!(s.scroll_offset, 8);
}

#[test]
fn scroll_down_subtracts() {
    let mut s = new_state();
    s.scroll_offset = 10;
    s.scroll_down(3);
    assert_eq!(s.scroll_offset, 7);
}

#[test]
fn scroll_down_saturates_at_zero() {
    let mut s = new_state();
    s.scroll_offset = 2;
    s.scroll_down(100);
    assert_eq!(s.scroll_offset, 0);
}

#[test]
fn scroll_to_bottom_resets() {
    let mut s = new_state();
    s.scroll_offset = 42;
    s.scroll_to_bottom();
    assert_eq!(s.scroll_offset, 0);
}

// ── toggle_all_actions ──────────────────────────────────────────────

#[test]
fn toggle_all_actions_expands_when_any_collapsed() {
    let mut s = new_state();
    let mut msg1 = DisplayMessage::user("u");
    msg1.role = MessageRole::Action;
    msg1.collapsed = true;
    let mut msg2 = DisplayMessage::user("u2");
    msg2.role = MessageRole::Action;
    msg2.collapsed = false;
    s.messages.push_back(msg1);
    s.messages.push_back(msg2);

    s.toggle_all_actions();
    // Any was collapsed → expand all (collapsed = false)
    for msg in &s.messages {
        assert!(!msg.collapsed);
    }
}

#[test]
fn toggle_all_actions_collapses_when_none_collapsed() {
    let mut s = new_state();
    let mut msg = DisplayMessage::user("u");
    msg.role = MessageRole::Action;
    msg.collapsed = false;
    s.messages.push_back(msg);

    s.toggle_all_actions();
    for m in &s.messages {
        assert!(m.collapsed);
    }
}

// ── Spinner ─────────────────────────────────────────────────────────

#[test]
fn spinner_frame_fallback_when_empty() {
    let mut s = new_state();
    s.spinner_frames.clear();
    assert_eq!(s.spinner_frame(), "⠋");
}

#[test]
fn spinner_frame_cycles() {
    let mut s = new_state();
    s.spinner_frames = vec!["a".into(), "b".into(), "c".into()];
    s.spinner_tick = 0;
    assert_eq!(s.spinner_frame(), "a");
    s.spinner_tick = 1;
    assert_eq!(s.spinner_frame(), "b");
    s.spinner_tick = 3;
    assert_eq!(s.spinner_frame(), "a"); // wraps
}

#[test]
fn tick_spinner_wraps() {
    let mut s = new_state();
    s.spinner_tick = usize::MAX;
    s.tick_spinner();
    assert_eq!(s.spinner_tick, 0);
}

// ── Fun facts ───────────────────────────────────────────────────────

#[test]
fn current_fun_fact_fallback_when_empty() {
    let mut s = new_state();
    s.fun_facts.clear();
    assert_eq!(s.current_fun_fact(), "Thinking...");
}

#[test]
fn next_fun_fact_cycles() {
    let mut s = new_state();
    s.fun_facts = vec!["a".into(), "b".into()];
    s.fun_fact_index = 0;
    s.next_fun_fact();
    assert_eq!(s.fun_fact_index, 1);
    s.next_fun_fact();
    assert_eq!(s.fun_fact_index, 0);
}

#[test]
fn next_fun_fact_empty_no_panic() {
    let mut s = new_state();
    s.fun_facts.clear();
    s.next_fun_fact(); // should not panic
}

// ── add_message / MAX_DISPLAY_MESSAGES ──────────────────────────────

#[test]
fn add_message_caps_at_limit() {
    let mut s = new_state();
    for i in 0..1005 {
        s.add_message(DisplayMessage::system(format!("msg {}", i)));
    }
    assert!(s.messages.len() <= 1000);
    // Oldest messages were dropped
    assert!(s.messages.front().unwrap().content != "msg 0");
}

// ── reset_conversation ──────────────────────────────────────────────

#[test]
fn reset_conversation_clears_state() {
    let mut s = new_state();
    s.conversation_id = Some(uuid::Uuid::new_v4());
    s.conversation_title = Some("title".into());
    s.messages.push_back(DisplayMessage::user("hi"));
    s.active_skills = vec!["uv".into()];
    s.input_mode = InputMode::Confirmation;

    s.reset_conversation();

    assert!(s.conversation_id.is_none());
    assert!(s.conversation_title.is_none());
    assert!(s.messages.is_empty());
    assert!(s.active_skills.is_empty());
    assert_eq!(s.input_mode, InputMode::Normal);
}

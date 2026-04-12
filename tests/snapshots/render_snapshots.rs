use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use rho::state::{
    AppState, ConfirmationPolicy, DisplayMessage, MessageRole, Notification, TaskItem,
};
use rho::ui::input::InputWidget;
use rho::ui::messages::MessageListWidget;
use rho::ui::spinner::SpinnerWidget;
use rho::ui::status::BottomStatusBar;
use rho::ui::tasks::TaskListWidget;

/// Render a widget into a buffer and return the content as a plain-text string.
fn render_to_string(widget: impl Widget, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    widget.render(area, &mut buf);
    buffer_to_string(&buf, width, height)
}

/// Convert a ratatui Buffer to a plain-text string, trimming trailing spaces
/// per line but preserving the grid structure.
fn buffer_to_string(buf: &Buffer, width: u16, height: u16) -> String {
    let mut lines = Vec::new();
    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buf[(x, y)];
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn new_state() -> AppState {
    AppState::default()
}

// ── InputWidget ─────────────────────────────────────────────────────

#[test]
fn snapshot_input_empty() {
    let state = new_state();
    let widget = InputWidget::new(&state);
    let output = render_to_string(widget, 60, 3);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_input_with_text() {
    let mut state = new_state();
    state.input_buffer = "hello world".into();
    state.cursor_position = 5;
    let widget = InputWidget::new(&state);
    let output = render_to_string(widget, 60, 3);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_input_bash_mode() {
    let mut state = new_state();
    state.input_buffer = "!ls -la".into();
    state.cursor_position = 7;
    let widget = InputWidget::new(&state);
    let output = render_to_string(widget, 60, 3);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_input_cursor_at_start() {
    let mut state = new_state();
    state.input_buffer = "abc".into();
    state.cursor_position = 0;
    let widget = InputWidget::new(&state);
    let output = render_to_string(widget, 40, 3);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_input_multiline() {
    let mut state = new_state();
    state.input_buffer = "line one\nline two\nline three".into();
    state.cursor_position = 5;
    let widget = InputWidget::new(&state);
    let output = render_to_string(widget, 60, 6);
    insta::assert_snapshot!(output);
}

// ── BottomStatusBar ─────────────────────────────────────────────────

#[test]
fn snapshot_status_bar_default() {
    let state = new_state();
    let widget = BottomStatusBar::new(&state);
    let output = render_to_string(widget, 120, 1);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_status_bar_with_metrics() {
    let mut state = new_state();
    state.metrics.elapsed_seconds = 95;
    state.metrics.prompt_tokens = 12500;
    state.metrics.completion_tokens = 3400;
    state.metrics.total_cost = 0.0567;
    state.metrics.per_turn_tokens = 50000;
    state.metrics.context_window = 200000;
    state.confirmation_policy = ConfirmationPolicy::NeverConfirm;
    let widget = BottomStatusBar::new(&state);
    let output = render_to_string(widget, 140, 1);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_status_bar_high_context() {
    let mut state = new_state();
    state.metrics.per_turn_tokens = 190000;
    state.metrics.context_window = 200000;
    let widget = BottomStatusBar::new(&state);
    let output = render_to_string(widget, 120, 1);
    insta::assert_snapshot!(output);
}

// ── TaskListWidget ──────────────────────────────────────────────────

#[test]
fn snapshot_task_list_empty() {
    let state = new_state();
    let widget = TaskListWidget::new(&state);
    // Empty task list should produce blank output
    let output = render_to_string(widget, 80, 5);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_task_list_with_tasks() {
    let mut state = new_state();
    state.tasks = vec![
        TaskItem {
            title: "Set up project structure".into(),
            notes: "".into(),
            status: "done".into(),
        },
        TaskItem {
            title: "Implement API client".into(),
            notes: "in progress".into(),
            status: "in_progress".into(),
        },
        TaskItem {
            title: "Write tests".into(),
            notes: "".into(),
            status: "todo".into(),
        },
    ];
    state.tasks_visible = true;
    let widget = TaskListWidget::new(&state);
    let output = render_to_string(widget, 80, 6);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_task_list_hidden() {
    let mut state = new_state();
    state.tasks = vec![TaskItem {
        title: "A task".into(),
        notes: "".into(),
        status: "todo".into(),
    }];
    state.tasks_visible = false;
    let widget = TaskListWidget::new(&state);
    let output = render_to_string(widget, 80, 5);
    insta::assert_snapshot!(output);
}

// ── MessageListWidget ───────────────────────────────────────────────

#[test]
fn snapshot_messages_empty() {
    let state = new_state();
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 100, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_user_and_assistant() {
    let mut state = new_state();
    state.add_message(DisplayMessage::user("How do I sort a list in Python?"));
    state.add_message(DisplayMessage::assistant(
        "Use `sorted(my_list)` for a new sorted list, or `my_list.sort()` to sort in place.",
    ));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 100, 25);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_with_activated_skills() {
    let mut state = new_state();
    let mut msg = DisplayMessage::user("set up uv for this project");
    msg.activated_skills = vec!["uv".into(), "python-packaging".into()];
    state.messages.push_back(msg);
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 100, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_error() {
    let mut state = new_state();
    state.add_message(DisplayMessage::error(
        "Connection failed\nServer returned 503",
    ));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_system() {
    let mut state = new_state();
    state.add_message(DisplayMessage::system("Loaded 5 tools"));
    state.add_message(DisplayMessage::system("Conversation paused"));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_terminal() {
    let mut state = new_state();
    state.add_message(DisplayMessage::terminal(
        "cargo build",
        "   Compiling rho v0.1.0\n    Finished in 2.5s",
    ));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_btw() {
    let mut state = new_state();
    state.add_message(DisplayMessage::btw(
        "What version of Python?",
        "Python **3.12.1**",
    ));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_btw_waiting() {
    let mut state = new_state();
    state.add_message(DisplayMessage::btw("What's the CPU?", "Asking agent..."));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_queued() {
    let mut state = new_state();
    state.add_message(DisplayMessage::user("first question"));
    state.message_queue.push_back("second question".into());
    state.message_queue.push_back("third question".into());
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 25);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_action_collapsed() {
    let mut state = new_state();
    let mut action = DisplayMessage::user("");
    action.role = MessageRole::Action;
    action.collapsed = true;
    action.tool_name = Some("terminal".into());
    action.content = "command: ls -la\nList directory contents".into();
    action.id = Some("tc-1".into());
    state.messages.push_back(action);
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_messages_action_expanded() {
    let mut state = new_state();
    let mut action = DisplayMessage::user("");
    action.role = MessageRole::Action;
    action.collapsed = false;
    action.tool_name = Some("terminal".into());
    action.content = "command: cargo test\nRun tests".into();
    action.id = Some("tc-1".into());
    action.security_risk = Some(rho::events::SecurityRisk::Low);
    action.accepted = true;
    state.messages.push_back(action);
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 80, 20);
    insta::assert_snapshot!(output);
}

// ── SpinnerWidget ───────────────────────────────────────────────────

#[test]
fn snapshot_spinner_not_running() {
    let state = new_state();
    let widget = SpinnerWidget::new(&state);
    // Not running → empty
    let output = render_to_string(widget, 60, 1);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_spinner_running() {
    let mut state = new_state();
    state.execution_status = rho::client::ExecutionStatus::Running;
    state.spinner_frames = vec!["⠋".into()];
    state.spinner_tick = 0;
    state.fun_facts = vec!["Thinking hard...".into()];
    state.fun_fact_index = 0;
    let widget = SpinnerWidget::new(&state);
    let output = render_to_string(widget, 60, 1);
    insta::assert_snapshot!(output);
}

// ── NotificationWidget ──────────────────────────────────────────────

#[test]
fn snapshot_notification_info() {
    let mut state = new_state();
    state
        .notifications
        .push(Notification::info("Success", "Skills loaded"));
    let widget = rho::ui::status::NotificationWidget::new(&state);
    let output = render_to_string(widget, 60, 15);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_notification_error() {
    let mut state = new_state();
    state
        .notifications
        .push(Notification::error("Error", "Connection lost"));
    let widget = rho::ui::status::NotificationWidget::new(&state);
    let output = render_to_string(widget, 60, 15);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_notification_none() {
    let state = new_state();
    let widget = rho::ui::status::NotificationWidget::new(&state);
    let output = render_to_string(widget, 60, 15);
    insta::assert_snapshot!(output);
}

// ── Narrow terminal ─────────────────────────────────────────────────

#[test]
fn snapshot_messages_narrow_terminal() {
    let mut state = new_state();
    state.add_message(DisplayMessage::user(
        "A long message that should wrap in a narrow terminal window",
    ));
    state.add_message(DisplayMessage::assistant("Short reply."));
    let widget = MessageListWidget::new(&state);
    let output = render_to_string(widget, 40, 20);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_status_bar_narrow() {
    let state = new_state();
    let widget = BottomStatusBar::new(&state);
    let output = render_to_string(widget, 60, 1);
    insta::assert_snapshot!(output);
}

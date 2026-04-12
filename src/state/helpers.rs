//! AppState helper methods — input, scroll, timer, spinner, notifications.

use std::time::{Duration, Instant};

use crate::client::ExecutionStatus;

use super::types::{DisplayMessage, InputMode, MessageRole, Notification};
use super::AppState;

/// Maximum number of messages to keep in history for display
const MAX_DISPLAY_MESSAGES: usize = 1000;

impl AppState {
    /// Add a message to the conversation
    pub fn add_message(&mut self, message: DisplayMessage) {
        self.messages.push_back(message);
        if self.messages.len() > MAX_DISPLAY_MESSAGES {
            self.messages.pop_front();
        }
        self.scroll_to_bottom();
    }

    // ── Input handling ──────────────────────────────────────────────────

    pub fn handle_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_buffer.remove(self.cursor_position);
        }
    }

    pub fn handle_delete(&mut self) {
        if self.cursor_position < self.input_buffer.len() {
            self.input_buffer.remove(self.cursor_position);
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor_position < self.input_buffer.len() {
            self.cursor_position += 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    pub fn cursor_end(&mut self) {
        self.cursor_position = self.input_buffer.len();
    }

    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input_buffer);
        self.cursor_position = 0;
        input
    }

    // ── Scrolling ───────────────────────────────────────────────────────

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    // ── Actions ─────────────────────────────────────────────────────────

    pub fn toggle_all_actions(&mut self) {
        let any_collapsed = self
            .messages
            .iter()
            .any(|msg| matches!(msg.role, MessageRole::Action) && msg.collapsed);

        let new_state = !any_collapsed;
        for msg in &mut self.messages {
            if matches!(msg.role, MessageRole::Action) {
                msg.collapsed = new_state;
            }
        }
    }

    pub fn clear_pending_actions(&mut self) {
        for pending in &self.pending_actions {
            for msg in self.messages.iter_mut() {
                if msg.role == MessageRole::Action {
                    if let Some(ref msg_id) = msg.id {
                        if msg_id == &pending.tool_call_id {
                            msg.accepted = true;
                            break;
                        }
                    }
                }
            }
        }
        self.pending_actions.clear();
        if self.input_mode == InputMode::Confirmation {
            self.input_mode = InputMode::Normal;
        }
    }

    // ── Notifications ───────────────────────────────────────────────────

    pub fn notify(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn cleanup_notifications(&mut self, max_age: Duration) {
        self.notifications.retain(|n| !n.is_expired(max_age));
    }

    // ── Timer ───────────────────────────────────────────────────────────

    pub fn update_elapsed(&mut self) {
        if let Some(start) = self.metrics.start_time {
            if self.execution_status == ExecutionStatus::Running {
                self.metrics.elapsed_seconds =
                    self.metrics.elapsed_base + start.elapsed().as_secs();
            }
        }
    }

    pub fn start_timer(&mut self) {
        self.metrics.elapsed_base = self.metrics.elapsed_seconds;
        self.metrics.start_time = Some(Instant::now());
    }

    pub fn is_running(&self) -> bool {
        self.execution_status == ExecutionStatus::Running
    }

    // ── Spinner / fun facts ─────────────────────────────────────────────

    pub fn tick_spinner(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
    }

    pub fn next_fun_fact(&mut self) {
        if !self.fun_facts.is_empty() {
            self.fun_fact_index = (self.fun_fact_index + 1) % self.fun_facts.len();
        }
    }

    pub fn randomize_spinner(&mut self) {
        if self.spinner_names.len() > 1 {
            let current_idx = self
                .spinner_names
                .iter()
                .position(|n| *n == self.spinner_style)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % self.spinner_names.len();
            self.spinner_style = self.spinner_names[next_idx].clone();
            self.spinner_frames = self
                .spinners
                .get(&self.spinner_style)
                .cloned()
                .unwrap_or_default();
        }
        self.spinner_tick = 0;
    }

    pub fn spinner_frame(&self) -> &str {
        if self.spinner_frames.is_empty() {
            "⠋"
        } else {
            &self.spinner_frames[self.spinner_tick % self.spinner_frames.len()]
        }
    }

    pub fn current_fun_fact(&self) -> &str {
        if self.fun_facts.is_empty() {
            "Thinking..."
        } else {
            &self.fun_facts[self.fun_fact_index % self.fun_facts.len()]
        }
    }

    // ── Misc ────────────────────────────────────────────────────────────

    pub fn set_workspace(&mut self, path: String) {
        self.workspace_path = path;
    }

    /// Parse metrics from a JSON value (delegates to MetricsState).
    pub fn parse_metrics(&mut self, value: &serde_json::Value) {
        self.metrics.parse(value);
    }

    /// Reset conversation state (shared by NewConversation and ResumeConversation).
    pub fn reset_conversation(&mut self) {
        self.conversation_id = None;
        self.conversation_title = None;
        self.messages.clear();
        self.pending_actions.clear();
        self.execution_status = ExecutionStatus::Idle;
        self.input_mode = InputMode::Normal;
        self.metrics.elapsed_seconds = 0;
        self.metrics.elapsed_base = 0;
        self.metrics.start_time = None;
        self.active_skills.clear();
    }
}

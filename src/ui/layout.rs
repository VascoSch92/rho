//! Main layout and rendering.

use ratatui::{
    layout::{Constraint, Layout},
    Frame,
};

use super::{
    command_menu::CommandMenuWidget,
    confirmation::{ConfirmationPanel, ExitConfirmationModal},
    help_modal::{HelpModal, PolicyModal},
    input::{input_height, InputWidget},
    messages::MessageListWidget,
    settings_modal::SettingsModal,
    startup_modal::StartupModal,
    status::{BottomStatusBar, NotificationWidget, TopStatusBar},
    token_modal::TokenUsageModal,
};
use crate::state::AppState;

/// Main render function for the TUI
pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Calculate dynamic input height based on multiline mode
    let input_h = input_height(state);

    // Main vertical layout: header, messages, input, status bar
    let chunks = Layout::vertical([
        Constraint::Length(1),       // Top status bar (single line with box chars)
        Constraint::Min(8),          // Messages area
        Constraint::Length(input_h), // Input area (dynamic for multiline)
        Constraint::Length(1),       // Bottom status bar
    ])
    .split(area);

    // Render main components
    frame.render_widget(TopStatusBar::new(state), chunks[0]);
    frame.render_widget(MessageListWidget::new(state), chunks[1]);
    frame.render_widget(InputWidget::new(state), chunks[2]);
    frame.render_widget(BottomStatusBar::new(state), chunks[3]);

    // Render overlays
    // Command menu (if typing a slash command)
    if state.show_command_menu {
        frame.render_widget(CommandMenuWidget::new(state), area);
    }

    // Confirmation panel (if there are pending actions)
    if !state.pending_actions.is_empty() {
        frame.render_widget(ConfirmationPanel::new(state), area);
    }

    // Exit confirmation modal
    if state.exit_confirmation_pending {
        frame.render_widget(ExitConfirmationModal { show: true, state }, area);
    }

    // Token usage modal
    if state.show_token_modal {
        frame.render_widget(TokenUsageModal::new(state), area);
    }

    // Help modal
    if state.show_help_modal {
        frame.render_widget(HelpModal::new(state), area);
    }

    // Policy modal
    if state.show_policy_modal {
        frame.render_widget(PolicyModal::new(state), area);
    }

    // Settings modal
    if state.show_settings_modal {
        frame.render_widget(SettingsModal::new(state), area);
    }

    // Startup modal (server initializing)
    if state.server_starting {
        frame.render_widget(StartupModal::new(state), area);
    }

    // Notification modal (centered, same style as other modals)
    if !state.notifications.is_empty() {
        frame.render_widget(NotificationWidget::new(state), area);
    }

    // Note: We don't set terminal cursor position because we render our own
    // visual cursor in the InputWidget. This avoids the blinking terminal cursor.
}

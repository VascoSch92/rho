//! Main layout and rendering.

use ratatui::{
    layout::{Constraint, Layout},
    Frame,
};

use super::{
    command_menu::CommandMenuWidget,
    input::{input_height, InputWidget},
    messages::MessageListWidget,
    modals::{
        ConfirmationPanel, ExitConfirmationModal, HelpModal, PolicyModal, ResumeModal,
        SettingsModal, StartupModal, ThemeModal, TokenUsageModal,
    },
    spinner::{spinner_height, SpinnerWidget},
    status::{BottomStatusBar, NotificationWidget},
};
use crate::state::AppState;

/// Main render function for the TUI
pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Calculate dynamic heights
    let input_h = input_height(state);
    let spinner_h = spinner_height(state);

    // Main vertical layout: messages, spinner, input, status bar
    let chunks = Layout::vertical([
        Constraint::Min(8),            // Messages area
        Constraint::Length(spinner_h), // Spinner (1 when running, 0 otherwise)
        Constraint::Length(input_h),   // Input area
        Constraint::Length(1),         // Bottom status bar
    ])
    .split(area);

    // Render main components
    frame.render_widget(MessageListWidget::new(state), chunks[0]);
    frame.render_widget(SpinnerWidget::new(state), chunks[1]);
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
    if state.settings.show {
        frame.render_widget(SettingsModal::new(state), area);
    }

    // Theme modal
    if state.show_theme_modal {
        frame.render_widget(ThemeModal::new(state), area);
    }

    // Resume modal
    if state.show_resume_modal {
        frame.render_widget(ResumeModal::new(state), area);
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

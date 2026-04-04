//! Settings modal state.

/// Settings modal state.
#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub show: bool,
    pub field: usize,             // 0=Provider, 1=Model, 2=API Key, 3=Base URL
    pub editing: bool,            // Whether currently editing a text field
    pub edit_buffer: String,      // Buffer for editing text fields
    pub dropdown: bool,           // Whether a dropdown list is open
    pub dropdown_selected: usize, // Selected index in the dropdown
}

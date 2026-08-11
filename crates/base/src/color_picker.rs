use gpui::{Hsla, Rgba};

fn parse_hex(value: &str) -> Option<Hsla> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 && value.len() != 8 {
        return None;
    }

    let component = |range| {
        u8::from_str_radix(&value[range], 16)
            .ok()
            .map(|v| v as f32 / 255.0)
    };
    let r = component(0..2)?;
    let g = component(2..4)?;
    let b = component(4..6)?;
    let a = if value.len() == 8 {
        component(6..8)?
    } else {
        1.0
    };
    Some(Rgba { r, g, b, a }.into())
}

/// Presentation-independent state and transitions for a color picker.
///
/// The application owns the palette, text input, sliders, popup, and all
/// presentation. This model keeps the committed color separate from the color
/// currently being previewed while the user edits or hovers.
#[derive(Clone, Debug, Default)]
pub struct ColorPickerState {
    value: Option<Hsla>,
    preview: Option<Hsla>,
    open: bool,
    active_tab: usize,
}

impl ColorPickerState {
    /// Creates an empty, closed picker state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial committed and previewed color.
    pub fn default_value(mut self, value: impl Into<Hsla>) -> Self {
        let value = value.into();
        self.value = Some(value);
        self.preview = Some(value);
        self
    }

    /// Returns the committed color.
    pub fn value(&self) -> Option<Hsla> {
        self.value
    }

    /// Returns the color currently being previewed.
    pub fn preview(&self) -> Option<Hsla> {
        self.preview
    }

    /// Returns the previewed color, falling back to the committed color.
    pub fn displayed_color(&self) -> Option<Hsla> {
        self.preview.or(self.value)
    }

    /// Replaces the committed color and resets the preview to it.
    pub fn set_value(&mut self, value: Option<Hsla>) {
        self.value = value;
        self.preview = value;
    }

    /// Updates only the transient preview color.
    pub fn preview_color(&mut self, value: Hsla) {
        self.preview = Some(value);
    }

    /// Parses a hex color into the transient preview.
    ///
    /// Invalid or incomplete input leaves the current preview unchanged.
    pub fn preview_hex(&mut self, value: &str) -> bool {
        let Some(value) = parse_hex(value) else {
            return false;
        };
        self.preview_color(value);
        true
    }

    /// Parses and commits a hex color, closing the picker on success.
    pub fn commit_hex(&mut self, value: &str) -> Option<Hsla> {
        let value = parse_hex(value)?;
        self.select_color(value);
        Some(value)
    }

    /// Commits a palette color and closes the picker.
    pub fn select_color(&mut self, value: Hsla) {
        self.set_value(Some(value));
        self.open = false;
    }

    /// Commits a color without changing the open state.
    pub fn update_color(&mut self, value: Hsla) {
        self.set_value(Some(value));
    }

    /// Sets whether the picker popup is open.
    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    /// Toggles the picker popup.
    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    /// Returns whether the picker popup is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Selects the application-defined picker panel.
    pub fn set_active_tab(&mut self, tab: usize) {
        self.active_tab = tab;
    }

    /// Returns the selected application-defined picker panel.
    pub fn active_tab(&self) -> usize {
        self.active_tab
    }
}

#[cfg(test)]
mod tests {
    use gpui::hsla;

    use super::ColorPickerState;

    #[test]
    fn default_value_initializes_committed_and_preview_colors() {
        let color = hsla(0.2, 0.3, 0.4, 0.5);
        let state = ColorPickerState::new().default_value(color);

        assert_eq!(state.value(), Some(color));
        assert_eq!(state.preview(), Some(color));
    }

    #[test]
    fn preview_does_not_change_committed_value() {
        let committed = hsla(0.1, 0.2, 0.3, 1.0);
        let preview = hsla(0.6, 0.7, 0.8, 1.0);
        let mut state = ColorPickerState::new().default_value(committed);

        state.preview_color(preview);

        assert_eq!(state.value(), Some(committed));
        assert_eq!(state.displayed_color(), Some(preview));
    }

    #[test]
    fn invalid_hex_does_not_replace_preview_or_commit() {
        let color = hsla(0.1, 0.2, 0.3, 1.0);
        let mut state = ColorPickerState::new().default_value(color);

        assert!(!state.preview_hex("#nope"));
        assert_eq!(state.preview(), Some(color));
        assert_eq!(state.commit_hex("#12"), None);
        assert_eq!(state.value(), Some(color));
    }

    #[test]
    fn committing_hex_updates_value_and_closes_picker() {
        let mut state = ColorPickerState::new();
        state.set_open(true);

        let committed = state.commit_hex("#ff0000").unwrap();

        assert_eq!(state.value(), Some(committed));
        assert_eq!(state.preview(), Some(committed));
        assert!(!state.is_open());
    }

    #[test]
    fn palette_selection_closes_but_slider_update_stays_open() {
        let mut state = ColorPickerState::new();
        state.set_open(true);
        state.update_color(hsla(0.2, 0.3, 0.4, 1.0));
        assert!(state.is_open());

        state.select_color(hsla(0.5, 0.6, 0.7, 1.0));
        assert!(!state.is_open());
    }

    #[test]
    fn open_and_active_panel_are_controlled() {
        let mut state = ColorPickerState::new();
        state.toggle_open();
        state.set_active_tab(1);

        assert!(state.is_open());
        assert_eq!(state.active_tab(), 1);
    }
}

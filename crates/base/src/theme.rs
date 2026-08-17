use gpui::{App, Global};

use crate::{ScrollbarMode, ScrollbarMotion, ScrollbarStyles, SemanticThemeTokens};

/// Application-wide defaults for Base behavior modules.
#[derive(Clone, Default)]
pub struct Theme {
    pub tokens: SemanticThemeTokens,
    pub scrollbar: ScrollbarTheme,
    pub resizable: ResizableTheme,
}

impl Global for Theme {}

impl Theme {
    pub fn global(cx: &App) -> Self {
        cx.try_global::<Self>().cloned().unwrap_or_default()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
        cx.global_mut::<Self>()
    }
}

/// Access to the active base theme through an application context.
pub(crate) trait ActiveTheme {
    fn theme(&self) -> Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> Theme {
        Theme::global(self)
    }
}

/// Global defaults used by [`crate::Scrollbar`].
///
/// `motion` defaults to motionless. Styled layers project their own timing;
/// Base never installs a fade or slide of its own.
#[derive(Clone, Default)]
pub struct ScrollbarTheme {
    mode: ScrollbarMode,
    motion: ScrollbarMotion,
    styles: ScrollbarStyles,
}

impl ScrollbarTheme {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mode(mut self, mode: ScrollbarMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_motion(mut self, motion: ScrollbarMotion) -> Self {
        self.motion = motion;
        self
    }

    pub fn with_styles(mut self, styles: ScrollbarStyles) -> Self {
        self.styles = styles;
        self
    }

    pub fn mode(&self) -> ScrollbarMode {
        self.mode
    }

    pub fn motion(&self) -> ScrollbarMotion {
        self.motion
    }

    pub fn styles(&self) -> &ScrollbarStyles {
        &self.styles
    }
}

/// Global visual defaults used by resizable panel handles.
///
/// The Base default is transparent. Applications and styled façades may
/// project their own colors without coupling resize behavior to a theme crate.
#[derive(Clone, Copy, Default)]
pub struct ResizableTheme {
    pub handle: gpui::Hsla,
    pub active_handle: gpui::Hsla,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_theme_supports_builder_construction() {
        let mode = ScrollbarMode::Hover;
        let motion = ScrollbarMotion::default().with_enter(std::time::Duration::from_millis(120));
        let styles = ScrollbarStyles::default();
        let theme = ScrollbarTheme::new()
            .with_mode(mode)
            .with_motion(motion)
            .with_styles(styles);

        assert_eq!(theme.mode(), mode);
        assert_eq!(theme.motion(), motion);
        let _ = theme.styles();
    }
}

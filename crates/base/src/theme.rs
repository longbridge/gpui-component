use gpui::{App, Global};

use crate::{ScrollbarMode, ScrollbarStyles, SemanticThemeTokens};

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
#[derive(Clone, Default)]
pub struct ScrollbarTheme {
    pub mode: ScrollbarMode,
    pub styles: ScrollbarStyles,
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

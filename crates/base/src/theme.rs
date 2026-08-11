use gpui::{App, Global, Hsla};
use std::sync::Arc;

use crate::{ScrollbarMode, ScrollbarStyles, SemanticThemeTokens};

/// Application-wide defaults for Base behavior modules.
#[derive(Clone, Default)]
pub struct Theme {
    pub tokens: SemanticThemeTokens,
    pub scrollbar: ScrollbarTheme,
    pub resizable: ResizableTheme,
    pub input_editor: InputEditorTheme,
}

/// Presentation tokens consumed by the Base text editor renderer.
#[derive(Clone)]
pub struct InputEditorTheme {
    pub foreground: Hsla,
    pub muted_foreground: Hsla,
    pub background: Hsla,
    pub border: Hsla,
    pub selection: Hsla,
    pub caret: Hsla,
    pub highlight_theme: Arc<crate::highlighter::HighlightTheme>,
}

impl Default for InputEditorTheme {
    fn default() -> Self {
        Self {
            foreground: Hsla::default(),
            muted_foreground: Hsla::default(),
            background: Hsla::default(),
            border: Hsla::default(),
            selection: Hsla::default(),
            caret: Hsla::default(),
            highlight_theme: crate::highlighter::HighlightTheme::default_light(),
        }
    }
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

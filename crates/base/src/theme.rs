use gpui::{App, Global};

use crate::{ScrollbarMode, ScrollbarStyles, SemanticThemeTokens};

/// Application-wide defaults for Base behavior modules.
#[derive(Clone, Default)]
pub struct Theme {
    pub tokens: SemanticThemeTokens,
    pub scrollbar: ScrollbarTheme,
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

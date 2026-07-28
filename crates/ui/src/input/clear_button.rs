use gpui::{App, Styled};

use crate::{
    button::{Button, ButtonVariants as _},
    ActiveTheme as _, Icon, IconName, Sizable as _,
};

#[inline]
pub(crate) fn clear_button(cx: &App) -> Button {
    Button::new("clean")
        .icon(Icon::new(IconName::CircleX))
        .text()
        .xsmall()
        .tab_stop(false)
        .text_color(cx.theme().muted_foreground)
}

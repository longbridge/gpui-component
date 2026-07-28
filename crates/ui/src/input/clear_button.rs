use gpui::App;

use crate::{
    button::{Button, ButtonVariants as _},
    Icon, IconName, Sizable as _,
};

#[inline]
pub(crate) fn clear_button(_: &App) -> Button {
    Button::new("clean")
        .icon(Icon::new(IconName::Close))
        .text()
        .xsmall()
        .tab_stop(false)
}

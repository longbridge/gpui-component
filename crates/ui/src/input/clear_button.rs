use gpui::{App, AppContext, Styled, Window};

use crate::{
    button::{Button, ButtonVariants as _},
    ActiveTheme as _, Icon, IconName, Sizable as _,
};

pub(crate) struct ClearButton {}

impl ClearButton {
    pub fn new(window: &mut Window, cx: &mut App) -> Button {
        Button::new("clean")
            .icon(Icon::new(IconName::CircleX).text_color(cx.theme().muted_foreground))
            .ghost()
            .xsmall()
    }
}

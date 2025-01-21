use gpui::{Styled, WindowContext};

use crate::{
    button::{Button, ButtonVariants as _},
    ActiveTheme as _, Icon, IconName, Sizable as _,
};

pub(crate) struct ClearButton {}

impl ClearButton {
    pub fn new(cx: &mut WindowContext) -> Button {
        Button::new("clean")
            .icon(Icon::new(IconName::CircleX).text_color(cx.theme().muted_foreground))
            .ghost()
            .xsmall()
    }
}

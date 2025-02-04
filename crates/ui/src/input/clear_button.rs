use gpui::{
    App, InteractiveElement, IntoElement, RenderOnce, StatefulInteractiveElement, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants as _},
    ActiveTheme as _, Disableable, Icon, IconName, Sizable as _,
};

#[derive(IntoElement)]
pub(crate) struct ClearButton {
    base: Button,
}

impl ClearButton {
    pub fn new() -> Self {
        Self {
            base: Button::new("clean")
                .icon(Icon::new(IconName::CircleX))
                .ghost()
                .xsmall(),
        }
    }
}

impl Styled for ClearButton {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}
impl StatefulInteractiveElement for ClearButton {}
impl Disableable for ClearButton {
    fn disabled(self, disabled: bool) -> Self {
        Self {
            base: self.base.disabled(disabled),
        }
    }
}

impl InteractiveElement for ClearButton {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for ClearButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base.text_color(cx.theme().muted_foreground)
    }
}

use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, IntoElement, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, Window,
};

use crate::{button::Button, v_flex, ActiveTheme, StyledExt};

/// An empty‑state container with optional icon, title, description, and action.
#[derive(IntoElement)]
pub struct Empty {
    style: StyleRefinement,
    icon: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    action: Option<Button>,
}

impl Empty {
    /// Create a new empty‑state container.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            icon: None,
            title: None,
            description: None,
            action: None,
        }
    }

    /// Set the icon element.
    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.icon = Some(icon.into_any_element());
        self
    }

    /// Set the title text.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description text.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the action button.
    pub fn action(mut self, action: impl Into<Button>) -> Self {
        self.action = Some(action.into());
        self
    }
}

impl Styled for Empty {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Empty {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .refine_style(&self.style)
            .when_some(self.icon, |this, icon| this.child(icon))
            .when_some(self.title, |this, title| {
                this.child(
                    div()
                        .text_xl()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(title),
                )
            })
            .when_some(self.description, |this, desc| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(desc),
                )
            })
            .when_some(self.action, |this, action| this.child(action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_empty_minimal(cx: &mut TestAppContext) {
        cx.update(|_| {
            let el = Empty::new().into_any_element();
            let _ = el;
        });
    }

    #[gpui::test]
    fn test_empty_full(cx: &mut TestAppContext) {
        cx.update(|_| {
            let el = Empty::new()
                .icon(gpui::div())
                .title("Nothing here")
                .description("Try adding some content")
                .action(crate::button::Button::new("add").label("Add Item"))
                .into_any_element();
            let _ = el;
        });
    }
}

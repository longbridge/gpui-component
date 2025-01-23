use crate::{h_flex, v_flex, ActiveTheme, Disableable, IconName, Selectable};
use gpui::{
    div, prelude::FluentBuilder as _, relative, svg, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _,
    WindowContext,
};

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut WindowContext) + 'static>>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            checked: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut WindowContext) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn element_id(&self) -> &ElementId {
        &self.id
    }

    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }
}

impl RenderOnce for Checkbox {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let (color, icon_color) = if self.disabled {
            (
                cx.theme().primary.opacity(0.5),
                cx.theme().primary_foreground.opacity(0.5),
            )
        } else {
            (cx.theme().primary, cx.theme().primary_foreground)
        };

        // wrap a flex to patch for let Checkbox display inline
        div().flex().child(
            h_flex()
                .id(self.id)
                .gap_2()
                .items_center()
                .line_height(relative(1.))
                .child(
                    v_flex()
                        .relative()
                        .border_1()
                        .border_color(color)
                        .rounded_sm()
                        .size_4()
                        .flex_shrink_0()
                        .map(|this| match self.checked {
                            false => this.bg(cx.theme().transparent),
                            _ => this.bg(color),
                        })
                        .child(
                            svg()
                                .absolute()
                                .top_px()
                                .left_px()
                                .size_3()
                                .text_color(icon_color)
                                .map(|this| match self.checked {
                                    true => this.path(IconName::Check.path()),
                                    _ => this,
                                }),
                        ),
                )
                .map(|this| {
                    if let Some(label) = self.label {
                        this.text_color(cx.theme().foreground).child(
                            div()
                                .w_full()
                                .overflow_x_hidden()
                                .text_ellipsis()
                                .line_height(relative(1.))
                                .child(label),
                        )
                    } else {
                        this
                    }
                })
                .when(self.disabled, |this| {
                    this.cursor_not_allowed()
                        .text_color(cx.theme().muted_foreground)
                })
                .when_some(
                    self.on_click.filter(|_| !self.disabled),
                    |this, on_click| {
                        this.on_click(move |_, cx| {
                            let checked = !self.checked;
                            on_click(&checked, cx);
                        })
                    },
                ),
        )
    }
}

use crate::{h_flex, text::Text, v_flex, ActiveTheme, Disableable, IconName, Selectable};
use gpui::{
    div, prelude::FluentBuilder as _, px, relative, svg, AnyElement, App, Div, ElementId,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement as _,
    Styled, Window,
};

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    label: Option<Text>,
    children: Vec<AnyElement>,
    checked: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            label: None,
            children: Vec::new(),
            checked: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
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

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (color, icon_color) = if self.disabled {
            (
                cx.theme().primary.opacity(0.5),
                cx.theme().primary_foreground.opacity(0.5),
            )
        } else {
            (cx.theme().primary, cx.theme().primary_foreground)
        };
        let radius = (cx.theme().radius / 2.).min(px(6.));

        self.base.child(
            h_flex()
                .id(self.id)
                .gap_2()
                .items_start()
                .line_height(relative(1.))
                .text_color(cx.theme().foreground)
                .child(
                    v_flex()
                        .relative()
                        .size_4()
                        .flex_shrink_0()
                        .border_1()
                        .border_color(color)
                        .rounded(radius)
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
                .child(
                    v_flex()
                        .w_full()
                        .line_height(relative(1.2))
                        .gap_1()
                        .map(|this| {
                            if let Some(label) = self.label {
                                this.child(
                                    div()
                                        .size_full()
                                        .text_color(cx.theme().foreground)
                                        .line_height(relative(1.))
                                        .child(label),
                                )
                            } else {
                                this
                            }
                        })
                        .children(self.children),
                )
                .when(self.disabled, |this| {
                    this.cursor_not_allowed()
                        .text_color(cx.theme().muted_foreground)
                })
                .when_some(
                    self.on_click.filter(|_| !self.disabled),
                    |this, on_click| {
                        this.on_click(move |_, window, cx| {
                            let checked = !self.checked;
                            on_click(&checked, window, cx);
                        })
                    },
                ),
        )
    }
}

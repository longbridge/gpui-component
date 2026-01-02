use crate::theme::ActiveTheme;
use crate::{Disableable, Icon, IconName, Sizable, Size, StyledExt};
use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui::{Hsla, SharedString};

/// A simple star Rating element.
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    disabled: bool,
    allow_clear: bool,
    value: usize,
    max: usize,
    color: Option<Hsla>,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl Rating {
    /// Create a new Rating with an `ElementId`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            disabled: false,
            allow_clear: true,
            value: 0,
            max: 5,
            color: None,
            on_click: None,
        }
    }

    /// Set the star size.
    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Disable interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Whether clicking the currently selected star clears the rating.
    pub fn allow_clear(mut self, allow: bool) -> Self {
        self.allow_clear = allow;
        self
    }

    /// Set active color, default will use `yellow` from theme colors.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set initial value (0..=max)
    pub fn value(mut self, value: usize) -> Self {
        self.value = value;
        if self.value > self.max {
            self.value = self.max;
        }
        self
    }

    /// Set maximum number of stars.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        if self.value > self.max {
            self.value = self.max;
        }
        self
    }

    /// Add on_click handler when the rating changes.
    ///
    /// The `&usize` parameter is the new rating value.
    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Styled for Rating {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Rating {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Rating {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Rating {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let allow_clear = self.allow_clear;
        let size = self.size;
        let disabled = self.disabled;
        let max = self.max;
        let value = self.value;
        let active_color = self.color.unwrap_or(cx.theme().yellow);
        let on_click = self.on_click.clone();

        div()
            .id(id)
            .flex()
            .items_center()
            .gap_1()
            .refine_style(&self.style)
            .map(|mut this| {
                for ix in 1..=max {
                    let filled = ix <= value;
                    let group_name: SharedString = format!("rating-item-{}", ix).into();

                    this = this.child(
                        div()
                            .id(ix)
                            .group(group_name.clone())
                            .flex_none()
                            .flex_shrink_0()
                            .when(filled, |this| this.text_color(active_color))
                            .group_hover(group_name, |this| this.text_color(active_color))
                            .child(
                                Icon::new(if filled {
                                    IconName::StarFill
                                } else {
                                    IconName::Star
                                })
                                .with_size(size),
                            )
                            .when(!disabled, |this| {
                                this.on_mouse_down(MouseButton::Left, {
                                    let on_click = on_click.clone();
                                    move |_: &MouseDownEvent, window, cx| {
                                        let new = if value == ix && allow_clear { 0 } else { ix };
                                        if let Some(on_click) = &on_click {
                                            on_click(&new, window, cx);
                                        }
                                    }
                                })
                            }),
                    );
                }

                this
            })
    }
}

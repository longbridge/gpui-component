use crate::theme::ActiveTheme;
use crate::{Disableable, Icon, IconName, Sizable, Size, StyledExt, h_flex};
use gpui_base::TestSupportExt as _;
use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui::{ClickEvent, Hsla, StatefulInteractiveElement};

/// A simple star Rating element.
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    disabled: bool,
    value: usize,
    controlled: bool,
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
            value: 0,
            controlled: false,
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

    /// Set active color, default will use `yellow` from theme colors.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set initial value (0..=max).
    pub fn value(mut self, value: usize) -> Self {
        self.controlled = false;
        self.value = value;
        if self.value > self.max {
            self.value = self.max;
        }
        self
    }

    /// Uses an owner-controlled rating. Activation requests a value through
    /// `on_click`; the displayed value changes only when the owner supplies it.
    /// The ordinary `value` builder retains its existing internal-state behavior.
    pub fn controlled_value(mut self, value: usize) -> Self {
        self.controlled = true;
        self.value = value.min(self.max);
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

struct RaitingState {
    /// To save the default value on init state, to detect external value changes.
    default_value: usize,
    /// To store the current selected value.
    value: usize,
    /// To store the currently hovered value.
    hovered_value: usize,
}

impl RenderOnce for Rating {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let size = self.size;
        let disabled = self.disabled;
        let max = self.max;
        let default_value = self.value;
        let active_color = self.color.unwrap_or(cx.theme().yellow);
        let on_click = self.on_click.clone();

        let state = window.use_keyed_state(id.clone(), cx, |_, _| RaitingState {
            default_value,
            value: default_value,
            hovered_value: 0,
        });

        // Reset state if outside has changed `value` prop.
        if state.read(cx).default_value != default_value {
            state.update(cx, |state, _| {
                state.default_value = default_value;
                state.value = default_value;
            });
        }
        let value = if self.controlled {
            default_value
        } else {
            state.read(cx).value
        };

        h_flex()
            .id(id)
            .test_support()
            .flex_nowrap()
            .refine_style(&self.style)
            .on_hover(window.listener_for(&state, move |state, hovered, _, cx| {
                if !hovered && state.hovered_value != 0 {
                    state.hovered_value = 0;
                    cx.notify();
                }
            }))
            .map(|mut this| {
                for ix in 1..=max {
                    let filled = ix <= value;
                    let hovered = state.read(cx).hovered_value >= ix;

                    this = this.child(
                        div()
                            .id(ix)
                            .test_support()
                            .p_0p5()
                            .flex_none()
                            .flex_shrink_0()
                            .when(filled || hovered, |this| this.text_color(active_color))
                            .child(
                                Icon::new(if filled {
                                    IconName::StarFill
                                } else {
                                    IconName::Star
                                })
                                .with_size(size),
                            )
                            .when(!disabled, |this| {
                                this.on_mouse_move(window.listener_for(
                                    &state,
                                    move |state, _, _, cx| {
                                        if state.hovered_value != ix {
                                            state.hovered_value = ix;
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_click({
                                    let state = state.clone();
                                    let on_click = on_click.clone();
                                    move |_: &ClickEvent, window, cx| {
                                        let new = if value >= ix {
                                            ix.saturating_sub(1)
                                        } else {
                                            ix
                                        };

                                        state.update(cx, |state, cx| {
                                            state.value = new;
                                            cx.notify();
                                        });

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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Modifiers, Render, TestAppContext, point, px};
    use std::cell::RefCell;

    struct RatingHarness {
        controlled: bool,
        disabled: bool,
        changes: Rc<RefCell<Vec<usize>>>,
    }
    impl Render for RatingHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            Rating::new("rating")
                .h(px(40.))
                .w(px(220.))
                .max(7)
                .map(|rating| {
                    if self.controlled {
                        rating.controlled_value(6)
                    } else {
                        rating.value(6)
                    }
                })
                .disabled(self.disabled)
                .on_click(move |next, _, _| changes.borrow_mut().push(*next))
        }
    }
    fn activate_twice(cx: &mut TestAppContext, controlled: bool, disabled: bool) -> Vec<usize> {
        cx.update(crate::init);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let changes = changes.clone();
            move |_, _| RatingHarness {
                controlled,
                disabled,
                changes,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(point(px(10.), px(20.)), Modifiers::default());
        cx.simulate_click(point(px(10.), px(20.)), Modifiers::default());
        let result = changes.borrow().clone();
        result
    }
    #[gpui::test]
    fn controlled_rating_keeps_the_owner_value_until_accepted(cx: &mut TestAppContext) {
        assert_eq!(activate_twice(cx, true, false), vec![0, 0]);
    }
    #[gpui::test]
    fn ordinary_rating_preserves_its_internal_selection(cx: &mut TestAppContext) {
        assert_eq!(activate_twice(cx, false, false), vec![0, 1]);
    }
    #[gpui::test]
    fn disabled_controlled_rating_does_not_request_changes(cx: &mut TestAppContext) {
        assert!(activate_twice(cx, true, true).is_empty());
    }
    #[gpui::test]
    fn test_rating_builder(_cx: &mut TestAppContext) {
        let rating = Rating::new("rating")
            .max(7)
            .controlled_value(9)
            .large()
            .disabled(true);
        assert_eq!(rating.value, 7);
        assert!(rating.controlled);
        assert!(rating.disabled);
    }
}

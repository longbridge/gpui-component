use crate::{
    ActiveTheme, Disableable, Side, Sizable, Size, StyledExt, h_flex, text::Text, tooltip::Tooltip,
};
use gpui::{
    Animation, AnimationExt as _, App, ElementId, InteractiveElement, IntoElement,
    ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};
use std::{rc::Rc, time::Duration};

/// A Switch element that can be toggled on or off.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    label: Option<Text>,
    label_side: Side,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    size: Size,
    tooltip: Option<SharedString>,
}

impl Switch {
    /// Create a new Switch element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id: id.clone(),
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            label: None,
            on_click: None,
            label_side: Side::Right,
            size: Size::Medium,
            tooltip: None,
        }
    }

    /// Set the checked state of the switch.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the label of the switch.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add a click handler for the switch.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set tooltip for the switch.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Switch {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let on_click = self.on_click.clone();
        let toggle_state = window.use_keyed_state(self.id.clone(), cx, |_, _| checked);

        let (bg, toggle_bg) = match checked {
            true => (cx.theme().primary, cx.theme().switch_thumb),
            false => (cx.theme().switch, cx.theme().switch_thumb),
        };

        let (bg, toggle_bg) = if self.disabled {
            (
                if checked { bg.alpha(0.5) } else { bg },
                toggle_bg.alpha(0.35),
            )
        } else {
            (bg, toggle_bg)
        };

        let (bg_width, bg_height) = match self.size {
            Size::XSmall | Size::Small => (px(28.), px(16.)),
            _ => (px(36.), px(20.)),
        };
        let bar_width = match self.size {
            Size::XSmall | Size::Small => px(12.),
            _ => px(16.),
        };
        let inset = px(2.);
        let radius = if cx.theme().radius >= px(4.) {
            bg_height
        } else {
            cx.theme().radius
        };

        div().refine_style(&self.style).child(
            h_flex()
                .id(self.id.clone())
                .gap_2()
                .items_start()
                .when(self.label_side.is_left(), |this| this.flex_row_reverse())
                .child(
                    // Switch Bar
                    div()
                        .id(self.id.clone())
                        .w(bg_width)
                        .h(bg_height)
                        .rounded(radius)
                        .flex()
                        .items_center()
                        .border(inset)
                        .border_color(gpui::transparent_white())
                        .bg(bg)
                        .when_some(self.tooltip.clone(), |this, tooltip| {
                            this.tooltip(move |window, cx| {
                                Tooltip::new(tooltip.clone()).build(window, cx)
                            })
                        })
                        .child(
                            // Switch Toggle
                            div()
                                .rounded(radius)
                                .bg(toggle_bg)
                                .shadow_md()
                                .size(bar_width)
                                .map(|this| {
                                    let prev_checked = toggle_state.read(cx);
                                    if !self.disabled && *prev_checked != checked {
                                        let duration = Duration::from_secs_f64(0.15);
                                        cx.spawn({
                                            let toggle_state = toggle_state.clone();
                                            async move |cx| {
                                                cx.background_executor().timer(duration).await;
                                                _ = toggle_state
                                                    .update(cx, |this, _| *this = checked);
                                            }
                                        })
                                        .detach();

                                        this.with_animation(
                                            ElementId::NamedInteger("move".into(), checked as u64),
                                            Animation::new(duration),
                                            move |this, delta| {
                                                let max_x = bg_width - bar_width - inset * 2;
                                                let x = if checked {
                                                    max_x * delta
                                                } else {
                                                    max_x - max_x * delta
                                                };
                                                this.left(x)
                                            },
                                        )
                                        .into_any_element()
                                    } else {
                                        let max_x = bg_width - bar_width - inset * 2;
                                        let x = if checked { max_x } else { px(0.) };
                                        this.left(x).into_any_element()
                                    }
                                }),
                        ),
                )
                .when_some(self.label, |this, label| {
                    this.child(div().line_height(bg_height).child(label).map(
                        |this| match self.size {
                            Size::XSmall | Size::Small => this.text_sm(),
                            _ => this.text_base(),
                        },
                    ))
                })
                .when_some(
                    on_click
                        .as_ref()
                        .map(|c| c.clone())
                        .filter(|_| !self.disabled),
                    |this, on_click| {
                        let toggle_state = toggle_state.clone();
                        this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            _ = toggle_state.update(cx, |this, _| *this = checked);
                            on_click(&!checked, window, cx);
                        })
                    },
                ),
        )
    }
}

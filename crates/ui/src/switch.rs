use crate::{
    h_flex, text::Text, tooltip::Tooltip, ActiveTheme, Colorize, Disableable, Side, Sizable, Size,
};
use gpui::{
    div, prelude::FluentBuilder as _, px, Animation, AnimationExt as _, AnyElement, App, Div,
    Element, ElementId, GlobalElementId, InteractiveElement, IntoElement, LayoutId,
    ParentElement as _, SharedString, StatefulInteractiveElement, Styled, Window,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

/// A Switch element that can be toggled on or off.
pub struct Switch {
    id: ElementId,
    base: Div,
    checked: bool,
    disabled: bool,
    label: Option<Text>,
    label_side: Side,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    size: Size,
    tooltip: Option<SharedString>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id: id.clone(),
            base: div(),
            checked: false,
            disabled: false,
            label: None,
            on_click: None,
            label_side: Side::Right,
            size: Size::Medium,
            tooltip: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn label_side(mut self, label_side: Side) -> Self {
        self.label_side = label_side;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
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

impl IntoElement for Switch {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct SwitchState {
    prev_checked: Rc<RefCell<Option<bool>>>,
}

impl Element for Switch {
    type RequestLayoutState = AnyElement;

    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state::<SwitchState, _>(global_id.unwrap(), move |state, window| {
            let state = state.unwrap_or_default();
            let checked = self.checked;
            let on_click = self.on_click.clone();
            let style = self.base.style();

            let (bg, toggle_bg) = match self.checked {
                true => (cx.theme().primary, cx.theme().background),
                false => (cx.theme().switch, cx.theme().background),
            };

            let (bg, toggle_bg) = match self.disabled {
                true => {
                    if self.checked {
                        (cx.theme().muted.darken(0.05), toggle_bg.opacity(0.8))
                    } else {
                        (cx.theme().muted, toggle_bg.opacity(0.8))
                    }
                }
                false => (bg, toggle_bg),
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

            let mut root = div();
            *root.style() = style.clone();

            let mut element = root
                .child(
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
                                .border_color(cx.theme().transparent)
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
                                            let prev_checked = state.prev_checked.clone();
                                            if !self.disabled
                                                && prev_checked
                                                    .borrow()
                                                    .map_or(false, |prev| prev != checked)
                                            {
                                                let dur = Duration::from_secs_f64(0.15);
                                                cx.spawn(async move |cx| {
                                                    cx.background_executor().timer(dur).await;

                                                    *prev_checked.borrow_mut() = Some(checked);
                                                })
                                                .detach();
                                                this.with_animation(
                                                    ElementId::NamedInteger(
                                                        "move".into(),
                                                        checked as u64,
                                                    ),
                                                    Animation::new(dur),
                                                    move |this, delta| {
                                                        let max_x =
                                                            bg_width - bar_width - inset * 2;
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
                        .when_some(self.label.take(), |this, label| {
                            this.child(div().line_height(bg_height).child(label).map(|this| {
                                match self.size {
                                    Size::XSmall | Size::Small => this.text_sm(),
                                    _ => this.text_base(),
                                }
                            }))
                        })
                        .when_some(
                            on_click
                                .as_ref()
                                .map(|c| c.clone())
                                .filter(|_| !self.disabled),
                            |this, on_click| {
                                let prev_checked = state.prev_checked.clone();
                                this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                                    cx.stop_propagation();
                                    *prev_checked.borrow_mut() = Some(checked);
                                    on_click(&!checked, window, cx);
                                })
                            },
                        ),
                )
                .into_any_element();

            ((element.request_layout(window, cx), element), state)
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx)
    }
}

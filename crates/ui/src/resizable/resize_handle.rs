use std::{cell::RefCell, rc::Rc};

use gpui::{
    deferred, div, prelude::FluentBuilder as _, px, AnyElement, App, Axis, Element, ElementId,
    Entity, GlobalElementId, InteractiveElement, IntoElement, MouseDownEvent, MouseUpEvent,
    ParentElement as _, Pixels, Point, Render, StatefulInteractiveElement, Styled as _, Window,
};

use crate::{dock::DockPlacement, ActiveTheme as _, AxisExt as _, ContextModal};

pub(crate) const HANDLE_SIZE: Pixels = px(5.);

/// Create a resize handle for a resizable panel.
pub(crate) fn resize_handle<T: 'static, E: 'static + Render>(
    id: impl Into<ElementId>,
    axis: Axis,
) -> ResizeHandle<T, E> {
    ResizeHandle::new(id, axis)
}

pub(crate) struct ResizeHandle<T: 'static, E: 'static + Render> {
    id: ElementId,
    axis: Axis,
    drag_value: Option<Rc<T>>,
    placement: Option<DockPlacement>,
    on_drag: Option<Rc<dyn Fn(&Point<Pixels>, &mut Window, &mut App) -> Entity<E>>>,
}

impl<T: 'static, E: 'static + Render> ResizeHandle<T, E> {
    fn new(id: impl Into<ElementId>, axis: Axis) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            on_drag: None,
            drag_value: None,
            placement: None,
            axis,
        }
    }

    pub(crate) fn on_drag(
        mut self,
        value: T,
        f: impl Fn(Rc<T>, &Point<Pixels>, &mut Window, &mut App) -> Entity<E> + 'static,
    ) -> Self {
        let value = Rc::new(value);
        self.drag_value = Some(value.clone());
        self.on_drag = Some(Rc::new(move |p, window, cx| {
            f(value.clone(), p, window, cx)
        }));
        self
    }

    pub(crate) fn placement(mut self, placement: DockPlacement) -> Self {
        self.placement = Some(placement);
        self
    }
}

#[derive(Default, Debug, Clone)]
struct ResizeHandleState {
    active: Rc<RefCell<bool>>,
}

impl ResizeHandleState {
    fn set_active(&self, active: bool) {
        *self.active.borrow_mut() = active;
    }

    fn is_active(&self) -> bool {
        *self.active.borrow()
    }
}

impl<T: 'static, E: 'static + Render> IntoElement for ResizeHandle<T, E> {
    type Element = ResizeHandle<T, E>;
    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct RequestLayoutState {
    handle: AnyElement,
    resizable: bool,
}

impl<T: 'static, E: 'static + Render> Element for ResizeHandle<T, E> {
    type RequestLayoutState = RequestLayoutState;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let neg_offset = -(HANDLE_SIZE - px(1.)) / 2.;
        let resizable = !(window.has_active_modal(cx) || window.has_active_drawer(cx));
        let axis = self.axis;

        window.with_element_state(id.unwrap(), |state, window| {
            let state = state.unwrap_or(ResizeHandleState::default());

            let border_color = if state.is_active() {
                cx.theme().drag_border
            } else {
                cx.theme().border
            };

            let mut el = div()
                .id("handle")
                .group("handle")
                .absolute()
                .flex_shrink_0()
                .when(axis.is_horizontal(), |this| {
                    this.top_0().left_0().h_full().w(HANDLE_SIZE)
                })
                .when(axis.is_vertical(), |this| {
                    this.top_0().left_0().w_full().h(HANDLE_SIZE)
                })
                .map(|this| match self.placement {
                    Some(DockPlacement::Left) => this.left_auto().right_0(),
                    _ => this,
                })
                .child(
                    div()
                        .relative()
                        .size_full()
                        .map(|this| match self.placement {
                            Some(DockPlacement::Left) => {
                                this.border_color(border_color).border_r_1()
                            }
                            Some(DockPlacement::Right) => {
                                this.border_color(border_color).border_l_1()
                            }
                            Some(DockPlacement::Bottom) => {
                                this.border_color(border_color).border_t_1()
                            }
                            _ => this.child(
                                div()
                                    .absolute()
                                    .bg(border_color)
                                    .when(axis.is_horizontal(), |this| {
                                        this.h_full().left_0().w(px(1.))
                                    })
                                    .when(axis.is_vertical(), |this| {
                                        this.w_full().top_0().h(px(1.))
                                    }),
                            ),
                        })
                        .when(resizable, |this| {
                            this.child(deferred(
                                div()
                                    .id("handle-dragger")
                                    .group("handle")
                                    .absolute()
                                    .occlude()
                                    .flex_shrink_0()
                                    .when_some(self.on_drag.clone(), |this, on_drag| {
                                        let Some(value) = self.drag_value.clone() else {
                                            return this;
                                        };

                                        this.on_drag(value, move |_, position, window, cx| {
                                            on_drag(&position, window, cx)
                                        })
                                    })
                                    .when(axis.is_horizontal(), |this| {
                                        this.cursor_col_resize()
                                            .top_0()
                                            .left(neg_offset)
                                            .h_full()
                                            .w(HANDLE_SIZE)
                                    })
                                    .when(axis.is_vertical(), |this| {
                                        this.cursor_row_resize()
                                            .left_0()
                                            .top(neg_offset)
                                            .w_full()
                                            .h(HANDLE_SIZE)
                                    })
                                    .map(|this| match self.placement {
                                        Some(DockPlacement::Left) => {
                                            this.left_auto().right(neg_offset)
                                        }
                                        _ => this,
                                    }),
                            ))
                        }),
                )
                .into_any_element();

            let layout_id = el.request_layout(window, cx);

            (
                (
                    layout_id,
                    RequestLayoutState {
                        handle: el,
                        resizable,
                    },
                ),
                state,
            )
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.handle.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.handle.paint(window, cx);

        if !request_layout.resizable {
            return;
        }

        let handle_size = HANDLE_SIZE - px(1.);
        let pos_offset = match self.placement {
            Some(DockPlacement::Left) => Point::new(handle_size / 2., px(0.)),
            _ => {
                if self.axis.is_horizontal() {
                    Point::new(-handle_size / 2., px(0.))
                } else {
                    Point::new(px(0.), -handle_size / 2.)
                }
            }
        };
        let mut bounds = bounds;
        bounds.origin = bounds.origin + pos_offset;

        window.with_element_state(id.unwrap(), |state: Option<ResizeHandleState>, window| {
            let state = state.unwrap_or(ResizeHandleState::default());

            window.on_mouse_event({
                let state = state.clone();
                move |ev: &MouseDownEvent, phase, window, _| {
                    if bounds.contains(&ev.position) && phase.bubble() {
                        state.set_active(true);
                        window.refresh();
                    }
                }
            });

            window.on_mouse_event({
                let state = state.clone();
                move |_: &MouseUpEvent, _, window, _| {
                    if state.is_active() {
                        state.set_active(false);
                        window.refresh();
                    }
                }
            });

            ((), state)
        });
    }
}

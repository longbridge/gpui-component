use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Axis, Element, ElementId, Entity, GlobalElementId, InteractiveElement,
    IntoElement, MouseDownEvent, MouseUpEvent, ParentElement as _, Pixels, Point, Render,
    StatefulInteractiveElement, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{AxisExt as _, Side, theme::ActiveTheme as _};

pub(crate) const HANDLE_PADDING: Pixels = px(4.);
pub(crate) const HANDLE_SIZE: Pixels = px(1.);

/// Create a resize handle for a resizable panel.
#[doc(hidden)]
pub fn resize_handle<T: 'static, E: 'static + Render>(
    id: impl Into<ElementId>,
    axis: Axis,
) -> ResizeHandle<T, E> {
    ResizeHandle::new(id, axis)
}

/// Draws the visible part of a resize handle.
///
/// Returning `None` keeps the built-in line, so a renderer can override some
/// handles and leave the rest alone.
pub type ResizeHandleRenderer =
    Rc<dyn Fn(&ResizeHandleContext, &mut Window, &mut App) -> Option<AnyElement>>;

/// What a [`ResizeHandleRenderer`] is told about the handle it is drawing.
///
/// The hit area, the cursor and the drag itself stay with the handle; a
/// renderer only supplies what is painted inside it.
pub struct ResizeHandleContext {
    axis: Axis,
    active: bool,
}

impl ResizeHandleContext {
    /// The axis the handle resizes along: `Horizontal` for a vertical divider
    /// between two side-by-side panels.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// Whether this handle is the one being dragged right now.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[doc(hidden)]
pub struct ResizeHandle<T: 'static, E: 'static + Render> {
    id: ElementId,
    axis: Axis,
    drag_value: Option<Rc<T>>,
    placement: Option<Side>,
    on_drag: Option<Rc<dyn Fn(&Point<Pixels>, &mut Window, &mut App) -> Entity<E>>>,
    appearance: Option<ResizeHandleRenderer>,
}

impl<T: 'static, E: 'static + Render> ResizeHandle<T, E> {
    fn new(id: impl Into<ElementId>, axis: Axis) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            on_drag: None,
            drag_value: None,
            placement: None,
            appearance: None,
            axis,
        }
    }

    /// Hand the painted part of this handle to `appearance`.
    pub fn with_appearance(mut self, appearance: ResizeHandleRenderer) -> Self {
        self.appearance = Some(appearance);
        self
    }

    pub fn on_drag(
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

    pub fn placement(mut self, placement: Side) -> Self {
        self.placement = Some(placement);
        self
    }
}

#[derive(Default, Debug, Clone)]
struct ResizeHandleState {
    active: Cell<bool>,
}

impl ResizeHandleState {
    fn set_active(&self, active: bool) {
        self.active.set(active);
    }

    fn is_active(&self) -> bool {
        self.active.get()
    }
}

impl<T: 'static, E: 'static + Render> IntoElement for ResizeHandle<T, E> {
    type Element = ResizeHandle<T, E>;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl<T: 'static, E: 'static + Render> Element for ResizeHandle<T, E> {
    type RequestLayoutState = AnyElement;
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
        let neg_offset = -HANDLE_PADDING;
        let axis = self.axis;

        window.with_element_state(id.unwrap(), |state, window| {
            let state = state.unwrap_or(ResizeHandleState::default());

            let bg_color = handle_color(&cx.theme(), state.is_active());

            let mut el = div()
                .id(self.id.clone())
                .occlude()
                .absolute()
                .flex_shrink_0()
                .group("handle")
                .when_some(self.on_drag.clone(), |this, on_drag| {
                    this.on_drag(
                        self.drag_value.clone().unwrap(),
                        move |_, position, window, cx| on_drag(&position, window, cx),
                    )
                })
                .map(|this| match self.placement {
                    Some(Side::Left) => {
                        // Special for Left Dock
                        //  FIXME: Improve this to let the scroll bar have px(HANDLE_PADDING)
                        this.cursor_col_resize()
                            .top_0()
                            .right(px(1.))
                            .h_full()
                            .w(HANDLE_SIZE)
                            .pl(HANDLE_PADDING)
                    }
                    _ => this
                        .when(axis.is_horizontal(), |this| {
                            this.cursor_col_resize()
                                .top_0()
                                .left(neg_offset)
                                .h_full()
                                .w(HANDLE_SIZE)
                                .px(HANDLE_PADDING)
                        })
                        .when(axis.is_vertical(), |this| {
                            this.cursor_row_resize()
                                .top(neg_offset)
                                .left_0()
                                .w_full()
                                .h(HANDLE_SIZE)
                                .py(HANDLE_PADDING)
                        }),
                })
                .child(
                    // A renderer that declines — or is absent — leaves the
                    // built-in line, so overriding one handle never obliges a
                    // caller to redraw them all.
                    self.appearance
                        .as_ref()
                        .and_then(|appearance| {
                            appearance(
                                &ResizeHandleContext {
                                    axis,
                                    active: state.is_active(),
                                },
                                window,
                                cx,
                            )
                        })
                        .unwrap_or_else(|| {
                            div()
                                .bg(bg_color)
                                .group_hover("handle", |this| this.bg(bg_color))
                                .when(axis.is_horizontal(), |this| this.h_full().w(HANDLE_SIZE))
                                .when(axis.is_vertical(), |this| this.w_full().h(HANDLE_SIZE))
                                .into_any_element()
                        }),
                )
                .into_any_element();

            let layout_id = el.request_layout(window, cx);

            ((layout_id, el), state)
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
        request_layout.prepaint(window, cx);
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
        request_layout.paint(window, cx);

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

/// What a resize handle paints, given the active theme.
///
/// Projected colors win; without them the handle resolves from the tokens that
/// already mean these two states everywhere else -- `border` for a divider at
/// rest, `ring` for the thing the pointer currently owns. Before this the
/// unprojected answer was `Hsla::default()`, which is transparent, so a
/// consumer with no styled façade had no divider at all.
pub(crate) fn handle_color(theme: &crate::Theme, active: bool) -> gpui::Hsla {
    if active {
        theme
            .resizable
            .active_handle
            .unwrap_or(theme.tokens.colors.ring)
    } else {
        theme.resizable.handle.unwrap_or(theme.tokens.colors.border)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, hsla};

    use super::handle_color;
    use crate::{ResizableTheme, Theme};

    #[gpui::test]
    fn an_unprojected_handle_resolves_from_the_theme_tokens(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let border = hsla(0., 0., 0.5, 1.0);
            let ring = hsla(0.6, 0.5, 0.5, 1.0);
            let theme = Theme::global_mut(cx);
            theme.tokens.colors.border = border;
            theme.tokens.colors.ring = ring;
            theme.resizable = ResizableTheme::default();

            let theme = Theme::global(cx);
            assert_eq!(handle_color(&theme, false), border);
            assert_eq!(handle_color(&theme, true), ring);
            // The point of the change: the default used to be transparent, so
            // a divider with nothing projected onto it was not drawn at all.
            assert_ne!(handle_color(&theme, false), gpui::Hsla::default());
        });
    }

    #[gpui::test]
    fn a_projected_handle_still_wins(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let projected = hsla(0.3, 0.4, 0.5, 1.0);
            let active = hsla(0.9, 0.4, 0.5, 1.0);
            let theme = Theme::global_mut(cx);
            theme.tokens.colors.border = hsla(0., 0., 0.5, 1.0);
            theme.resizable = ResizableTheme {
                handle: Some(projected),
                active_handle: Some(active),
            };

            let theme = Theme::global(cx);
            assert_eq!(handle_color(&theme, false), projected);
            assert_eq!(handle_color(&theme, true), active);
        });
    }
}

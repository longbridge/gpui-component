//! The gpui-component appearance for the dock area: the outer frame, the
//! split frames, and one dock's chrome.

use std::{ops::Deref as _, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, AppContext as _, Axis, Context, Div, Element, Empty, InteractiveElement as _,
    IntoElement, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Render, Stateful, Style,
    Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::dock::{
    DockAreaRenderer, DockContext, DockEvent, DockPlacement, NodeId, PanelState, PanelView,
    TabGroupRenderer, TilesRenderer,
};

use crate::{
    ActiveTheme as _, Side, StyledExt as _,
    dock::{
        DockSkin, SkinShared, invalid_panel::InvalidPanel, panel_handle, tab_panel::TabGroupSkin,
        tiles::TilesSkin,
    },
    resize_handle,
};

/// The payload a dock's resize handle drags. It draws nothing: the handle
/// itself is the affordance.
#[derive(Clone)]
struct ResizePanel;

impl Render for ResizePanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl DockAreaRenderer for DockSkin {
    fn frame(&self, _: &mut Window, _: &mut App) -> Stateful<Div> {
        div()
            .id("dock-area")
            .relative()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_row()
    }

    fn center_frame(&self, _: &mut Window, _: &mut App) -> Stateful<Div> {
        div()
            .id("dock-area-center")
            .flex()
            .flex_1()
            .flex_col()
            .overflow_hidden()
    }

    fn split_frame(&self, node: NodeId, _: Axis, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        // A split frame with no size at all collapses: base puts it between
        // a `resizable_panel` and the resizable group, and between
        // `center_frame` and the centre's root split, and neither parent
        // sizes it. `the_centre_and_the_bottom_dock_share_the_column` fails
        // without this. `size_full` is what the old `StackPanel::render`
        // carried; `flex_1` is belt and braces — either alone passes every
        // case I could construct, so this does not depend on which one wins
        // in a given parent.
        div()
            .id(("dock-split-frame", node.as_u64()))
            .size_full()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
            .bg(cx.theme().tokens.tab_bar)
    }

    fn render_dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let placement = dock.placement();
        let open = dock.is_open();

        // A closed left or right dock takes no space at all; a closed bottom
        // dock keeps a strip so its tab bar stays clickable.
        if !open && !placement.is_bottom() {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_none()
            .relative()
            .overflow_hidden()
            .map(|this| match placement {
                DockPlacement::Left | DockPlacement::Right => this.h_flex().h_full().w(dock.size()),
                DockPlacement::Bottom => this.w_full().h(dock.size()),
                // Base never builds a dock for the centre.
                DockPlacement::Center => this,
            })
            .when(!open && placement.is_bottom(), |this| this.h(px(29.)))
            .child(content)
            .child(self.render_resize_handle(dock, window, cx))
            .child(DockResizeTracker {
                dock: dock.clone(),
                shared: self.shared().clone(),
            })
            .into_any_element()
    }

    /// The "unknown panel" message the old `InvalidPanel` drew.
    ///
    /// It answers `dump` with the state it was handed, so a layout written by
    /// a build that knows the panel survives a load and save here.
    fn build_placeholder(
        &self,
        state: &PanelState,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<Arc<dyn PanelView>> {
        let state = state.clone();
        Some(panel_handle(cx.new(|cx| {
            InvalidPanel::new(state.panel_name.clone(), state, cx)
        })))
    }

    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(TabGroupSkin::new(self.shared().clone()))
    }

    fn tiles_renderer(&self) -> Rc<dyn TilesRenderer> {
        Rc::new(TilesSkin::new(self.shared().clone()))
    }
}

impl DockSkin {
    fn render_resize_handle(
        &self,
        dock: &DockContext,
        _: &mut Window,
        _: &mut App,
    ) -> impl IntoElement {
        let placement = dock.placement();
        let shared = self.shared().clone();

        resize_handle("resize-handle", placement.axis())
            .when(placement.is_left(), |this| this.placement(Side::Left))
            .on_drag(ResizePanel, move |info, _, _, cx| {
                cx.stop_propagation();
                shared.resizing_dock().set(Some(placement));
                cx.new(|_| info.deref().clone())
            })
    }
}

/// Turns the window's mouse stream into dock resizing.
///
/// A resize is driven by pointer moves that land anywhere in the window, not
/// only on the handle, so it cannot be expressed as a listener on the handle
/// itself. This element paints nothing and exists for its `paint` hook, which
/// is the only place a window-level mouse listener can be registered — which
/// is why it stays in the skin rather than moving into base: it is a
/// paint-order concern of this appearance.
struct DockResizeTracker {
    dock: DockContext,
    shared: Rc<SkinShared>,
}

impl IntoElement for DockResizeTracker {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DockResizeTracker {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let placement = self.dock.placement();

        window.on_mouse_event({
            let dock = self.dock.clone();
            let shared = self.shared.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() || shared.resizing_dock().get() != Some(placement) {
                    return;
                }
                // Dragging a closed dock's handle reopens it, as the old dock
                // did. The live state is read rather than the render-time
                // snapshot in `dock`, which would still say closed for the
                // rest of the frame and toggle it shut again on the next move.
                let open = shared
                    .area()
                    .upgrade()
                    .is_some_and(|area| area.read(cx).is_dock_open(placement));
                if !open {
                    dock.toggle(window, cx);
                }
                dock.resize_to(event.position, window, cx);
            }
        });

        window.on_mouse_event({
            let shared = self.shared.clone();
            move |_: &MouseUpEvent, phase, _, cx| {
                if !phase.bubble() || shared.resizing_dock().get() != Some(placement) {
                    return;
                }
                shared.resizing_dock().set(None);
                // The size lives on the dock, not in the layout tree, so
                // nothing else tells a subscriber to persist it.
                _ = shared
                    .area()
                    .update(cx, |_, cx| cx.emit(DockEvent::LayoutChanged));
            }
        });
    }
}

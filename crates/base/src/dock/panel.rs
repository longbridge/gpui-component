use std::{collections::HashMap, sync::Arc};

use gpui::{
    AnyView, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, WeakEntity, Window,
};

use super::layout::PanelId;
use super::state::PanelState;
use super::state_convert::PanelSource;
use super::tab_group::TabGroup;

pub enum PanelEvent {
    ZoomIn,
    ZoomOut,
    LayoutChanged,
}

/// Behavior a dockable panel provides. Presentation lives in the layer above:
/// `gpui_component::dock::Panel` extends this with titles, toolbars, and menus.
#[allow(unused_variables)]
pub trait Panel: EventEmitter<PanelEvent> + Render + Focusable {
    /// Identifies the panel in persisted layouts. Once chosen, never change it.
    fn panel_name(&self) -> &'static str;

    fn visible(&self, cx: &App) -> bool {
        true
    }

    fn closable(&self, cx: &App) -> bool {
        true
    }

    /// Whether the panel can zoom at all. Where the zoom control appears is a
    /// presentation decision and belongs to the layer above.
    fn zoomable(&self, cx: &App) -> bool {
        true
    }

    /// Called with the frame-end net state when this panel becomes, or stops
    /// being, the displayed tab of its group: exactly one notification per
    /// edge, delivered on the next tick after the change — never same-value
    /// repeats nor false-then-true flips within one frame.
    ///
    /// A panel removed from its group is NOT told `false`; [`Panel::on_removed`]
    /// is the deactivation signal. A hidden panel occupying the active slot
    /// still receives `true` even though rendering falls back to the first
    /// visible panel.
    fn set_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {}

    fn set_zoomed(&mut self, zoomed: bool, window: &mut Window, cx: &mut Context<Self>) {}

    fn on_added_to(
        &mut self,
        group: WeakEntity<TabGroup>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
    }

    fn on_removed(&mut self, window: &mut Window, cx: &mut Context<Self>) {}

    fn dump(&self, cx: &App) -> PanelState {
        PanelState::new(self.panel_name())
    }
}

/// Object-safe counterpart of [`Panel`], used to hold heterogeneous panel
/// entities behind a single handle.
#[allow(unused_variables)]
pub trait PanelView: 'static + Send + Sync {
    fn panel_name(&self, cx: &App) -> &'static str;
    fn panel_id(&self, cx: &App) -> PanelId;
    fn closable(&self, cx: &App) -> bool;
    fn zoomable(&self, cx: &App) -> bool;
    fn visible(&self, cx: &App) -> bool;
    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App);
    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App);
    fn on_added_to(&self, group: WeakEntity<TabGroup>, window: &mut Window, cx: &mut App);
    fn on_removed(&self, window: &mut Window, cx: &mut App);
    fn view(&self) -> AnyView;
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    fn dump(&self, cx: &App) -> PanelState;
}

impl<T: Panel> PanelView for Entity<T> {
    fn panel_name(&self, cx: &App) -> &'static str {
        self.read(cx).panel_name()
    }

    fn panel_id(&self, _: &App) -> PanelId {
        PanelId::from(self.entity_id())
    }

    fn closable(&self, cx: &App) -> bool {
        self.read(cx).closable(cx)
    }

    fn zoomable(&self, cx: &App) -> bool {
        self.read(cx).zoomable(cx)
    }

    fn visible(&self, cx: &App) -> bool {
        self.read(cx).visible(cx)
    }

    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| {
            this.set_active(active, window, cx);
        })
    }

    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| {
            this.set_zoomed(zoomed, window, cx);
        })
    }

    fn on_added_to(&self, group: WeakEntity<TabGroup>, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.on_added_to(group, window, cx));
    }

    fn on_removed(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.on_removed(window, cx));
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn dump(&self, cx: &App) -> PanelState {
        self.read(cx).dump(cx)
    }
}

impl From<&dyn PanelView> for AnyView {
    fn from(handle: &dyn PanelView) -> Self {
        handle.view()
    }
}

impl<T: Panel> From<&dyn PanelView> for Entity<T> {
    fn from(value: &dyn PanelView) -> Self {
        value.view().downcast::<T>().unwrap()
    }
}

impl PartialEq for dyn PanelView {
    fn eq(&self, other: &Self) -> bool {
        self.view() == other.view()
    }
}

/// Reads panel properties out of the live entity map that `DockArea` keeps.
///
/// Unused until `DockArea` lands in Task 13, which is the first caller.
/// Kept here now (rather than deferred) because it is the natural
/// `PanelSource` companion to `PanelView` defined just above, and Task 5's
/// `LayoutTree::to_state` already expects a `PanelSource` implementation to
/// exist over live entities.
#[allow(dead_code)]
pub(crate) struct LivePanels<'a> {
    panels: &'a HashMap<PanelId, Arc<dyn PanelView>>,
    cx: &'a App,
}

#[allow(dead_code)]
impl<'a> LivePanels<'a> {
    pub(crate) fn new(panels: &'a HashMap<PanelId, Arc<dyn PanelView>>, cx: &'a App) -> Self {
        Self { panels, cx }
    }
}

impl PanelSource for LivePanels<'_> {
    fn panel_name(&self, id: PanelId) -> &'static str {
        self.panels
            .get(&id)
            .map(|panel| panel.panel_name(self.cx))
            .unwrap_or("")
    }

    fn is_visible(&self, id: PanelId) -> bool {
        self.panels
            .get(&id)
            .is_some_and(|panel| panel.visible(self.cx))
    }

    fn dump(&self, id: PanelId) -> PanelState {
        self.panels
            .get(&id)
            .map(|panel| panel.dump(self.cx))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::PanelInfo;
    use super::*;
    use gpui::{
        AppContext as _, Context, Empty, EventEmitter, FocusHandle, Focusable, IntoElement, Render,
        TestAppContext, Window,
    };

    struct Probe {
        focus_handle: FocusHandle,
        visible: bool,
    }

    impl Panel for Probe {
        fn panel_name(&self) -> &'static str {
            "Probe"
        }

        fn visible(&self, _: &App) -> bool {
            self.visible
        }
    }

    impl EventEmitter<PanelEvent> for Probe {}
    impl Focusable for Probe {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Empty
        }
    }

    #[gpui::test]
    fn a_panel_entity_answers_through_the_object_safe_view(cx: &mut TestAppContext) {
        let panel = cx.new(|cx| Probe {
            focus_handle: cx.focus_handle(),
            visible: false,
        });
        let view: Arc<dyn PanelView> = Arc::new(panel.clone());

        cx.read(|cx| {
            assert_eq!(view.panel_name(cx), "Probe");
            assert_eq!(view.visible(cx), false);
            assert_eq!(view.panel_id(cx), PanelId::from(panel.entity_id()));
        });
    }

    #[gpui::test]
    fn the_default_dump_records_only_the_panel_name(cx: &mut TestAppContext) {
        let panel = cx.new(|cx| Probe {
            focus_handle: cx.focus_handle(),
            visible: true,
        });
        let state = cx.read(|cx| panel.read(cx).dump(cx));

        assert_eq!(state.panel_name, "Probe");
        assert!(state.children.is_empty());
        assert_eq!(state.info, PanelInfo::panel(serde_json::Value::Null));
    }
}

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, FocusHandle, Global, InteractiveElement,
    IntoElement, LayoutId, ParentElement as _, Pixels, WeakFocusHandle, Window, div,
};
use std::collections::HashMap;

/// Global state to manage all focus trap containers
pub(crate) struct FocusTrapManager {
    /// Map from container element ID to its focus trap info
    traps: HashMap<ElementId, WeakFocusHandle>,
}

impl Global for FocusTrapManager {}

impl FocusTrapManager {
    /// Create a new focus trap manager
    fn new() -> Self {
        Self {
            traps: HashMap::new(),
        }
    }

    pub(crate) fn global(cx: &App) -> &Self {
        cx.global::<FocusTrapManager>()
    }

    pub(crate) fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<FocusTrapManager>()
    }

    /// Register a focus trap container
    fn register_trap(id: ElementId, container_handle: WeakFocusHandle, cx: &mut App) {
        Self::global_mut(cx).traps.insert(id, container_handle);
    }

    /// Find which focus trap contains the currently focused element
    pub(crate) fn find_active_trap(window: &Window, cx: &App) -> Option<FocusHandle> {
        for (_id, container_handle) in Self::global(cx).traps.iter() {
            let Some(container) = container_handle.upgrade() else {
                continue;
            };

            if container.contains_focused(window, cx) {
                return Some(container.clone());
            }
        }
        None
    }
}

impl Default for FocusTrapManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the focus trap manager as a global
pub(crate) fn init_focus_trap_manager(cx: &mut App) {
    cx.set_global(FocusTrapManager::new());
}

/// A wrapper element that implements focus trap behavior.
///
/// This element wraps another element and registers it as a focus trap container.
/// Focus will automatically cycle within the container when Tab/Shift-Tab is pressed.
pub struct FocusTrapElement {
    id: ElementId,
    focus_handle: FocusHandle,
    child: Option<AnyElement>,
}

impl FocusTrapElement {
    pub(crate) fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        focus_handle: FocusHandle,
        child: E,
    ) -> Self {
        Self {
            id: id.into(),
            focus_handle,
            child: Some(child.into_any_element()),
        }
    }
}

impl IntoElement for FocusTrapElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FocusTrapElement {
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
        _global_id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Register this focus trap with the manager
        FocusTrapManager::register_trap(self.id.clone(), self.focus_handle.downgrade(), cx);

        let mut el = div()
            .track_focus(&self.focus_handle)
            .children(self.child.take())
            .into_any_element();
        let layout_id = el.request_layout(window, cx);
        (layout_id, el)
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child_element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // Prepaint the child
        child_element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _global_id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child_element: &mut Self::RequestLayoutState,
        _prepaint_state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child_element.paint(window, cx);
    }
}

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, FocusHandle, Global, GlobalElementId,
    InteractiveElement, Interactivity, IntoElement, LayoutId, ParentElement, Pixels,
    StatefulInteractiveElement, StyleRefinement, Styled, WeakFocusHandle, Window,
};
use std::collections::HashMap;

/// Initialize the focus trap manager as a global
pub(crate) fn init(cx: &mut App) {
    cx.set_global(FocusTrapManager::new());
}

/// An extension trait to add `focus_trap` functionality to interactive elements.
pub trait FocusTrapElement: InteractiveElement + Sized {
    /// Enable focus trap for this element.
    ///
    /// When enabled, focus will automatically cycle within this container
    /// instead of escaping to parent elements. This is useful for modal dialogs,
    /// sheets, and other overlay components.
    ///
    /// The focus trap works by:
    /// 1. Registering this element as a focus trap container
    /// 2. When Tab/Shift-Tab is pressed, Root intercepts the event
    /// 3. If focus would leave the container, it cycles back to the beginning/end
    ///
    /// # Example
    ///
    /// ```ignore
    /// v_flex()
    ///     .child(Button::new("btn1").label("Button 1"))
    ///     .child(Button::new("btn2").label("Button 2"))
    ///     .child(Button::new("btn3").label("Button 3"))
    ///     .focus_trap("trap1", &self.container_focus_handle)
    /// // Pressing Tab will cycle: btn1 -> btn2 -> btn3 -> btn1
    /// // Focus will not escape to elements outside this container
    /// ```
    ///
    /// See also: <https://github.com/focus-trap/focus-trap-react>
    fn focus_trap(
        self,
        id: impl Into<ElementId>,
        focus_handle: &FocusHandle,
    ) -> FocusTrapContainer<Self>
    where
        Self: ParentElement + Styled + Element + 'static,
    {
        FocusTrapContainer::new(id, focus_handle.clone(), self)
    }
}
impl<T: InteractiveElement + Sized> FocusTrapElement for T {}

/// Global state to manage all focus trap containers
pub(crate) struct FocusTrapManager {
    /// Map from container element ID to its focus trap info
    traps: HashMap<GlobalElementId, WeakFocusHandle>,
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

    fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<FocusTrapManager>()
    }

    /// Register a focus trap container
    fn register_trap(id: &GlobalElementId, container_handle: WeakFocusHandle, cx: &mut App) {
        let this = Self::global_mut(cx);
        this.traps.insert(id.clone(), container_handle);
        this.cleanup();
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

    /// Cleanup any traps with dropped handles
    fn cleanup(&mut self) {
        self.traps.retain(|_, handle| handle.upgrade().is_some());
    }
}

impl Default for FocusTrapManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A wrapper element that implements focus trap behavior.
///
/// This element wraps another element and registers it as a focus trap container.
/// Focus will automatically cycle within the container when Tab/Shift-Tab is pressed.
pub struct FocusTrapContainer<E: InteractiveElement + ParentElement + Styled + Element> {
    id: ElementId,
    focus_handle: FocusHandle,
    base: E,
}

impl<E: InteractiveElement + ParentElement + Styled + Element> FocusTrapContainer<E> {
    pub(crate) fn new(id: impl Into<ElementId>, focus_handle: FocusHandle, child: E) -> Self {
        Self {
            id: id.into(),
            base: child.track_focus(&focus_handle),
            focus_handle,
        }
    }
}

impl<E: InteractiveElement + ParentElement + Styled + Element> IntoElement
    for FocusTrapContainer<E>
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
impl<E: InteractiveElement + ParentElement + Styled + Element> ParentElement
    for FocusTrapContainer<E>
{
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}
impl<E: InteractiveElement + ParentElement + Styled + Element> InteractiveElement
    for FocusTrapContainer<E>
{
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}
impl<E: InteractiveElement + ParentElement + Styled + Element> StatefulInteractiveElement
    for FocusTrapContainer<E>
{
}
impl<E: InteractiveElement + ParentElement + Styled + Element> Styled for FocusTrapContainer<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl<E: InteractiveElement + ParentElement + Styled + Element + 'static> Element
    for FocusTrapContainer<E>
{
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Register this focus trap with the manager
        FocusTrapManager::register_trap(global_id.unwrap(), self.focus_handle.downgrade(), cx);

        self.base.request_layout(global_id, None, window, cx)
    }

    fn prepaint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.base
            .prepaint(global_id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.base.paint(
            global_id,
            inspector_id,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        )
    }
}

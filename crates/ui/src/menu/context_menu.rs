use std::{cell::RefCell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Context, DismissEvent, Element, ElementId, Entity, Focusable,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, StyleRefinement, Styled,
    Subscription, Window, anchored, deferred, div, prelude::FluentBuilder, px,
};

use crate::menu::PopupMenu;
use crate::native_menu::NativeMenu;

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: InteractiveElement + ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// The menu is built lazily on right-click and shown as a native OS menu
    /// via [`NativeMenu`], so it is not clipped to the window bounds.
    fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        let id = self
            .interactivity()
            .element_id
            .clone()
            .map(|id| format!("context-menu-{:?}", id))
            .unwrap_or_else(|| format!("context-menu-{:p}", &self as *const _));
        ContextMenu::new(id, self).menu(f)
    }

    /// Add a context menu to the element, rendered as a GPUI [`PopupMenu`].
    ///
    /// Unlike [`Self::context_menu`] (which uses a native OS menu), this draws
    /// the menu with GPUI so it supports icons, links, custom elements, etc.,
    /// at the cost of being clipped to the window bounds.
    fn popup_context_menu(
        mut self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> PopupContextMenu<Self>
    where
        Self: Sized,
    {
        let id = self
            .interactivity()
            .element_id
            .clone()
            .map(|id| format!("context-menu-{:?}", id))
            .unwrap_or_else(|| format!("context-menu-{:p}", &self as *const _));
        PopupContextMenu::new(id, self).menu(f)
    }
}

impl<E: InteractiveElement + ParentElement + Styled> ContextMenuExt for E {}

/// A context menu that can be shown on right-click, rendered as a native OS menu.
pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
    _ignore_style: StyleRefinement,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            _ignore_style: StyleRefinement::default(),
        }
    }

    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }
}

impl<E: ParentElement + Styled> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut element = self
            .element
            .take()
            .expect("Element should exists.")
            .into_any_element();

        let layout_id = element.request_layout(window, cx);
        (layout_id, element)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        element.prepaint(window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        element: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);

        let Some(builder) = self.menu.clone() else {
            return;
        };
        let hitbox = hitbox.clone();

        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase.bubble() && event.button == MouseButton::Right && hitbox.is_hovered(window) {
                let position = event.position;
                let builder = builder.clone();

                window.defer(cx, move |window, cx| {
                    let menu = builder(NativeMenu::new(), window, cx);
                    menu.show(position, window, cx);
                });
            }
        });
    }
}

/// A context menu that can be shown on right-click, drawn by GPUI as a [`PopupMenu`].
pub struct PopupContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>>,
    _ignore_style: StyleRefinement,
    anchor: Anchor,
}

impl<E: ParentElement + Styled> PopupContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Anchor::TopLeft,
            _ignore_style: StyleRefinement::default(),
        }
    }

    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut PopupContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<PopupContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<E: ParentElement + Styled> ParentElement for PopupContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for PopupContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for PopupContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct PopupContextMenuSharedState {
    menu_view: Option<Entity<PopupMenu>>,
    open: bool,
    position: Point<Pixels>,
    _subscription: Option<Subscription>,
}

pub struct PopupContextMenuState {
    element: Option<AnyElement>,
    shared_state: Rc<RefCell<PopupContextMenuSharedState>>,
}

impl Default for PopupContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            shared_state: Rc::new(RefCell::new(PopupContextMenuSharedState {
                menu_view: None,
                open: false,
                position: Default::default(),
                _subscription: None,
            })),
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for PopupContextMenu<E> {
    type RequestLayoutState = PopupContextMenuState;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let anchor = self.anchor;

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, state: &mut PopupContextMenuState, window, cx| {
                let (position, open) = {
                    let shared_state = state.shared_state.borrow();
                    (shared_state.position, shared_state.open)
                };
                let menu_view = state.shared_state.borrow().menu_view.clone();
                let mut menu_element = None;
                if open {
                    let has_menu_item = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_menu_item {
                        menu_element = Some(
                            deferred(
                                anchored().child(
                                    div()
                                        .w(window.bounds().size.width)
                                        .h(window.bounds().size.height)
                                        .on_scroll_wheel(|_, _, cx| {
                                            cx.stop_propagation();
                                        })
                                        .child(
                                            anchored()
                                                .position(position)
                                                .snap_to_window_with_margin(px(8.))
                                                .anchor(anchor)
                                                .when_some(menu_view, |this, menu| {
                                                    if !menu
                                                        .focus_handle(cx)
                                                        .contains_focused(window, cx)
                                                    {
                                                        menu.focus_handle(cx).focus(window, cx);
                                                    }

                                                    this.child(menu.clone())
                                                }),
                                        ),
                                ),
                            )
                            .with_priority(1)
                            .into_any(),
                        );
                    }
                }

                let mut element = this
                    .element
                    .take()
                    .expect("Element should exists.")
                    .children(menu_element)
                    .into_any_element();

                let layout_id = element.request_layout(window, cx);

                (
                    layout_id,
                    PopupContextMenuState {
                        element: Some(element),
                        ..Default::default()
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        let builder = self.menu.clone();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_view, state: &mut PopupContextMenuState, window, _| {
                let shared_state = state.shared_state.clone();

                let hitbox = hitbox.clone();
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase.bubble()
                        && event.button == MouseButton::Right
                        && hitbox.is_hovered(window)
                    {
                        {
                            let mut shared_state = shared_state.borrow_mut();
                            shared_state.menu_view = None;
                            shared_state._subscription = None;
                            shared_state.position = event.position;
                            shared_state.open = true;
                        }

                        window.defer(cx, {
                            let shared_state = shared_state.clone();
                            let builder = builder.clone();
                            move |window, cx| {
                                let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                                    let Some(build) = &builder else {
                                        return menu;
                                    };
                                    build(menu, window, cx)
                                });

                                let _subscription = window.subscribe(&menu, cx, {
                                    let shared_state = shared_state.clone();
                                    move |_, _: &DismissEvent, window, _cx| {
                                        shared_state.borrow_mut().open = false;
                                        window.refresh();
                                    }
                                });

                                {
                                    let mut state = shared_state.borrow_mut();
                                    state.menu_view = Some(menu.clone());
                                    state._subscription = Some(_subscription);
                                    window.refresh();
                                }
                            }
                        });
                    }
                });
            },
        );
    }
}

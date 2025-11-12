use std::{cell::RefCell, rc::Rc};

use gpui::{
    anchored, deferred, div, prelude::FluentBuilder, px, AnyElement, App, Context, Corner,
    DismissEvent, Element, ElementId, Entity, Focusable, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    StyleRefinement, Styled, Subscription, Window,
};

use crate::menu::PopupMenu;

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// This will changed the element to be `relative` positioned, and add a child `ContextMenu` element.
    /// Because the `ContextMenu` element is positioned `absolute`, it will not affect the layout of the parent element.
    fn context_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self> {
        ContextMenu::new("context-menu", self).menu(f)
    }
}

impl<E: ParentElement + Styled> ContextMenuExt for E {}

/// A context menu that can be shown on right-click.
pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    // This is not in use, just for style refinement forwarding.
    _ingore_style: StyleRefinement,
    anchor: Corner,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Corner::TopLeft,
            _ingore_style: StyleRefinement::default(),
        }
    }

    /// Build the context menu using the given builder function.
    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.menu = Some(Box::new(builder));
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
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
            &mut self._ingore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct ContextMenuSharedState {
    menu_view: Option<Entity<PopupMenu>>,
    open: bool,
    position: Point<Pixels>,
    _subscription: Option<Subscription>,
}

pub struct ContextMenuState {
    element: Option<AnyElement>,
    shared_state: Rc<RefCell<ContextMenuSharedState>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            shared_state: Rc::new(RefCell::new(ContextMenuSharedState {
                menu_view: None,
                open: false,
                position: Default::default(),
                _subscription: None,
            })),
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = ContextMenuState;
    type PrepaintState = ();

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
            |this, state: &mut ContextMenuState, window, cx| {
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
                                anchored()
                                    .position(position)
                                    .snap_to_window_with_margin(px(8.))
                                    .anchor(anchor)
                                    .when_some(menu_view, |this, menu| {
                                        // Focus the menu, so that can be handle the action.
                                        if !menu.focus_handle(cx).contains_focused(window, cx) {
                                            menu.focus_handle(cx).focus(window);
                                        }

                                        this.child(div().occlude().child(menu.clone()))
                                    }),
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
                    ContextMenuState {
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
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        let Some(builder) = self.menu.take() else {
            return;
        };

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_view, state: &mut ContextMenuState, window, _| {
                let shared_state = state.shared_state.clone();

                // When right mouse click, to build content menu, and show it at the mouse position.
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase.bubble()
                        && event.button == MouseButton::Right
                        && bounds.contains(&event.position)
                    {
                        {
                            let mut shared_state = shared_state.borrow_mut();
                            shared_state.position = event.position;
                            shared_state.open = true;
                        }

                        let menu = PopupMenu::build(window, cx, |menu, window, cx| {
                            (builder)(menu, window, cx)
                        })
                        .into_element();

                        let _subscription = window.subscribe(&menu, cx, {
                            let shared_state = shared_state.clone();
                            move |_, _: &DismissEvent, window, _| {
                                shared_state.borrow_mut().open = false;
                                window.refresh();
                            }
                        });

                        shared_state.borrow_mut().menu_view = Some(menu.clone());
                        shared_state.borrow_mut()._subscription = Some(_subscription);
                        window.refresh();
                    }
                });
            },
        );
    }
}

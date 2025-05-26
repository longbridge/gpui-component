use std::{cell::RefCell, rc::Rc};

use gpui::{
    anchored, deferred, div, prelude::FluentBuilder, px, relative, AnyElement, App, Context,
    Corner, DismissEvent, DispatchPhase, Element, ElementId, Entity, Focusable, FocusableWrapper,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Pixels, Point, Position, Stateful, Style, Window,
};

use crate::menu::popup_menu::PopupMenu;

pub trait ContextMenuExt: ParentElement + Sized {
    fn context_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.child(ContextMenu::new("context-menu").menu(f))
    }
}

impl<E> ContextMenuExt for Stateful<E> where E: ParentElement {}
impl<E> ContextMenuExt for FocusableWrapper<E> where E: ParentElement {}

/// A context menu that can be shown on right-click.
pub struct ContextMenu {
    id: ElementId,
    menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    anchor: Corner,
}

impl ContextMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            menu: None,
            anchor: Corner::TopLeft,
        }
    }

    #[must_use]
    pub fn menu<F>(mut self, builder: F) -> Self
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

impl IntoElement for ContextMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct ContextMenuState {
    menu_view: Rc<RefCell<Option<Entity<PopupMenu>>>>,
    menu_element: Option<AnyElement>,
    open: Rc<RefCell<bool>>,
    position: Rc<RefCell<Point<Pixels>>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            menu_view: Rc::new(RefCell::new(None)),
            menu_element: None,
            open: Rc::new(RefCell::new(false)),
            position: Default::default(),
        }
    }
}

impl Element for ContextMenu {
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
        let mut style = Style::default();
        // Set the layout style relative to the table view to get same size.
        style.position = Position::Absolute;
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        let anchor = self.anchor;

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_, state: &mut ContextMenuState, window, cx| {
                let position = state.position.clone();
                let position = position.borrow();
                let open = state.open.clone();
                let menu_view = state.menu_view.borrow().clone();

                let (menu_element, menu_layout_id) = if *open.borrow() {
                    let has_menu_item = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_menu_item {
                        let mut menu_element = deferred(
                            anchored()
                                .position(*position)
                                .snap_to_window_with_margin(px(8.))
                                .anchor(anchor)
                                .when_some(menu_view, |this, menu| {
                                    // Focus the menu, so that can be handle the action.
                                    menu.focus_handle(cx).focus(window);

                                    this.child(div().occlude().child(menu.clone()))
                                }),
                        )
                        .with_priority(1)
                        .into_any();

                        let menu_layout_id = menu_element.request_layout(window, cx);
                        (Some(menu_element), Some(menu_layout_id))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                let mut layout_ids = vec![];
                if let Some(menu_layout_id) = menu_layout_id {
                    layout_ids.push(menu_layout_id);
                }

                let layout_id = window.request_layout(style, layout_ids, cx);

                (
                    layout_id,
                    ContextMenuState {
                        menu_element,

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
        if let Some(menu_element) = &mut request_layout.menu_element {
            menu_element.prepaint(window, cx);
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
        if let Some(menu_element) = &mut request_layout.menu_element {
            menu_element.paint(window, cx);
        }

        let Some(builder) = self.menu.take() else {
            return;
        };

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_view, state: &mut ContextMenuState, window, _| {
                let position = state.position.clone();
                let open = state.open.clone();
                let menu_view = state.menu_view.clone();

                // When right mouse click, to build content menu, and show it at the mouse position.
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble
                        && event.button == MouseButton::Right
                        && bounds.contains(&event.position)
                    {
                        *position.borrow_mut() = event.position;
                        *open.borrow_mut() = true;

                        let menu = PopupMenu::build(window, cx, |menu, window, cx| {
                            (builder)(menu, window, cx)
                        })
                        .into_element();

                        let open = open.clone();
                        window
                            .subscribe(&menu, cx, move |_, _: &DismissEvent, window, _| {
                                *open.borrow_mut() = false;
                                window.refresh();
                            })
                            .detach();

                        *menu_view.borrow_mut() = Some(menu);

                        window.refresh();
                    }
                });
            },
        );
    }
}

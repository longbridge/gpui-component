use std::rc::Rc;

use gpui::{
    anchored, deferred, div, prelude::FluentBuilder, px, App, ElementId, InteractiveElement as _,
    IntoElement, OwnedMenuItem, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    h_flex,
    popup_menu::PopupMenu,
    Disableable, Selectable,
};

#[derive(Default)]
struct MenuBarState {
    selected_ix: Option<usize>,
}

/// A menu bar component for the UI.
#[derive(IntoElement)]
pub struct MenuBar {
    id: ElementId,
    menus: Vec<MenuBarMenu>,
}

impl MenuBar {
    /// Create a new menu bar with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id,
            menus: Vec::new(),
        }
    }

    /// Add a menu to the menu bar.
    pub fn menu(mut self, menu: MenuBarMenu) -> Self {
        self.menus.push(menu);
        self
    }

    /// Add multiple menus to the menu bar.
    pub fn menus(mut self, menus: impl IntoIterator<Item = MenuBarMenu>) -> Self {
        self.menus = menus.into_iter().collect();
        self
    }
}

impl RenderOnce for MenuBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| MenuBarState::default());
        let selected_ix = state.read(cx).selected_ix;
        let has_actived_menu = selected_ix.is_some();

        h_flex()
            .id(self.id)
            .gap_x_1()
            .children(self.menus.into_iter().enumerate().map(|(ix, menu)| {
                let is_selected = selected_ix == Some(ix);
                menu.selected(is_selected)
                    .on_click({
                        let state = state.clone();
                        move |_, cx| {
                            _ = state.update(cx, |state, cx| {
                                state.selected_ix = Some(ix);
                                cx.notify();
                            });
                        }
                    })
                    .when(has_actived_menu, |m| {
                        m.on_hover({
                            let state = state.clone();
                            move |hovered, _, cx| {
                                if *hovered {
                                    _ = state.update(cx, |state, cx| {
                                        state.selected_ix = Some(ix);
                                        cx.notify();
                                    });
                                }
                            }
                        })
                    })
                    .when(is_selected, |this| {
                        this.on_mouse_down_out({
                            let state = state.clone();
                            move |_, cx| {
                                _ = state.update(cx, |state, cx| {
                                    state.selected_ix = None;
                                    cx.notify();
                                });
                            }
                        })
                    })
            }))
    }
}

/// A menu in the menu bar.
#[derive(IntoElement)]
pub struct MenuBarMenu {
    id: ElementId,
    name: SharedString,
    items: Vec<OwnedMenuItem>,
    disabled: bool,
    selected: bool,
    on_click: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_hover: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    on_mouse_down_out: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl MenuBarMenu {
    pub fn new(name: impl Into<SharedString>) -> Self {
        let name: SharedString = name.into();
        Self {
            id: name.clone().into(),
            name,
            items: Vec::new(),
            disabled: false,
            selected: false,
            on_click: None,
            on_hover: None,
            on_mouse_down_out: None,
        }
    }

    pub fn item(mut self, item: OwnedMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = OwnedMenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(handler));
        self
    }

    fn on_mouse_down_out(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_mouse_down_out = Some(Rc::new(handler));
        self
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Disableable for MenuBarMenu {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for MenuBarMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let popup_menu = window.use_keyed_state(self.id.clone(), cx, |_, cx| PopupMenu::new(cx));

        div()
            .id(self.id)
            .relative()
            .child(
                Button::new("menu")
                    .py_0p5()
                    .compact()
                    .ghost()
                    .label(self.name)
                    .disabled(self.disabled)
                    .selected(self.selected)
                    .when_some(self.on_click, |b, handler| {
                        b.on_click({
                            let handler = handler.clone();
                            move |_, window, cx| {
                                handler(window, cx);
                            }
                        })
                    }),
            )
            .when_some(self.on_hover, |b, handler| {
                b.on_hover({
                    let handler = handler.clone();
                    move |hovered, window, cx| {
                        handler(hovered, window, cx);
                    }
                })
            })
            .when(self.selected && !self.disabled, |this| {
                this.child(deferred(
                    anchored()
                        .anchor(gpui::Corner::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .mt_1()
                                .child(PopupMenu::build(window, cx, |menu, window, cx| {
                                    menu.with_menu_items(self.items.clone(), window, cx)
                                }))
                                .when_some(self.on_mouse_down_out, |this, handler| {
                                    this.on_mouse_down_out({
                                        let handler = handler.clone();
                                        move |_, window, cx| {
                                            handler(window, cx);
                                        }
                                    })
                                }),
                        ),
                ))
            })
    }
}

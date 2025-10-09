use gpui::{
    anchored, deferred, div, prelude::FluentBuilder, px, App, AppContext as _, Context,
    DismissEvent, Entity, InteractiveElement as _, IntoElement, KeyBinding, OwnedMenu,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
};

use crate::{
    actions::{Cancel, Confirm, SelectLeft, SelectNext, SelectPrev, SelectRight},
    button::{Button, ButtonVariants},
    h_flex,
    popup_menu::PopupMenu,
    Selectable, Sizable,
};

const CONTEXT: &str = "menu_bar";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("up", SelectPrev, Some(CONTEXT)),
        KeyBinding::new("down", SelectNext, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

/// The application menu bar, for Windows and Linux.
pub struct AppMenuBar {
    menus: Vec<Entity<MenuBarMenu>>,
    selected_ix: Option<usize>,
}

impl AppMenuBar {
    /// Create a new menu bar with the given ID.
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let menus = cx
                .get_menus()
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(ix, menu)| MenuBarMenu::new(ix, menu, cx.entity(), window, cx))
                .collect();

            Self {
                selected_ix: None,
                menus,
            }
        })
    }

    fn on_action_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if selected_ix > 0 {
                self.selected_ix = Some(selected_ix - 1);
                cx.notify();
            } else {
                self.selected_ix = Some(self.menus.len().saturating_sub(1));
                cx.notify();
            }
        }
    }

    #[inline]
    fn has_activated_menu(&self) -> bool {
        self.selected_ix.is_some()
    }
}

impl Render for AppMenuBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("app-menu-bar")
            .size_full()
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_left))
            .gap_x_1()
            .overflow_x_scroll()
            .children(self.menus.clone())
    }
}

/// A menu in the menu bar.
pub(super) struct MenuBarMenu {
    menu_bar: Entity<AppMenuBar>,
    ix: usize,
    name: SharedString,
    popup_menu: Entity<PopupMenu>,

    _subscription: Subscription,
}

impl MenuBarMenu {
    pub(super) fn new(
        ix: usize,
        menu: &OwnedMenu,
        menu_bar: Entity<AppMenuBar>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let name = menu.name.clone().into();
        let items = menu.items.clone();

        let popup_menu = PopupMenu::build(window, cx, |menu, window, cx| {
            menu.with_menu_items(items, window, cx)
        });

        cx.new(|cx| {
            let _subscription = cx.subscribe_in(&popup_menu, window, Self::handle_dismiss);

            Self {
                ix,
                menu_bar,
                name,
                popup_menu,
                _subscription,
            }
        })
    }

    fn handle_dismiss(
        &mut self,
        _: &Entity<PopupMenu>,
        _: &DismissEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        _ = self.menu_bar.update(cx, |state, cx| {
            if state.selected_ix == Some(self.ix) {
                state.selected_ix = None;
                cx.notify();
            }
        });
    }
}

impl Render for MenuBarMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_ix = self.ix;
        let menu_bar = self.menu_bar.read(cx);
        let is_selected = menu_bar.selected_ix == Some(self.ix);
        let has_activated_menu = menu_bar.has_activated_menu();

        div()
            .id(self.ix)
            .relative()
            .child(
                Button::new("menu")
                    .small()
                    .py_0p5()
                    .compact()
                    .ghost()
                    .label(self.name.clone())
                    .selected(is_selected)
                    .on_click({
                        let menu_bar = self.menu_bar.clone();
                        move |_, _, cx| {
                            if is_selected {
                                _ = menu_bar.update(cx, |state, cx| {
                                    state.selected_ix = None;
                                    cx.notify();
                                });
                            } else {
                                _ = menu_bar.update(cx, |state, cx| {
                                    state.selected_ix = Some(menu_ix);
                                    cx.notify();
                                });
                            }
                        }
                    }),
            )
            .when(has_activated_menu, |this| {
                this.on_hover({
                    let menu_bar = self.menu_bar.clone();
                    move |hovered, _, cx| {
                        if *hovered {
                            _ = menu_bar.update(cx, |state, cx| {
                                state.selected_ix = Some(menu_ix);
                                cx.notify();
                            });
                        }
                    }
                })
            })
            .when(is_selected, |this| {
                this.child(deferred(
                    anchored()
                        .anchor(gpui::Corner::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .size_full()
                                .occlude()
                                .top_1()
                                .child(self.popup_menu.clone())
                                .on_mouse_down_out({
                                    let menu_bar = self.menu_bar.clone();
                                    move |_, _, cx| {
                                        _ = menu_bar.update(cx, |state, cx| {
                                            if state.selected_ix == Some(menu_ix) {
                                                state.selected_ix = None;
                                                cx.notify();
                                            }
                                        });
                                    }
                                }),
                        ),
                ))
            })
    }
}

use gpui::{
    anchored, deferred, div, px, App, AppContext as _, Context, DismissEvent, Entity, IntoElement,
    ParentElement as _, Pixels, Point, Render, Styled, Subscription, Window,
};

use crate::{
    input::{self, InputState},
    popup_menu::PopupMenu,
};

/// Context menu for mouse right clicks.
pub(crate) struct MouseContextMenu {
    menu: Entity<PopupMenu>,
    mouse_position: Point<Pixels>,
    open: bool,

    _subscriptions: Vec<Subscription>,
}

impl MouseContextMenu {
    pub(crate) fn new(
        _editor: Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let menu = PopupMenu::build(window, cx, |menu, _, _| {
            menu.small()
                .menu("Go to Definition", Box::new(input::GoToDefinition))
                .separator()
                .menu("Cut", Box::new(input::Cut))
                .menu("Copy", Box::new(input::Copy))
                .menu("Paste", Box::new(input::Paste))
        });

        cx.new(|cx| {
            let _subscriptions = vec![cx.subscribe(&menu, {
                move |this: &mut Self, _, _: &DismissEvent, cx| {
                    this.open = false;
                    cx.notify();
                }
            })];

            Self {
                menu,
                mouse_position: Point::default(),
                open: false,
                _subscriptions,
            }
        })
    }

    #[inline]
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn show(&mut self, mouse_position: Point<Pixels>, cx: &mut Context<Self>) {
        self.mouse_position = mouse_position;
        self.open = true;
        cx.notify();
    }
}

impl Render for MouseContextMenu {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        let pos = self.mouse_position;

        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(gpui::Corner::TopLeft)
                .position(pos)
                .child(
                    div()
                        .font_family(".SystemUIFont")
                        .text_size(px(14.))
                        .cursor_default()
                        .child(self.menu.clone()),
                ),
        )
        .into_any_element()
    }
}

use gpui::{
    anchored, deferred, div, px, App, AppContext as _, Context, Corner, DismissEvent, Entity,
    IntoElement, MouseDownEvent, ParentElement as _, Pixels, Point, Render, Styled, Subscription,
    Window,
};
use rust_i18n::t;

use crate::{
    input::{self, popovers::ContextMenu, InputContextMenuItem, InputState},
    menu::{PopupMenu, PopupMenuItem},
    ActiveTheme as _,
};

/// Context menu for mouse right clicks.
pub(crate) struct MouseContextMenu {
    editor: Entity<InputState>,
    menu: Entity<PopupMenu>,
    mouse_position: Point<Pixels>,
    open: bool,

    _subscriptions: Vec<Subscription>,
}

impl InputState {
    fn append_extra_mouse_context_menu_items(
        mut menu: PopupMenu,
        items: &[InputContextMenuItem],
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        for item in items {
            menu = Self::append_extra_mouse_context_menu_item(menu, item, window, cx);
        }
        menu
    }

    fn append_extra_mouse_context_menu_item(
        mut menu: PopupMenu,
        item: &InputContextMenuItem,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        match item {
            InputContextMenuItem::Separator => menu.separator(),
            InputContextMenuItem::Item {
                label,
                icon,
                disabled,
                action,
                on_click,
            } => {
                let mut popup_item = PopupMenuItem::new(label.clone()).disabled(*disabled);
                if let Some(icon) = icon.clone() {
                    popup_item = popup_item.icon(icon);
                }
                if let Some(action) = action {
                    popup_item = popup_item.action(action());
                }
                if let Some(on_click) = on_click {
                    let on_click = on_click.clone();
                    popup_item = popup_item.on_click(move |event, window, cx| {
                        on_click(event, window, cx);
                    });
                }
                menu.item(popup_item)
            }
            InputContextMenuItem::Submenu {
                label,
                icon,
                disabled,
                items,
            } => {
                let submenu_items = items.clone();
                menu = menu.submenu_with_icon(icon.clone(), label.clone(), window, cx, {
                    move |submenu, window, cx| {
                        Self::append_extra_mouse_context_menu_items(
                            submenu,
                            &submenu_items,
                            window,
                            cx,
                        )
                    }
                });

                if *disabled {
                    if let Some(PopupMenuItem::Submenu { disabled, .. }) =
                        menu.menu_items.last_mut()
                    {
                        *disabled = true;
                    }
                }

                menu
            }
        }
    }

    pub(crate) fn handle_right_click_menu(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Show Mouse context menu
        if !self.selected_range.contains(offset) {
            self.move_to(offset, None, cx);
        }

        self.context_menu = Some(ContextMenu::MouseContext(self.mouse_context_menu.clone()));

        let is_code_editor = self.mode.is_code_editor();
        if is_code_editor {
            self.handle_hover_definition(offset, window, cx);
        }

        let is_enable = !self.disabled;
        let is_selected = !self.selected_range.is_empty();
        let has_paste = is_enable && cx.read_from_clipboard().is_some();
        let extra_menu_items = self.mouse_context_menu_items.clone();

        let action_context = self.focus_handle.clone();
        self.mouse_context_menu.update(cx, |this, cx| {
            this.mouse_position = event.position;
            this.menu.update(cx, |menu, cx| {
                let mut new_menu = PopupMenu::new(cx);

                if !extra_menu_items.is_empty() {
                    new_menu = new_menu.separator();
                    new_menu = Self::append_extra_mouse_context_menu_items(
                        new_menu,
                        &extra_menu_items,
                        window,
                        cx,
                    );
                    new_menu = new_menu.separator();
                }

                new_menu = new_menu
                    .menu_with_enable(
                        t!("Input.Cut"),
                        Box::new(input::Cut),
                        is_enable && is_selected,
                    )
                    .menu_with_enable(t!("Input.Copy"), Box::new(input::Copy), is_selected)
                    .menu_with_enable(t!("Input.Paste"), Box::new(input::Paste), has_paste)
                    .separator()
                    .menu(t!("Input.Select All"), Box::new(input::SelectAll));

                menu.menu_items = new_menu.menu_items;
                menu.action_context = Some(action_context);
                cx.notify();
            });
            cx.defer_in(window, |this, _, cx| {
                this.open = true;
                cx.notify();
            });
        });
    }
}

impl MouseContextMenu {
    pub(crate) fn new(
        editor: Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let menu = cx.new(|cx| PopupMenu::new(cx).small());

            let _subscriptions = vec![cx.subscribe_in(&menu, window, {
                move |this: &mut Self, _, _: &DismissEvent, window, cx| {
                    this.close(window, cx);
                }
            })];

            Self {
                editor,
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

    #[inline]
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.editor.update(cx, |this, cx| {
            this.focus(window, cx);
        });
    }
}

impl Render for MouseContextMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(Corner::TopLeft)
                .position(self.mouse_position)
                .child(
                    div()
                        .font_family(cx.theme().font_family.clone())
                        .cursor_default()
                        .child(self.menu.clone()),
                ),
        )
        .into_any_element()
    }
}

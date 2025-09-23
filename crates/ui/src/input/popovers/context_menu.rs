use gpui::{
    anchored, deferred, div, prelude::FluentBuilder as _, px, App, AppContext as _, Context,
    DismissEvent, Entity, IntoElement, MouseDownEvent, ParentElement as _, Pixels, Point, Render,
    Styled, Subscription, Window,
};
use rust_i18n::t;

use crate::{
    input::{self, popovers::ContextMenu, InputState},
    popup_menu::PopupMenu,
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
    pub(crate) fn handle_right_click_menu(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Show Mouse context menu
        if !self.selected_range.contains(offset) {
            self.move_to(offset, cx);
        }

        self.context_menu = Some(ContextMenu::MouseContext(self.mouse_context_menu.clone()));

        let is_code_editor = self.mode.is_code_editor();
        if is_code_editor {
            self.handle_hover_definition(offset, window, cx);
        }

        let has_goto_definition = self.lsp.definition_provider.is_some();
        let has_code_action = !self.lsp.code_action_providers.is_empty();
        let is_selected = !self.selected_range.is_empty();
        let has_paste = cx.read_from_clipboard().is_some();

        self.mouse_context_menu.update(cx, |this, cx| {
            this.mouse_position = event.position;
            this.menu.update(cx, |menu, cx| {
                let new_menu = PopupMenu::new(cx)
                    .when(is_code_editor, |m| {
                        m.menu_with_enable(
                            t!("Input.Go to Definition"),
                            Box::new(input::GoToDefinition),
                            has_goto_definition,
                        )
                        .menu_with_enable(
                            t!("Input.Show Code Actions"),
                            Box::new(input::ToggleCodeActions),
                            has_code_action,
                        )
                        .separator()
                    })
                    .menu_with_enable(t!("Input.Cut"), Box::new(input::Cut), is_selected)
                    .menu_with_enable(t!("Input.Copy"), Box::new(input::Copy), is_selected)
                    .menu_with_enable(t!("Input.Paste"), Box::new(input::Paste), has_paste)
                    .separator()
                    .menu(t!("Input.Select All"), Box::new(input::SelectAll));

                menu.menu_items = new_menu.menu_items;
                cx.notify();
            });
            this.open = true;

            cx.notify();
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

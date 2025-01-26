use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

use gpui::{
    actions, div, prelude::FluentBuilder, px, Action, AppContext, DismissEvent, EventEmitter,
    FocusHandle, InteractiveElement, IntoElement, KeyBinding, ParentElement, Pixels, Render,
    SharedString, VisualContext as _, Window,
};
use gpui::{
    anchored, canvas, rems, AnyElement, App, Bounds, Context, Corner, Edges, Entity, Focusable,
    Keystroke, ScrollHandle, StatefulInteractiveElement, Styled, WeakEntity,
};

use crate::scroll::{Scrollbar, ScrollbarState};
use crate::StyledExt;
use crate::{
    button::Button, h_flex, list::ListItem, popover::Popover, v_flex, ActiveTheme, Icon, IconName,
    Selectable, Sizable as _,
};

actions!(menu, [Confirm, Dismiss, SelectNext, SelectPrev]);

pub fn init(cx: &mut App) {
    let context = Some("PopupMenu");
    cx.bind_keys([
        KeyBinding::new("enter", Confirm, context),
        KeyBinding::new("escape", Dismiss, context),
        KeyBinding::new("up", SelectPrev, context),
        KeyBinding::new("down", SelectNext, context),
    ]);
}

pub trait PopupMenuExt: Styled + Selectable + IntoElement + 'static {
    /// Create a popup menu with the given items, anchored to the TopLeft corner
    fn popup_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Popover<PopupMenu> {
        self.popup_menu_with_anchor(Corner::TopLeft, f)
    }

    /// Create a popup menu with the given items, anchored to the given corner
    fn popup_menu_with_anchor(
        mut self,
        anchor: impl Into<Corner>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Popover<PopupMenu> {
        let style = self.style().clone();
        let element_id = self.element_id();

        Popover::new(SharedString::from(format!("popup-menu:{:?}", element_id)))
            .no_style()
            .trigger(self)
            .trigger_style(style)
            .anchor(anchor.into())
            .content(move |cx| PopupMenu::build(window, cx, |menu, window, cx| f(menu, window, cx)))
    }
}
impl PopupMenuExt for Button {}

enum PopupMenuItem {
    Separator,
    Item {
        icon: Option<Icon>,
        label: SharedString,
        action: Option<Box<dyn Action>>,
        handler: Rc<dyn Fn(&mut Window, &mut App)>,
    },
    ElementItem {
        render: Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>,
        handler: Rc<dyn Fn(&mut Window, &mut App)>,
    },
    Submenu {
        icon: Option<Icon>,
        label: SharedString,
        menu: Entity<PopupMenu>,
    },
}

impl PopupMenuItem {
    fn is_clickable(&self) -> bool {
        !matches!(self, PopupMenuItem::Separator)
    }

    fn is_separator(&self) -> bool {
        matches!(self, PopupMenuItem::Separator)
    }

    fn has_icon(&self) -> bool {
        matches!(self, PopupMenuItem::Item { icon: Some(_), .. })
    }
}

pub struct PopupMenu {
    /// The parent menu of this menu, if this is a submenu
    parent_menu: Option<WeakEntity<Self>>,
    focus_handle: FocusHandle,
    menu_items: Vec<PopupMenuItem>,
    has_icon: bool,
    selected_index: Option<usize>,
    min_width: Pixels,
    max_width: Pixels,
    max_height: Option<Pixels>,
    hovered_menu_ix: Option<usize>,
    bounds: Bounds<Pixels>,

    scrollable: bool,
    scroll_handle: ScrollHandle,
    scroll_state: Rc<Cell<ScrollbarState>>,

    action_focus_handle: Option<FocusHandle>,
    _subscriptions: [gpui::Subscription; 1],
}

impl PopupMenu {
    pub fn build(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(Self, &mut Window, &mut Context<PopupMenu>) -> Self,
    ) -> Entity<Self> {
        cx.new_view(|cx| {
            let focus_handle = cx.focus_handle();
            let _on_blur_subscription = cx.on_blur(&focus_handle, |this: &mut PopupMenu, cx| {
                this.dismiss(&Dismiss, window, cx)
            });

            let menu = Self {
                focus_handle,
                action_focus_handle: None,
                parent_menu: None,
                menu_items: Vec::new(),
                selected_index: None,
                min_width: px(120.),
                max_width: px(500.),
                max_height: None,
                has_icon: false,
                hovered_menu_ix: None,
                bounds: Bounds::default(),
                scrollable: false,
                scroll_handle: ScrollHandle::default(),
                scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
                _subscriptions: [_on_blur_subscription],
            };
            cx.refresh();
            f(menu, cx)
        })
    }

    /// Bind the focus handle of the menu, when clicked, it will focus back to this handle and then dispatch the action
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.action_focus_handle = Some(focus_handle.clone());
        self
    }

    /// Set min width of the popup menu, default is 120px
    pub fn min_w(mut self, width: impl Into<Pixels>) -> Self {
        self.min_width = width.into();
        self
    }

    /// Set max width of the popup menu, default is 500px
    pub fn max_w(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = width.into();
        self
    }

    /// Set max height of the popup menu, default is half of the window height
    pub fn max_h(mut self, height: impl Into<Pixels>) -> Self {
        self.max_height = Some(height.into());
        self
    }

    /// Set the menu to be scrollable to show vertical scrollbar.
    ///
    /// NOTE: If this is true, the sub-menus will cannot be support.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Add Menu Item
    pub fn menu(mut self, label: impl Into<SharedString>, action: Box<dyn Action>) -> Self {
        self.add_menu_item(label, None, action);
        self
    }

    /// Add Menu to open link
    pub fn link(mut self, label: impl Into<SharedString>, href: impl Into<String>) -> Self {
        let href = href.into();
        self.menu_items.push(PopupMenuItem::Item {
            icon: None,
            label: label.into(),
            action: None,
            handler: Rc::new(move |cx| cx.open_url(&href)),
        });
        self
    }

    /// Add Menu to open link
    pub fn link_with_icon(
        mut self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        href: impl Into<String>,
    ) -> Self {
        let href = href.into();
        self.menu_items.push(PopupMenuItem::Item {
            icon: Some(icon.into()),
            label: label.into(),
            action: None,
            handler: Rc::new(move |cx| cx.open_url(&href)),
        });
        self
    }

    /// Add Menu Item with Icon
    pub fn menu_with_icon(
        mut self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        action: Box<dyn Action>,
    ) -> Self {
        self.add_menu_item(label, Some(icon.into()), action);
        self
    }

    /// Add Menu Item with check icon
    pub fn menu_with_check(
        mut self,
        label: impl Into<SharedString>,
        checked: bool,
        action: Box<dyn Action>,
    ) -> Self {
        if checked {
            self.add_menu_item(label, Some(IconName::Check.into()), action);
        } else {
            self.add_menu_item(label, None, action);
        }

        self
    }

    /// Add Menu Item with custom element render.
    pub fn menu_with_element<F, E>(mut self, builder: F, action: Box<dyn Action>) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.menu_items.push(PopupMenuItem::ElementItem {
            render: Box::new(move |cx| builder(cx).into_any_element()),
            handler: self.wrap_handler(action),
        });
        self
    }

    fn wrap_handler(&self, action: Box<dyn Action>) -> Rc<dyn Fn(&mut Window, &mut App)> {
        let action_focus_handle = self.action_focus_handle.clone();

        Rc::new(move |cx| {
            cx.activate_window();

            // Focus back to the user expected focus handle
            // Then the actions listened on that focus handle can be received
            //
            // For example:
            //
            // TabPanel
            //   |- PopupMenu
            //   |- PanelContent (actions are listened here)
            //
            // The `PopupMenu` and `PanelContent` are at the same level in the TabPanel
            // If the actions are listened on the `PanelContent`,
            // it can't receive the actions from the `PopupMenu`, unless we focus on `PanelContent`.
            if let Some(handle) = action_focus_handle.as_ref() {
                cx.focus(&handle);
            }

            cx.dispatch_action(action.boxed_clone());
        })
    }

    fn add_menu_item(
        &mut self,
        label: impl Into<SharedString>,
        icon: Option<Icon>,
        action: Box<dyn Action>,
    ) -> &mut Self {
        if icon.is_some() {
            self.has_icon = true;
        }

        self.menu_items.push(PopupMenuItem::Item {
            icon,
            label: label.into(),
            action: Some(action.boxed_clone()),
            handler: self.wrap_handler(action),
        });
        self
    }

    /// Add a separator Menu Item
    pub fn separator(mut self) -> Self {
        if self.menu_items.is_empty() {
            return self;
        }

        if let Some(PopupMenuItem::Separator) = self.menu_items.last() {
            return self;
        }

        self.menu_items.push(PopupMenuItem::Separator);
        self
    }

    pub fn submenu(
        self,
        label: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.submenu_with_icon(None, label, cx, window, f)
    }

    /// Add a Submenu item with icon
    pub fn submenu_with_icon(
        mut self,
        icon: Option<Icon>,
        label: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        let submenu = PopupMenu::build(cx, window, f);
        let parent_menu = cx.view().downgrade();
        submenu.update(cx, |view, _| {
            view.parent_menu = Some(parent_menu);
        });

        self.menu_items.push(PopupMenuItem::Submenu {
            icon,
            label: label.into(),
            menu: submenu,
        });
        self
    }

    pub(crate) fn active_submenu(&self) -> Option<Entity<PopupMenu>> {
        if let Some(ix) = self.hovered_menu_ix {
            if let Some(item) = self.menu_items.get(ix) {
                return match item {
                    PopupMenuItem::Submenu { menu, .. } => Some(menu.clone()),
                    _ => None,
                };
            }
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.menu_items.is_empty()
    }

    fn clickable_menu_items(&self) -> impl Iterator<Item = (usize, &PopupMenuItem)> {
        self.menu_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_clickable())
    }

    fn on_click(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        cx.prevent_default();
        self.selected_index = Some(ix);
        self.confirm(&Confirm, window, cx);
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        match self.selected_index {
            Some(index) => {
                let item = self.menu_items.get(index);
                match item {
                    Some(PopupMenuItem::Item { handler, .. }) => {
                        handler(cx);
                        self.dismiss(&Dismiss, window, cx)
                    }
                    Some(PopupMenuItem::ElementItem { handler, .. }) => {
                        handler(cx);
                        self.dismiss(&Dismiss, window, cx)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn select_next(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.clickable_menu_items().count();
        if count > 0 {
            let last_ix = count.saturating_sub(1);
            let ix = self
                .selected_index
                .map(|index| if index == last_ix { 0 } else { index + 1 })
                .unwrap_or(0);

            self.selected_index = Some(ix);
            cx.notify();
        }
    }

    fn select_prev(&mut self, _: &SelectPrev, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.clickable_menu_items().count();
        if count > 0 {
            let last_ix = count.saturating_sub(1);

            let ix = self
                .selected_index
                .map(|index| {
                    if index == last_ix {
                        0
                    } else {
                        index.saturating_sub(1)
                    }
                })
                .unwrap_or(last_ix);
            self.selected_index = Some(ix);
            cx.notify();
        }
    }

    fn dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_submenu().is_some() {
            return;
        }

        cx.emit(DismissEvent);
        // Dismiss parent menu, when this menu is dismissed
        if let Some(parent_menu) = self.parent_menu.clone().and_then(|menu| menu.upgrade()) {
            parent_menu.update(cx, |view, cx| {
                view.hovered_menu_ix = None;
                view.dismiss(&Dismiss, window, cx);
            })
        }
    }

    fn render_keybinding(
        action: Option<Box<dyn Action>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        if let Some(action) = action {
            if let Some(keybinding) = cx.bindings_for_action(action.deref()).first() {
                let el = div().text_color(cx.theme().muted_foreground).children(
                    keybinding
                        .keystrokes()
                        .into_iter()
                        .map(|key| key_shortcut(key.clone())),
                );

                return Some(el);
            }
        }

        return None;
    }

    fn render_icon(
        has_icon: bool,
        icon: Option<Icon>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let icon_placeholder = if has_icon { Some(Icon::empty()) } else { None };

        if !has_icon {
            return None;
        }

        let icon = h_flex()
            .w_3p5()
            .h_3p5()
            .justify_center()
            .text_sm()
            .map(|this| {
                if let Some(icon) = icon {
                    this.child(icon.clone().xsmall())
                } else {
                    this.children(icon_placeholder.clone())
                }
            });

        Some(icon)
    }
}

impl FluentBuilder for PopupMenu {}
impl EventEmitter<DismissEvent> for PopupMenu {}
impl Focusable for PopupMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopupMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.view().clone();
        let has_icon = self.menu_items.iter().any(|item| item.has_icon());
        let items_count = self.menu_items.len();
        let max_width = self.max_width;
        let bounds = self.bounds;
        let max_height = self.max_height.map_or_else(
            || {
                let window_half_height = cx.window_bounds().get_bounds().size.height * 0.5;
                window_half_height.min(px(450.))
            },
            |height| height,
        );

        const ITEM_HEIGHT: Pixels = px(26.);

        v_flex()
            .id("popup-menu")
            .key_context("PopupMenu")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_prev))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::dismiss))
            .on_mouse_down_out(cx.listener(|this, _, cx| this.dismiss(&Dismiss, cx)))
            .popover_style(cx)
            .text_color(cx.theme().popover_foreground)
            .relative()
            .p_1()
            .child(
                div()
                    .id("popup-menu-items")
                    .when(self.scrollable, |this| {
                        this.max_h(max_height)
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                    })
                    .child(
                        v_flex()
                            .gap_y_0p5()
                            .min_w(self.min_width)
                            .max_w(self.max_width)
                            .min_w(rems(8.))
                            .child({
                                canvas(
                                    move |bounds, window, cx| {
                                        view.update(cx, |r, _| r.bounds = bounds)
                                    },
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .size_full()
                            })
                            .children(
                                self.menu_items
                                    .iter_mut()
                                    .enumerate()
                                    // Skip last separator
                                    .filter(|(ix, item)| {
                                        !(*ix == items_count - 1 && item.is_separator())
                                    })
                                    .map(|(ix, item)| {
                                        let this = ListItem::new(("menu-item", ix))
                                            .relative()
                                            .text_sm()
                                            .py_0()
                                            .px_1()
                                            .rounded_md()
                                            .items_center()
                                            .on_mouse_enter(cx.listener(move |this, _, cx| {
                                                this.hovered_menu_ix = Some(ix);
                                                cx.notify();
                                            }));

                                        match item {
                                            PopupMenuItem::Separator => {
                                                this.h_auto().p_0().disabled(true).child(
                                                    div()
                                                        .rounded_none()
                                                        .h(px(1.))
                                                        .mx_neg_1()
                                                        .my_0p5()
                                                        .bg(cx.theme().muted),
                                                )
                                            }
                                            PopupMenuItem::ElementItem { render, .. } => this
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.on_click(ix, window, cx)
                                                    },
                                                ))
                                                .child(
                                                    h_flex()
                                                        .min_h(ITEM_HEIGHT)
                                                        .items_center()
                                                        .gap_x_1()
                                                        .children(Self::render_icon(
                                                            has_icon, None, window, cx,
                                                        ))
                                                        .child((render)(cx)),
                                                ),
                                            PopupMenuItem::Item {
                                                icon,
                                                label,
                                                action,
                                                ..
                                            } => {
                                                let action = action
                                                    .as_ref()
                                                    .map(|action| action.boxed_clone());
                                                let key =
                                                    Self::render_keybinding(action, window, cx);

                                                this.on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.on_click(ix, window, cx)
                                                    },
                                                ))
                                                .child(
                                                    h_flex()
                                                        .h(ITEM_HEIGHT)
                                                        .items_center()
                                                        .gap_x_1()
                                                        .children(Self::render_icon(
                                                            has_icon,
                                                            icon.clone(),
                                                            window,
                                                            cx,
                                                        ))
                                                        .child(
                                                            h_flex()
                                                                .flex_1()
                                                                .gap_2()
                                                                .items_center()
                                                                .justify_between()
                                                                .child(label.clone())
                                                                .children(key),
                                                        ),
                                                )
                                            }
                                            PopupMenuItem::Submenu { icon, label, menu } => this
                                                .when(self.hovered_menu_ix == Some(ix), |this| {
                                                    this.selected(true)
                                                })
                                                .child(
                                                    h_flex()
                                                        .items_start()
                                                        .child(
                                                            h_flex()
                                                                .size_full()
                                                                .items_center()
                                                                .gap_x_1()
                                                                .children(Self::render_icon(
                                                                    has_icon,
                                                                    icon.clone(),
                                                                    window,
                                                                    cx,
                                                                ))
                                                                .child(
                                                                    h_flex()
                                                                        .flex_1()
                                                                        .gap_2()
                                                                        .items_center()
                                                                        .justify_between()
                                                                        .child(label.clone())
                                                                        .child(
                                                                            IconName::ChevronRight,
                                                                        ),
                                                                ),
                                                        )
                                                        .when_some(
                                                            self.hovered_menu_ix,
                                                            |this, hovered_ix| {
                                                                let (anchor, left) =
                                                                    if window.bounds().size.width
                                                                        - bounds.origin.x
                                                                        < max_width
                                                                    {
                                                                        (Corner::TopRight, -px(15.))
                                                                    } else {
                                                                        (
                                                                            Corner::TopLeft,
                                                                            bounds.size.width
                                                                                - px(10.),
                                                                        )
                                                                    };

                                                                let top = if bounds.origin.y
                                                                    + bounds.size.height
                                                                    > window.bounds().size.height
                                                                {
                                                                    px(32.)
                                                                } else {
                                                                    -px(10.)
                                                                };

                                                                if hovered_ix == ix {
                                                                    this.child(
                                                                anchored()
                                                                    .anchor(anchor)
                                                                    .child(
                                                                        div()
                                                                            .occlude()
                                                                            .top(top)
                                                                            .left(left)
                                                                            .child(menu.clone()),
                                                                    )
                                                                    .snap_to_window_with_margin(
                                                                        Edges::all(px(8.)),
                                                                    ),
                                                            )
                                                                } else {
                                                                    this
                                                                }
                                                            },
                                                        ),
                                                ),
                                        }
                                    }),
                            ),
                    ),
            )
            .when(self.scrollable, |this| {
                // TODO: When the menu is limited by `overflow_y_scroll`, the sub-menu will cannot be displayed.
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0p5()
                        .bottom_0()
                        .child(Scrollbar::vertical(
                            cx.entity_id(),
                            self.scroll_state.clone(),
                            self.scroll_handle.clone(),
                            self.bounds.size,
                        )),
                )
            })
    }
}

/// Return the Platform specific keybinding string by KeyStroke
pub fn key_shortcut(key: Keystroke) -> String {
    if cfg!(target_os = "macos") {
        return format!("{}", key);
    }

    let mut parts = vec![];
    if key.modifiers.control {
        parts.push("Ctrl");
    }
    if key.modifiers.alt {
        parts.push("Alt");
    }
    if key.modifiers.platform {
        parts.push("Win");
    }
    if key.modifiers.shift {
        parts.push("Shift");
    }

    // Capitalize the first letter
    let key = if let Some(first_c) = key.key.chars().next() {
        format!("{}{}", first_c.to_uppercase(), &key.key[1..])
    } else {
        key.key.to_string()
    };

    parts.push(&key);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_key_shortcut() {
        use super::key_shortcut;
        use gpui::Keystroke;

        if cfg!(target_os = "windows") {
            assert_eq!(key_shortcut(Keystroke::parse("a").unwrap()), "A");
            assert_eq!(key_shortcut(Keystroke::parse("ctrl-a").unwrap()), "Ctrl+A");
            assert_eq!(
                key_shortcut(Keystroke::parse("ctrl-alt-a").unwrap()),
                "Ctrl+Alt+A"
            );
            assert_eq!(
                key_shortcut(Keystroke::parse("ctrl-alt-shift-a").unwrap()),
                "Ctrl+Alt+Shift+A"
            );
            assert_eq!(
                key_shortcut(Keystroke::parse("ctrl-alt-shift-win-a").unwrap()),
                "Ctrl+Alt+Win+Shift+A"
            );
            assert_eq!(
                key_shortcut(Keystroke::parse("ctrl-shift-backspace").unwrap()),
                "Ctrl+Shift+Backspace"
            );
        }
    }
}

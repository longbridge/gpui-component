use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Action, App, AppContext, Bounds, ClickEvent, Context, Div, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement as _, Pixels,
    Point, Render, SharedString, Styled as _, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, ElementExt, button::Button, native_menu::NativeMenu, v_flex,
};
use serde::Deserialize;

use crate::section;

/// Dispatched by every native menu item; the payload is the item label so the
/// story can report which item was selected.
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = native_menu_story, no_json)]
struct MenuClick(SharedString);

const CONTEXT: &str = "NativeMenuStory";

pub struct NativeMenuStory {
    focus_handle: FocusHandle,
    message: String,
}

impl super::Story for NativeMenuStory {
    fn title() -> &'static str {
        "NativeMenu"
    }

    fn description() -> &'static str {
        "A menu rendered by the operating system. Unlike `PopupMenu`, it is drawn \
        by the OS and can extend beyond the window bounds — useful for small windows."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl NativeMenuStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            message: String::new(),
        }
    }

    fn on_click(&mut self, click: &MenuClick, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("Selected: {}", click.0);
        cx.notify();
    }

    fn trigger(&self, label: &str, cx: &mut App) -> Div {
        // A bordered box that opens a native menu where the user right-clicks.
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_full()
            .h_24()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg()
            .text_color(cx.theme().muted_foreground)
            .child(SharedString::from(label.to_string()))
    }
}

impl Focusable for NativeMenuStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NativeMenuStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let result = if self.message.is_empty() {
            "Right-click a box above to open a native menu.".to_string()
        } else {
            self.message.clone()
        };
        let focus_handle = self.focus_handle.clone();

        v_flex()
            .track_focus(&self.focus_handle)
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_click))
            .size_full()
            .gap_6()
            .child(
                section("Builder API").child(
                    self.trigger("Right-click here", cx).on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            // Focus the story so the dispatched action reaches `on_click`.
                            this.focus_handle.focus(window, cx);
                            NativeMenu::new()
                                .menu("Cut", Box::new(MenuClick("Cut".into())))
                                .menu("Copy", Box::new(MenuClick("Copy".into())))
                                .menu("Paste", Box::new(MenuClick("Paste".into())))
                                .separator()
                                .menu_with_disabled(
                                    "Disabled",
                                    true,
                                    Box::new(MenuClick("Disabled".into())),
                                )
                                .menu_with_check(
                                    "Word Wrap",
                                    true,
                                    Box::new(MenuClick("Word Wrap".into())),
                                )
                                .separator()
                                .menu("Select All", Box::new(MenuClick("Select All".into())))
                                // Nudge right so the cursor doesn't land on the first item.
                                .popup(
                                    Point {
                                        x: ev.position.x + px(4.),
                                        y: ev.position.y,
                                    },
                                    window,
                                    cx,
                                );
                        }),
                    ),
                ),
            )
            .child(
                section("From gpui::Menu items").child(
                    self.trigger("Right-click here", cx).on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            this.focus_handle.focus(window, cx);
                            // Reuse a GPUI menu definition directly as a native menu.
                            NativeMenu::from_menu_items([
                                gpui::MenuItem::action("Copy", MenuClick("Copy".into())),
                                gpui::MenuItem::separator(),
                                gpui::MenuItem::action("Paste", MenuClick("Paste".into())),
                            ])
                            .popup(ev.position, window, cx);
                        }),
                    ),
                ),
            )
            .child(
                section("Dropdown (click to open)").child({
                    // A native menu isn't limited to right-click — `popup` takes
                    // any window position. Capture the trigger's bounds so the
                    // menu opens at its bottom-left, like a real dropdown.
                    let trigger_bounds: Rc<Cell<Bounds<Pixels>>> =
                        Rc::new(Cell::new(Bounds::default()));
                    let bounds_writer = trigger_bounds.clone();

                    div()
                        .on_prepaint(move |bounds, _, _| bounds_writer.set(bounds))
                        .child(Button::new("native-dropdown").outline().label("Open Menu").on_click(
                            move |_: &ClickEvent, window, cx| {
                                let bounds = trigger_bounds.get();
                                let position = Point {
                                    x: bounds.origin.x,
                                    // Just below the button. NSMenu's frame has a
                                    // little top padding, so add a small gap to keep
                                    // the menu from overlapping the button.
                                    y: bounds.origin.y + bounds.size.height + px(8.),
                                };
                                // Focus the story so the dispatched action is handled.
                                focus_handle.focus(window, cx);
                                NativeMenu::new()
                                    .menu("New File", Box::new(MenuClick("New File".into())))
                                    .menu("Open…", Box::new(MenuClick("Open".into())))
                                    .separator()
                                    .menu("Save", Box::new(MenuClick("Save".into())))
                                    .popup(position, window, cx);
                            },
                        ))
                }),
            )
            .child(section("Result").child(SharedString::from(result)))
    }
}

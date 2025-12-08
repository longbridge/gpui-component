use gpui::{
    Action, App, AppContext, Context, Corner, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement as _,
    Render, Styled as _, Window, actions, div, px,
};
use gpui_component::{
    ActiveTheme, StyledExt, WindowExt,
    button::{Button, ButtonVariants as _},
    divider::Divider,
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    popover::Popover,
    v_flex,
};
use serde::Deserialize;

use crate::section;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = popover_story, no_json)]
struct Info(usize);

actions!(popover_story, [Copy, Paste, Cut, SearchAll, ToggleCheck]);
const CONTEXT: &str = "popover-story";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, Some(CONTEXT)),
    ])
}

struct Form {
    parent: Entity<PopoverStory>,
    input1: Entity<InputState>,
}

impl Form {
    fn new(parent: Entity<PopoverStory>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            parent,
            input1: cx.new(|cx| InputState::new(window, cx)),
        })
    }
}

impl Focusable for Form {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input1.focus_handle(cx)
    }
}

struct DropdownListDelegate {
    parent: Entity<PopoverStory>,
}
impl ListDelegate for DropdownListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        10
    }

    fn render_item(
        &mut self,
        ix: gpui_component::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        Some(ListItem::new(ix).child(format!("Item {}", ix.row)))
    }

    fn set_selected_index(
        &mut self,
        _: Option<gpui_component::IndexPath>,
        _: &mut Window,
        _: &mut Context<gpui_component::list::ListState<Self>>,
    ) {
    }

    fn confirm(&mut self, _: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        })
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        })
    }
}

impl EventEmitter<DismissEvent> for Form {}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let parent = self.parent.clone();
        v_flex()
            .gap_2()
            .p_3()
            .size_full()
            .child("This is a form container.")
            .child("Click submit to dismiss the popover.")
            .child(Input::new(&self.input1))
            .child(
                Button::new("submit")
                    .label("Submit")
                    .primary()
                    .on_click(cx.listener(move |_, _, _, cx| {
                        parent.update(cx, |this, cx| {
                            this.form_popover_open = false;
                            cx.notify();
                        })
                    })),
            )
    }
}

pub struct PopoverStory {
    focus_handle: FocusHandle,
    form: Entity<Form>,
    list: Entity<ListState<DropdownListDelegate>>,
    form_popover_open: bool,
    list_popover_open: bool,
    checked: bool,
    message: String,
}

impl super::Story for PopoverStory {
    fn title() -> &'static str {
        "Popover"
    }

    fn description() -> &'static str {
        "A popup displays content on top of the main page."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PopoverStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form = Form::new(cx.entity(), window, cx);
        let parent = cx.entity();
        let list = cx
            .new(|cx| ListState::new(DropdownListDelegate { parent }, window, cx).searchable(true));

        cx.focus_self(window);

        Self {
            form,
            list,
            checked: true,
            form_popover_open: false,
            list_popover_open: false,
            focus_handle: cx.focus_handle(),
            message: "".to_string(),
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked copy".to_string();
        cx.notify()
    }

    fn on_cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked cut".to_string();
        cx.notify()
    }

    fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked paste".to_string();
        cx.notify()
    }

    fn on_search_all(&mut self, _: &SearchAll, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked search all".to_string();
        cx.notify()
    }

    fn on_action_info(&mut self, info: &Info, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("You have clicked info: {}", info.0);
        cx.notify()
    }

    fn on_action_toggle_check(&mut self, _: &ToggleCheck, _: &mut Window, cx: &mut Context<Self>) {
        self.checked = !self.checked;
        self.message = format!("You have clicked toggle check: {}", self.checked);
        cx.notify()
    }
}

impl Focusable for PopoverStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.clone();

        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_action_info))
            .on_action(cx.listener(Self::on_action_toggle_check))
            .size_full()
            .gap_6()
            .child(
                section("Basic Popover").child(
                    Popover::new("popover-0")
                        .max_w(px(600.))
                        .trigger(Button::new("btn").outline().label("Popover"))
                        .gap_2()
                        .text_sm()
                        .w(px(400.))
                        .child("Hello, this is a Popover.")
                        .child(Divider::horizontal())
                        .child(
                            "You can put any content here, including text,\
                            buttons, forms, and more.",
                        ),
                ),
            )
            .child(
                section("Popover with Form").child(
                    Popover::new("popover-form")
                        .p_0()
                        .text_sm()
                        .trigger(Button::new("pop").outline().label("Popup Form"))
                        .track_focus(&form.focus_handle(cx))
                        .open(self.form_popover_open)
                        .on_open_change(cx.listener(move |this, open, _, cx| {
                            println!("Popover form open changed: {}", open);
                            this.form_popover_open = *open;
                            cx.notify();
                        }))
                        .child(form.clone()),
                ),
            )
            .child(
                section("Popover with List").child(
                    Popover::new("popover-list")
                        .p_0()
                        .text_sm()
                        .open(self.list_popover_open)
                        .on_open_change(cx.listener(move |this, open, _, cx| {
                            this.list_popover_open = *open;
                            cx.notify();
                        }))
                        .trigger(Button::new("pop").outline().label("Popup List"))
                        .track_focus(&self.list.focus_handle(cx))
                        .child(List::new(&self.list))
                        .w_64()
                        .h(px(200.)),
                ),
            )
            .child(
                section("Right click to open Popover").child(
                    Popover::new("popover-right-click")
                        .mouse_button(MouseButton::Right)
                        .trigger(Button::new("btn").outline().label("Right Click Popover"))
                        .max_w(px(600.))
                        .content(|_, _, cx| {
                            v_flex()
                                .gap_2()
                                .child("Hello, this is a Popover on the Bottom Right.")
                                .child(Divider::horizontal())
                                .child(
                                    Button::new("info1")
                                        .primary()
                                        .label("Dismiss")
                                        .w(px(80.))
                                        .on_click(cx.listener(|_, _, window, cx| {
                                            window.push_notification(
                                                "You have clicked dismiss via DismissEvent.",
                                                cx,
                                            );
                                            cx.emit(DismissEvent);
                                        })),
                                )
                        }),
                ),
            )
            .child(
                section("Styling Popover").child(
                    Popover::new("popover-1")
                        .trigger(Button::new("btn").outline().label("Style Popover"))
                        .appearance(false)
                        .py_1()
                        .px_2()
                        .bg(cx.theme().primary)
                        .text_color(cx.theme().primary_foreground)
                        .max_w(px(600.))
                        .rounded_sm()
                        .text_sm()
                        .shadow_2xl()
                        .child("A styled Popover with custom background and text color."),
                ),
            )
            .child(
                section("Default Open").child(
                    Popover::new("default-open-popover")
                        .default_open(true)
                        .trigger(
                            Button::new("default-open-btn")
                                .label("Default Open")
                                .outline(),
                        )
                        .child("This popover is open by default when first rendered."),
                ),
            )
            .child(
                section("Popover Anchor")
                    .min_h(px(320.))
                    .v_flex()
                    .child(
                        div().absolute().top_0().left_0().w_full().h_10().child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    Popover::new("anchor0")
                                        .max_w(px(600.))
                                        .trigger(Button::new("btn").outline().label("TopLeft"))
                                        .child("This is a Popover on the Top Left."),
                                )
                                .child(
                                    Popover::new("anchor1")
                                        .anchor(Corner::TopRight)
                                        .trigger(Button::new("btn").outline().label("TopRight"))
                                        .child("This is a Popover on the Top Right."),
                                ),
                        ),
                    )
                    .child(
                        div().absolute().bottom_0().left_0().w_full().h_10().child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    Popover::new("anchor2")
                                        .trigger(Button::new("btn").outline().label("BottomLeft"))
                                        .anchor(Corner::BottomLeft)
                                        .child("This is a Popover on the Bottom Left."),
                                )
                                .child(
                                    Popover::new("anchor3")
                                        .anchor(Corner::BottomRight)
                                        .trigger(Button::new("btn").outline().label("BottomRight"))
                                        .child("This is a Popover on the Bottom Right."),
                                ),
                        ),
                    ),
            )
    }
}

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Action, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Pixels, Render, SharedString, Styled as _,
    Window, div, img, px, size,
};
use gpui_component::{
    ActiveTheme as _, ElementExt as _, IconName, StyledExt as _,
    avatar::Avatar,
    button::*,
    h_flex,
    input::{Input, InputState},
    native_popover::{self, NativePopover},
    switch::Switch,
    v_flex,
};
use serde::Deserialize;

use crate::section;

/// Dispatched by every popover button; payload is the button label so the story
/// can report which one was clicked.
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = native_popover_story, no_json)]
struct PopoverClick(SharedString);

const CONTEXT: &str = "NativePopoverStory";

/// A button dispatching `PopoverClick(label)`.
fn click(label: &str) -> Box<dyn gpui::Action> {
    Box::new(PopoverClick(label.to_string().into()))
}

/// SPIKE content: arbitrary GPUI rendered inside a native `NSPopover` (via
/// `native_popover::show_view` reparenting). The counter button proves both
/// rendering and interaction work after reparenting.
struct SpikeContent {
    count: usize,
    enabled: bool,
    input: Entity<InputState>,
}

impl SpikeContent {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Type inside a native popover…"));
        Self {
            count: 0,
            enabled: true,
            input,
        }
    }
}

impl Render for SpikeContent {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .p_4()
            .gap_4()
            // Header: avatar + title / subtitle.
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Avatar::new().name("GP"))
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_bold().child("Arbitrary GPUI content"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Real GPUI widgets in a native NSPopover"),
                            ),
                    ),
            )
            // A real (bitmap) image, centered.
            .child(
                h_flex().justify_center().child(
                    img("https://avatars.githubusercontent.com/u/5518?v=4")
                        .size_20()
                        .rounded_lg(),
                ),
            )
            // A row of icons.
            .child(
                h_flex()
                    .gap_4()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground)
                    .child(IconName::Star)
                    .child(IconName::Heart)
                    .child(IconName::Bell)
                    .child(IconName::Calendar)
                    .child(IconName::Github),
            )
            // A text input — verifies keyboard focus works inside the popover.
            .child(Input::new(&self.input))
            // An interactive switch.
            .child(
                Switch::new("spike-switch")
                    .checked(self.enabled)
                    .label("Enable feature")
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.enabled = *checked;
                        cx.notify();
                    })),
            )
            // Buttons with live state.
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("spike-inc")
                            .primary()
                            .label(SharedString::from(format!("Count: {}", self.count)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("spike-reset")
                            .outline()
                            .label("Reset")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count = 0;
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub struct NativePopoverStory {
    focus_handle: FocusHandle,
    message: String,
    /// Persisted across popover open/close so its state (the counter) survives.
    spike_content: Entity<SpikeContent>,
}

impl super::Story for NativePopoverStory {
    fn title() -> &'static str {
        "NativePopover"
    }

    fn description() -> &'static str {
        "A popover rendered natively by the OS (macOS `NSPopover`): system arrow, vibrant \
        backdrop, show/dismiss animation, and transient behavior (click outside to dismiss). \
        It can extend beyond the window. Content is native (a title and buttons), so it carries \
        GPUI actions rather than arbitrary GPUI views."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl NativePopoverStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            message: String::new(),
            spike_content: cx.new(|cx| SpikeContent::new(window, cx)),
        }
    }

    fn on_click(&mut self, click: &PopoverClick, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("Clicked: {}", click.0);
        cx.notify();
    }

    /// A trigger button that captures its own bounds (so the popover can anchor
    /// to it) and opens a native popover below it on click.
    fn trigger(&self, id: &'static str, label: &'static str) -> impl IntoElement {
        let bounds: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
        let writer = bounds.clone();
        let focus_handle = self.focus_handle.clone();

        div().on_prepaint(move |b, _, _| writer.set(b)).child(
            Button::new(id)
                .outline()
                .label(label)
                .on_click(move |_: &ClickEvent, window, cx| {
                    // Focus the story so the dispatched action reaches `on_click`.
                    focus_handle.focus(window, cx);
                    NativePopover::new()
                        .title("Quick actions")
                        .button("Duplicate", click("Duplicate"))
                        .button("Rename", click("Rename"))
                        .button("Delete", click("Delete"))
                        .show(bounds.get(), window, cx);
                }),
        )
    }

    /// SPIKE trigger: open a native popover whose content is arbitrary GPUI.
    /// Reuses the persisted `spike_content` entity so its counter survives
    /// across open/close.
    fn spike_trigger(&self) -> impl IntoElement {
        let bounds: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
        let writer = bounds.clone();
        let content = self.spike_content.clone();

        div().on_prepaint(move |b, _, _| writer.set(b)).child(
            Button::new("spike")
                .outline()
                .label("Open GPUI-content popover (spike)")
                .on_click(move |_: &ClickEvent, window, cx| {
                    let content = content.clone();
                    native_popover::show_view(
                        bounds.get(),
                        size(px(320.), px(320.)),
                        window,
                        cx,
                        move |_, _| content,
                    );
                }),
        )
    }
}

impl Focusable for NativePopoverStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NativePopoverStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let result = if self.message.is_empty() {
            "Click a trigger to open a native popover; click outside to dismiss.".to_string()
        } else {
            self.message.clone()
        };

        v_flex()
            .track_focus(&self.focus_handle)
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_click))
            .size_full()
            .gap_6()
            .child(
                section("SPIKE: arbitrary GPUI content (reparented into NSPopover)")
                    .child(self.spike_trigger()),
            )
            .child(section("Click to open").child(self.trigger("open-1", "Open Popover")))
            .child(
                section("Near the window edge (proves it overflows the window)")
                    .child(self.trigger("open-2", "Open at edge")),
            )
            .child(section("Result").child(SharedString::from(result)))
    }
}

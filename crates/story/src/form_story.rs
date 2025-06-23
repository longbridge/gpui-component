use gpui::{
    div, prelude::FluentBuilder as _, App, AppContext, Axis, Context, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    color_picker::{ColorPicker, ColorPickerState},
    date_picker::{DatePicker, DatePickerState},
    divider::Divider,
    form::{form_field, v_form},
    h_flex,
    input::{InputState, TextInput},
    switch::Switch,
    v_flex, AxisExt, Selectable, Sizable, Size,
};

pub struct FormStory {
    focus_handle: gpui::FocusHandle,
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
    bio_input: Entity<InputState>,
    color_state: Entity<ColorPickerState>,
    subscribe_email: bool,
    date: Entity<DatePickerState>,
    layout: Axis,
    size: Size,
}

impl super::Story for FormStory {
    fn title() -> &'static str {
        "Form"
    }

    fn description() -> &'static str {
        "Form to collect multiple inputs."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl FormStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).default_value("Jason Lee"));
        let color_state = cx.new(|cx| ColorPickerState::new(window, cx));

        let email_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter text here..."));
        let bio_input = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(5, 20)
                .placeholder("Enter text here...")
                .default_value("Hello 世界，this is GPUI component.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            name_input,
            email_input,
            bio_input,
            date,
            color_state,
            subscribe_email: false,
            layout: Axis::Vertical,
            size: Size::default(),
        }
    }
}

impl Focusable for FormStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FormStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("form-story")
            .size_full()
            .p_4()
            .justify_start()
            .gap_3()
            .child(
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .justify_between()
                    .child(
                        Switch::new("layout")
                            .checked(self.layout.is_horizontal())
                            .label("Horizontal")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                if *checked {
                                    this.layout = Axis::Horizontal;
                                } else {
                                    this.layout = Axis::Vertical;
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        ButtonGroup::new("size")
                            .small()
                            .child(
                                Button::new("large")
                                    .selected(self.size == Size::Large)
                                    .child("Large"),
                            )
                            .child(
                                Button::new("medium")
                                    .child("Medium")
                                    .selected(self.size == Size::Medium),
                            )
                            .child(
                                Button::new("small")
                                    .child("Small")
                                    .selected(self.size == Size::Small),
                            )
                            .on_click(cx.listener(|this, selecteds: &Vec<usize>, _, cx| {
                                if selecteds.contains(&0) {
                                    this.size = Size::Large;
                                } else if selecteds.contains(&1) {
                                    this.size = Size::Medium;
                                } else if selecteds.contains(&2) {
                                    this.size = Size::Small;
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(Divider::horizontal())
            .child(
                v_form()
                    .layout(self.layout)
                    .with_size(self.size)
                    .child(
                        form_field()
                            .label_fn(|_, _| "Name")
                            .child(TextInput::new(&self.name_input)),
                    )
                    .child(
                        form_field()
                            .label("Email")
                            .child(TextInput::new(&self.email_input))
                            .required(true),
                    )
                    .child(
                        form_field()
                            .label("Bio")
                            .when(self.layout.is_vertical(), |this| this.items_start())
                            .child(TextInput::new(&self.bio_input))
                            .description_fn(|_, _| {
                                div().child("Use at most 100 words to describe yourself.")
                            }),
                    )
                    .child(
                        form_field()
                            .no_label_indent()
                            .child("This is a full width form field."),
                    )
                    .child(
                        form_field()
                            .label("Birthday")
                            .child(DatePicker::new(&self.date))
                            .description("Select your birthday, we will send you a gift."),
                    )
                    .child(
                        form_field().child(
                            Switch::new("subscribe-newsletter")
                                .label("Subscribe our newsletter")
                                .checked(self.subscribe_email)
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.subscribe_email = *checked;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        form_field().child(
                            ColorPicker::new(&self.color_state)
                                .small()
                                .label("Theme color"),
                        ),
                    )
                    .child(
                        form_field().child(
                            Checkbox::new("use-vertical-layout")
                                .label("Vertical layout")
                                .checked(self.layout.is_vertical())
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.layout = if *checked {
                                        Axis::Vertical
                                    } else {
                                        Axis::Horizontal
                                    };
                                    cx.notify();
                                })),
                        ),
                    ),
            )
    }
}

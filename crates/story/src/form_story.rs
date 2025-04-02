use gpui::{
    actions, div, prelude::FluentBuilder as _, App, AppContext, Axis, Context, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonGroup, ButtonVariants},
    checkbox::Checkbox,
    color_picker::ColorPicker,
    date_picker::DatePicker,
    divider::Divider,
    form::{form_field, v_form},
    h_flex,
    input::TextInput,
    switch::Switch,
    v_flex, AxisExt, FocusableCycle, Selectable, Sizable, Size,
};

actions!(input_story, [Tab, TabPrev]);

pub struct FormStory {
    name_input: Entity<TextInput>,
    email_input: Entity<TextInput>,
    bio_input: Entity<TextInput>,
    color_picker: Entity<ColorPicker>,
    subscribe_email: bool,
    date_picker: Entity<DatePicker>,
    layout: Axis,
    size: Size,
}

impl super::Story for FormStory {
    fn title() -> &'static str {
        "FormStory"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl FormStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).cleanable();
            input.set_text("Jason Lee", window, cx);
            input
        });
        let color_picker = cx.new(|cx| {
            ColorPicker::new("color-picker-1", window, cx)
                .small()
                .label("Theme color")
        });

        let email_input = cx.new(|cx| TextInput::new(window, cx).placeholder("Enter text here..."));
        let bio_input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx)
                .multi_line()
                .rows(10)
                .placeholder("Enter text here...");
            input.set_text("Hello 世界，this is GPUI component.", window, cx);
            input
        });
        let date_picker = cx.new(|cx| DatePicker::new("birthday", window, cx));

        Self {
            name_input,
            email_input,
            bio_input,
            date_picker,
            color_picker,
            subscribe_email: false,
            layout: Axis::Vertical,
            size: Size::default(),
        }
    }
}

impl FocusableCycle for FormStory {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<gpui::FocusHandle>
    where
        Self: Sized,
    {
        vec![
            self.name_input.focus_handle(cx),
            self.email_input.focus_handle(cx),
            self.bio_input.focus_handle(cx),
        ]
    }
}

impl Focusable for FormStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.name_input.focus_handle(cx)
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
                            .outline()
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
                            .child(self.name_input.clone()),
                    )
                    .child(
                        form_field()
                            .label("Email")
                            .child(self.email_input.clone())
                            .required(true),
                    )
                    .child(
                        form_field()
                            .label("Bio")
                            .when(self.layout.is_vertical(), |this| this.items_start())
                            .child(self.bio_input.clone())
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
                            .child(self.date_picker.clone())
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
                    .child(form_field().child(self.color_picker.clone()))
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

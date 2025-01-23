use gpui::{
    actions, Axis, InteractiveElement, IntoElement, ParentElement as _, Render, Styled, View,
    ViewContext, VisualContext, WindowContext,
};
use ui::{
    date_picker::DatePicker,
    form::{form_field, v_form},
    input::TextInput,
    prelude::FluentBuilder as _,
    switch::Switch,
    v_flex, AxisExt, FocusableCycle,
};

actions!(input_story, [Tab, TabPrev]);

pub struct FormStory {
    name_input: View<TextInput>,
    email_input: View<TextInput>,
    bio_input: View<TextInput>,
    subscribe_email: bool,
    date_picker: View<DatePicker>,
    layout: Axis,
}

impl super::Story for FormStory {
    fn title() -> &'static str {
        "FormStory"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl FormStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    fn new(cx: &mut ViewContext<Self>) -> Self {
        let name_input = cx.new_view(|cx| {
            let mut input = TextInput::new(cx).cleanable();
            input.set_text("Jason Lee", cx);
            input
        });

        let email_input = cx.new_view(|cx| TextInput::new(cx).placeholder("Enter text here..."));
        let bio_input = cx.new_view(|cx| {
            let mut input = TextInput::new(cx)
                .multi_line()
                .rows(10)
                .placeholder("Enter text here...");
            input.set_text("Hello 世界，this is GPUI component.", cx);
            input
        });
        let date_picker = cx.new_view(|cx| DatePicker::new("birthday", cx));

        Self {
            name_input,
            email_input,
            bio_input,
            date_picker,
            subscribe_email: false,
            layout: Axis::Vertical,
        }
    }
}

impl FocusableCycle for FormStory {
    fn cycle_focus_handles(&self, cx: &mut WindowContext) -> Vec<gpui::FocusHandle>
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

impl gpui::FocusableView for FormStory {
    fn focus_handle(&self, cx: &gpui::AppContext) -> gpui::FocusHandle {
        self.name_input.focus_handle(cx)
    }
}

impl Render for FormStory {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .id("form-story")
            .size_full()
            .p_4()
            .justify_start()
            .gap_3()
            .child(
                Switch::new("layout")
                    .checked(self.layout.is_horizontal())
                    .label("Horizontal")
                    .on_click(cx.listener(|this, checked: &bool, cx| {
                        if *checked {
                            this.layout = Axis::Horizontal;
                        } else {
                            this.layout = Axis::Vertical;
                        }
                        cx.notify();
                    })),
            )
            .child(
                v_form()
                    .layout(self.layout)
                    .child(form_field().label("Name").child(self.name_input.clone()))
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
                            .description("Use at most 100 words to describe yourself."),
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
                                .on_click(cx.listener(|this, checked: &bool, cx| {
                                    this.subscribe_email = *checked;
                                    cx.notify();
                                })),
                        ),
                    ),
            )
    }
}

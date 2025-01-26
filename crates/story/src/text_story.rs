use gpui::{Window, ModelContext, AppContext, Model, 
    div, px, rems, IntoElement, ParentElement, Render, SharedString, Styled,  
    VisualContext as _, 
};

use ui::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    label::Label,
    link::Link,
    tag::Tag,
    v_flex, IconName, Sizable, StyledExt,
};

use crate::section;

pub struct TextStory {
    focus_handle: gpui::FocusHandle,
    masked: bool,
}

impl super::Story for TextStory {
    fn title() -> &'static str {
        "Text"
    }

    fn description() -> &'static str {
        "The text render testing and examples"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl gpui::Focusable> {
        Self::view(cx)
    }
}

impl TextStory {
    pub(crate) fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            masked: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(cx))
    }

    #[allow(unused)]
    fn on_click(checked: &bool, window: &mut Window, cx: &mut App) {
        println!("Check value changed: {}", checked);
    }
}
impl gpui::Focusable for TextStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TextStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Label", cx)
                    .items_start()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(Label::new("Text align left"))
                            .child(Label::new("Text align center").text_center())
                            .child(Label::new("Text align right").text_right()),
                    )
                    .child(Label::new("Color Label").text_color(ui::red_500()))
                    .child(Label::new("Font Size Label").text_size(px(20.)).font_semibold().line_height(rems(1.8)))
                    .child(
                        div().w(px(200.)).child(
                            Label::new("Label should support text wrap in default, if the text is too long, it should wrap to the next line.")
                                .line_height(rems(1.8)),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        section("Link", cx).child(
                            h_flex()
                                .items_start()
                                .gap_3()
                                .child(Link::new("link1").href("https://github.com").child("GitHub"))
                                .child(
                                    Link::new("link2")
                                        .href("https://github.com")
                                        .text_color(ui::red_500())
                                        .text_decoration_color(ui::red_500())
                                        .child("Red Link"),
                                )
                                .child(
                                    Link::new("link3")
                                        .child(h_flex().gap_1().child(IconName::GitHub).child("GitHub"))
                                        .on_click(cx.listener(|_, _, window, cx| cx.open_url("https://google.com"))),
                                )
                                .child(
                                    div().w(px(250.)).child(
                                        Link::new("link4")
                                            .child("https://github.com/longbridge/gpui-component")
                                            .href("https://github.com/longbridge/gpui-component"),
                                    ),
                                ),
                        ),
                    )
                    .child(
                        section("Clipboard", cx).child(
                            h_flex()
                                .w_full()
                                .gap_4()
                                .child(
                                    Clipboard::new("clipboard1")
                                        .content(|_| Label::new("Click icon to copy"))
                                        .value_fn({
                                            let view = cx.model().clone();
                                            move |cx| SharedString::from(format!("masked :{}", view.read(cx).masked))
                                        })
                                        .on_copied(|value, _| println!("Copied value: {}", value)),
                                )
                                .child(
                                    Clipboard::new("clipboard2")
                                        .content(|_| Link::new("link1").href("https://github.com").child("GitHub"))
                                        .value("https://github.com")
                                        .on_copied(|value, _| println!("Copied value: {}", value)),
                                ),
                        ),
                    ),
            )
            .child(
                section("Maksed Label", cx).child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            h_flex().child(Label::new("9,182,1 USD").text_2xl().masked(self.masked)).child(
                                Button::new("btn-mask")
                                    .with_variant(ButtonVariant::Ghost)
                                    .icon(if self.masked { IconName::EyeOff } else { IconName::Eye })
                                    .on_click(cx.listener(|this, _, _| {
                                        this.masked = !this.masked;
                                    })),
                            ),
                        )
                        .child(Label::new("500 USD").text_xl().masked(self.masked)),
                ),
            )
            .child(
                section("Tag", cx)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Tag::primary().small().child("Tag"))
                            .child(Tag::secondary().small().child("Secondary"))
                            .child(Tag::outline().small().child("Outline"))
                            .child(Tag::danger().small().child("danger"))
                            .child(Tag::custom(ui::yellow_500(), ui::yellow_800(), ui::yellow_500()).small().child("Custom")),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Tag::primary().child("Tag"))
                            .child(Tag::secondary().child("Secondary"))
                            .child(Tag::outline().child("Outline"))
                            .child(Tag::danger().child("danger"))
                            .child(Tag::custom(ui::yellow_500(), ui::yellow_800(), ui::yellow_500()).child("Custom")),
                    ),
            )
    }
}

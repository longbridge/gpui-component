use gpui::{
    div, px, rems, IntoElement, ParentElement, Render, Styled, View, ViewContext,
    VisualContext as _, WindowContext,
};

use ui::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    clipboard::Clipboard,
    h_flex,
    label::Label,
    link::Link,
    radio::Radio,
    tag::Tag,
    v_flex, Disableable as _, IconName, Sizable, StyledExt,
};

use crate::section;

pub struct TextStory {
    focus_handle: gpui::FocusHandle,
    check1: bool,
    check2: bool,
    check3: bool,
    radio_check1: bool,
    radio_check2: bool,
    masked: bool,
}

impl super::Story for TextStory {
    fn title() -> &'static str {
        "Text"
    }

    fn description() -> &'static str {
        "The text render testing and examples"
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl TextStory {
    pub(crate) fn new(cx: &mut WindowContext) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            check1: false,
            check2: false,
            check3: true,
            radio_check1: false,
            radio_check2: true,
            masked: false,
        }
    }

    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| Self::new(cx))
    }

    #[allow(unused)]
    fn on_click(checked: &bool, cx: &mut WindowContext) {
        println!("Check value changed: {}", checked);
    }
}
impl gpui::FocusableView for TextStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TextStory {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
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
                    .child(
                        Label::new("Font Size Label")
                            .text_size(px(20.))
                            .font_semibold()
                            .line_height(rems(1.8)),
                    )
                    .child(
                        div().w(px(200.)).child(
                            Label::new("Label should support text wrap in default, if the text is too long, it should wrap to the next line.")
                                .line_height(rems(1.8)),
                        ),
                    )

            )
            .child(
                section("Link", cx).child(
                    h_flex().items_start().gap_3()
                        .child(Link::new("link1").href("https://github.com").child("GitHub"))
                        .child(Link::new("link2").href("https://github.com").text_color(ui::red_500()).text_decoration_color(ui::red_500()).child("Red Link"))
                        .child(Link::new("link3").child(h_flex().gap_1().child(IconName::GitHub).child("GitHub")).on_click(cx.listener(|_, _, cx| {
                            cx.open_url("https://google.com")
                        })))
                        .child(div().w(px(250.)).child(Link::new("link4").child("https://github.com/longbridge/gpui-component").href("https://github.com/longbridge/gpui-component")))
                )
            )
            .child(
                section("Maksed Label", cx).child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            h_flex()
                                .child(Label::new("9,182,1 USD").text_2xl().masked(self.masked))
                                .child(
                                    Button::new("btn-mask")
                                        .with_variant(ButtonVariant::Ghost)
                                        .icon(if self.masked {
                                            IconName::EyeOff
                                        } else {
                                            IconName::Eye
                                        })
                                        .on_click(cx.listener(|this, _, _| {
                                            this.masked = !this.masked;
                                        })),
                                ),
                        )
                        .child(Label::new("500 USD").text_xl().masked(self.masked)),
                ),
            )
            .child(
                section("Checkbox", cx).child(
                    h_flex()
                        .w_full()
                        .items_start()
                        .gap_6()
                        .child(
                            Checkbox::new("check1")
                                .checked(self.check1)
                                .on_click(cx.listener(|v, _, _| {
                                    v.check1 = !v.check1;
                                })),
                        )
                        .child(
                            Checkbox::new("check2")
                                .checked(self.check2)
                                .label("Subscribe to newsletter")
                                .on_click(cx.listener(|v, _, _| {
                                    v.check2 = !v.check2;
                                })),
                        )
                        .child(
                            Checkbox::new("check3")
                                .checked(self.check3)
                                .label("Remember me")
                                .on_click(cx.listener(|v, _, _| {
                                    v.check3 = !v.check3;
                                })),
                        )
                        .child(
                            div().w(px(300.)).child(
                                Checkbox::new("longlong-checkbox")
                                .label("The long long label text, it should ellipsis when the text is too long.")
                            ),
                        )
                ),
            )
            .child(
                section("Disabled Checkbox", cx).child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_6()
                        .child(
                            Checkbox::new("check3")
                                .label("Disabled Checked")
                                .checked(true)
                                .disabled(true),
                        )
                        .child(
                            Checkbox::new("check3_1")
                                .label("Disabled Unchecked")
                                .checked(false)
                                .disabled(true),
                        )
                ),
            )
            .child(
                section("Radio", cx).child(
                    h_flex()
                        .w_full()
                        .gap_4()
                        .items_start()
                        .child(
                            Radio::new("radio1")
                                .checked(self.radio_check1)
                                .on_click(cx.listener(|this, v, _cx| {
                                    this.radio_check1 = *v;
                                })),
                        )
                        .child(
                            Radio::new("radio2")
                                .label("Radio")
                                .checked(self.radio_check2)
                                .on_click(cx.listener(|this, v, _cx| {
                                    this.radio_check2 = *v;
                                })),
                        )
                        .child(
                            Radio::new("radio3")
                                .label("Disabled Radio")
                                .checked(true)
                                .disabled(true),
                        )
                        .child(
                              div().w(px(200.)).child(
                                  Radio::new("radio3")
                                      .label("A long long long text radio label")
                                      .checked(true)
                                      .disabled(true),
                              ),
                        )
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
                                .value("Copied!")
                                .on_copied(|value, _| println!("Copied value: {}", value)),
                        )
                        .child(
                            Clipboard::new("clipboard2")
                                .content(|_| Link::new("link1").href("https://github.com").child("GitHub"))
                                .value("https://github.com")
                                .on_copied(|value, _| println!("Copied value: {}", value)),
                        )
                ),
            )
            .child(
                section("Tag", cx)
                    .child(
                        h_flex().gap_2()
                            .child(Tag::primary().small().child("Tag"))
                            .child(Tag::secondary().small().child("Secondary"))
                            .child(Tag::outline().small().child("Outline"))
                            .child(Tag::destructive().small().child("Destructive"))
                            .child(Tag::custom(ui::yellow_500(), ui::yellow_800(), ui::yellow_500()).small().child("Custom"))
                    )
                    .child(
                        h_flex().gap_2()
                            .child(Tag::primary().child("Tag"))
                            .child(Tag::secondary().child("Secondary"))
                            .child(Tag::outline().child("Outline"))
                            .child(Tag::destructive().child("Destructive"))
                            .child(Tag::custom(ui::yellow_500(), ui::yellow_800(), ui::yellow_500()).child("Custom"))
                    )
            )
    }
}

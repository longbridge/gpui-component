use gpui::{
    div, px, rems, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    green_500, h_flex,
    label::Label,
    v_flex, IconName, StyledExt,
};

use crate::section;

pub struct LabelStory {
    focus_handle: gpui::FocusHandle,
    masked: bool,
}

impl super::Story for LabelStory {
    fn title() -> &'static str {
        "Label"
    }

    fn description() -> &'static str {
        "Label used to display text or other content."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl LabelStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            masked: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    #[allow(unused)]
    fn on_click(checked: &bool, window: &mut Window, cx: &mut App) {
        println!("Check value changed: {}", checked);
    }
}
impl Focusable for LabelStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for LabelStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Label")
                    .max_w_md()
                    .items_start()
                    .child(Label::new("This is a label")),
            )
            .child(
                section("Alignment").max_w_md().child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(Label::new("Text align left"))
                        .child(Label::new("Text align center").text_center())
                        .child(Label::new("Text align right").text_right()),
                ),
            )
            .child(
                section("Label with color")
                    .max_w_md()
                    .child(Label::new("Color Label").text_color(green_500())),
            )
            .child(
                section("Font Size").max_w_md().child(
                    Label::new("Font Size Label")
                        .text_size(px(20.))
                        .font_semibold()
                        .line_height(rems(1.8)),
                ),
            )
            .child(
                section("Multi-line, line-height and text wrap")
                    .max_w_md()
                    .child(
                        div().w(px(200.)).child(
                            Label::new(
                                "Label should support text wrap in default, \
                                if the text is too long, it should wrap to the next line.",
                            )
                            .line_height(rems(1.8)),
                        ),
                    ),
            )
            .child(
                section("Maksed Label").max_w_md().child(
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
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.masked = !this.masked;
                                        })),
                                ),
                        )
                        .child(Label::new("500 USD").text_xl().masked(self.masked)),
                ),
            )
    }
}

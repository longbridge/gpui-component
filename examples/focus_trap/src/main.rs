use gpui::*;
use gpui_component::{ElementExt, button::*, h_flex, v_flex, *};

pub struct Example {
    trap1_handle: FocusHandle,
    trap2_handle: FocusHandle,
}
impl Example {
    fn new(cx: &mut App) -> Self {
        Self {
            trap1_handle: cx.focus_handle(),
            trap2_handle: cx.focus_handle(),
        }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .p_8()
            .child(div().text_xl().font_bold().child("Focus Trap Example"))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("按 Tab 键在按钮间导航。注意焦点如何在不同区域循环。"),
            )
            // 外部按钮 - 不在 focus trap 内
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("外部区域 (无 focus trap)"),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("outside-1").label("外部按钮 1"))
                            .child(Button::new("outside-2").label("外部按钮 2"))
                            .child(Button::new("outside-3").label("外部按钮 3")),
                    ),
            )
            // Focus trap 区域 1
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_base().font_semibold().child("Focus Trap 区域 1"))
                    .child(
                        h_flex()
                            .gap_2()
                            .p_4()
                            .bg(cx.theme().secondary)
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("trap1-1")
                                    .label("Trap 1 - 按钮 1")
                                    .on_click(|_, _, _| println!("Trap 1 - Button 1 clicked")),
                            )
                            .child(
                                Button::new("trap1-2")
                                    .label("Trap 1 - 按钮 2")
                                    .on_click(|_, _, _| println!("Trap 1 - Button 2 clicked")),
                            )
                            .child(
                                Button::new("trap1-3")
                                    .label("Trap 1 - 按钮 3")
                                    .on_click(|_, _, _| println!("Trap 1 - Button 3 clicked")),
                            )
                            .focus_trap("trap1", self.trap1_handle.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("→ 在此区域内按 Tab，焦点会在 3 个按钮间循环，不会跳出"),
                    ),
            )
            // 中间的外部按钮
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("外部区域 (无 focus trap)"),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("outside-4").label("外部按钮 4"))
                            .child(Button::new("outside-5").label("外部按钮 5")),
                    ),
            )
            // Focus trap 区域 2
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_base().font_semibold().child("Focus Trap 区域 2"))
                    .child(
                        v_flex()
                            .gap_2()
                            .p_4()
                            .bg(cx.theme().accent.opacity(0.1))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().accent)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Button::new("trap2-1").label("Trap 2 - 按钮 1"))
                                    .child(Button::new("trap2-2").label("Trap 2 - 按钮 2")),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("trap2-3").label("Trap 2 - 按钮 3").primary(),
                                    )
                                    .child(Button::new("trap2-4").label("Trap 2 - 按钮 4")),
                            )
                            .focus_trap("trap2", self.trap2_handle.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("→ 在此区域内按 Tab，焦点会在 4 个按钮间循环，不会跳出"),
                    ),
            )
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| Example::new(cx));
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

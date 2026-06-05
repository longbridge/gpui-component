use gpui::*;
use gpui_component::{
    Root, TitleBar, WindowExt,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

pub struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(TitleBar::new().child("Window Glass"))
            .child(
                v_flex()
                    .p_5()
                    .gap_3()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child("System glass window (requires macOS 26+ or Windows 11 22H2+)")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("primary").primary().label("Primary"))
                            .child(Button::new("outline").outline().label("Outline")),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                // Setup GPUI to use custom title bar
                titlebar: Some(TitleBar::title_bar_options()),
                ..Default::default()
            };

            let window = cx
                .open_window(options, |window, cx| {
                    let view = cx.new(|_| Example);
                    // This first level on the window, should be a Root.
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window");

            window
                .update(cx, |_, window, cx| {
                    if !window.enable_window_glass(cx) {
                        println!(
                            "Window glass requires macOS 26+ or Windows 11 22H2+, \
                            falling back to the opaque background."
                        );
                    }
                })
                .expect("Failed to update window");
        })
        .detach();
    });
}

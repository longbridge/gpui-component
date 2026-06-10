use gpui::*;
use gpui_component::{
    ActiveTheme as _, IconName, Root, TitleBar,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

/// Repro for https://github.com/longbridge/gpui-component/issues/2447
///
/// Buttons are placed inside the `TitleBar`. On Windows the title bar area is
/// marked as `WindowControlArea::Drag`, so clicking the buttons drags the
/// window (and double-click maximizes/restores) instead of firing `on_click`.
///
/// Expected: clicking the title bar buttons increments the counter below.
/// Actual (Windows): the counter stays at 0 and the window starts moving.
pub struct Example {
    settings_clicks: usize,
    profile_clicks: usize,
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                TitleBar::new()
                    // Left side: app name + badge
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child("App Name")
                            .child(Badge::new().count(5)),
                    )
                    // Right side: action buttons that SHOULD respond to clicks
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .pr_2()
                            .child(
                                Button::new("settings")
                                    .icon(IconName::Settings)
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.settings_clicks += 1;
                                        println!("settings clicked: {}", this.settings_clicks);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("profile")
                                    .icon(IconName::User)
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.profile_clicks += 1;
                                        println!("profile clicked: {}", this.profile_clicks);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id("window-body")
                    .p_5()
                    .gap_3()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child("Click the Settings / Profile buttons in the title bar.")
                    .child(div().text_color(cx.theme().muted_foreground).child(format!(
                        "settings clicks: {}   |   profile clicks: {}",
                        self.settings_clicks, self.profile_clicks
                    )))
                    .child(
                        // Same button in the body, as a baseline that always works.
                        Button::new("body-ok")
                            .primary()
                            .label("Baseline button (in body)")
                            .on_click(|_, _, _| println!("body button clicked")),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            let window_options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                ..Default::default()
            };

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example {
                    settings_clicks: 0,
                    profile_clicks: 0,
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}

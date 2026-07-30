use gpui::*;
use gpui_component::{
    ActiveTheme as _, Root, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::DropdownMenu as _,
    popover::Popover,
    v_flex,
};
use gpui_wry::WebView;

pub struct Example {
    focus_handle: FocusHandle,
    webview: Entity<WebView>,
    address_input: Entity<InputState>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let webview = cx.new(|cx| {
            let builder = wry::WebViewBuilder::new();
            #[cfg(any(debug_assertions, feature = "inspector"))]
            let builder = builder.with_devtools(true);

            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            )))]
            let webview = {
                use gtk::prelude::*;
                use wry::WebViewBuilderExtUnix;
                // borrowed from https://github.com/tauri-apps/wry/blob/dev/examples/gtk_multiwebview.rs
                // doesn't work yet
                // TODO: How to initialize this fixed?
                let fixed = gtk::Fixed::builder().build();
                fixed.show_all();
                builder.build_gtk(&fixed).unwrap()
            };
            #[cfg(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            ))]
            let webview = {
                use raw_window_handle::HasWindowHandle;

                let window_handle = window.window_handle().expect("No window handle");
                builder.build_as_child(&window_handle).unwrap()
            };

            WebView::new(webview, window, cx)
        });

        let address_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("https://longbridge.github.io/gpui-component")
        });

        let url = address_input.read(cx).value().clone();
        webview.update(cx, |view, _| {
            view.load_url(&url);
        });

        cx.new(|cx| {
            let this = Self {
                focus_handle: cx.focus_handle(),
                webview,
                address_input: address_input.clone(),
            };

            cx.subscribe(
                &address_input,
                |this: &mut Self, input, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        let url = input.read(cx).value().clone();
                        this.webview.update(cx, |view, _| {
                            view.load_url(&url);
                        });
                    }
                    _ => {}
                },
            )
            .detach();

            this
        })
    }

    pub fn hide(&self, _: &mut Window, cx: &mut App) {
        self.webview.update(cx, |webview, _| webview.hide())
    }

    #[allow(unused)]
    fn go_back(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.webview.update(cx, |webview, _| {
            webview.back().unwrap();
        });
    }

    fn show_dialog(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, |dialog, _, _| {
            dialog.title("WebView overlay dialog").child(
                v_flex()
                    .gap_3()
                    .w(px(520.))
                    .child("This modal, its translucent backdrop, text, and controls are all GPUI.")
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .child("The native WKWebView must remain visible behind the backdrop."),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("dialog-secondary").label("Secondary action"))
                            .child(
                                Button::new("dialog-primary")
                                    .primary()
                                    .label("Primary action"),
                            ),
                    ),
            )
        });
        cx.notify();
    }

    fn show_notification(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.push_notification(
            "This GPUI notification is rendered above the native WebView.",
            cx,
        );
        cx.notify();
    }
}

impl Focusable for Example {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let webview = self.webview.clone();

        v_flex()
            .p_2()
            .gap_3()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Input::new(&self.address_input))
                    .child(
                        Popover::new("webview-overlay-proof")
                            .trigger(Button::new("overlay-trigger").label("Overlay proof"))
                            .w(px(320.))
                            .child("This GPUI popover is rendered above the native WebView."),
                    )
                    .child(
                        Button::new("dialog-trigger")
                            .label("Open dialog")
                            .on_click(cx.listener(Self::show_dialog)),
                    )
                    .child(
                        Button::new("notification-trigger")
                            .label("Notify")
                            .on_click(cx.listener(Self::show_notification)),
                    )
                    .child(
                        Button::new("popup-menu-trigger")
                            .label("Popup menu")
                            .dropdown_menu(|menu, _, _| {
                                menu.label("GPUI above WebView").separator().link(
                                    "Open GPUI Component",
                                    "https://longbridge.github.io/gpui-component/",
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .h(gpui::px(400.))
                    .border_color(cx.theme().border)
                    .child(webview.clone()),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

fn main() {
    // Required this for Windows to render the WebView.
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("GPUI_DISABLE_DIRECT_COMPOSITION", "true");
    }

    gpui_platform::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        let window_bounds = WindowBounds::centered(size(px(800.), px(600.)), cx);

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(window_bounds),
                    ..Default::default()
                },
                |window, cx| {
                    let view = Example::new(window, cx);
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}

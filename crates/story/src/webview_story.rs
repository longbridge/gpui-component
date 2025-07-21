use gpui::{
    div, App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, Styled as _, Window,
};
use gpui_component::{
    h_flex,
    input::{InputEvent, InputState, TextInput},
    v_flex,
    webview::WebView,
    wry, ActiveTheme,
};

pub fn init(_: &mut App) {
    #[cfg(target_os = "linux")]
    gtk::init().unwrap();
}

pub struct WebViewStory {
    focus_handle: FocusHandle,
    webview: Entity<WebView>,
    address_input: Entity<InputState>,
}

impl super::Story for WebViewStory {
    fn title() -> &'static str {
        "WebView"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
    fn on_active(&mut self, active: bool, _window: &mut Window, cx: &mut App) {
        if active {
            self.webview.update(cx, |webview, _| webview.show());
        } else {
            self.webview.update(cx, |webview, _| webview.hide());
        }
    }
}

impl WebViewStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let focus_handle = cx.focus_handle();

        let webview = cx.new(|cx| {
            let builder = wry::WebViewBuilder::new();
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

        let address_input =
            cx.new(|cx| InputState::new(window, cx).default_value("https://google.com"));

        let url = address_input.read(cx).value().clone();
        webview.update(cx, |view, _| {
            view.load_url(&url);
        });

        cx.new(|cx| {
            let this = WebViewStory {
                focus_handle,
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
}

impl Focusable for WebViewStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WebViewStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let webview = self.webview.clone();

        v_flex()
            .p_2()
            .gap_3()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(TextInput::new(&self.address_input)),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .h(gpui::px(400.))
                    .border_color(cx.theme().border)
                    .child(webview.clone()),
            )
    }
}

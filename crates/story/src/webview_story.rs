use gpui::{
    div, App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, Styled as _, Window,
};
use ui::{
    h_flex,
    input::{InputEvent, TextInput},
    v_flex,
    webview::WebView,
    ActiveTheme,
};

pub struct WebViewStory {
    focus_handle: FocusHandle,
    webview: Entity<WebView>,
    address_input: Entity<TextInput>,
}

impl super::Story for WebViewStory {
    fn title() -> &'static str {
        "WebView"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl WebViewStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let focus_handle = cx.focus_handle();

        let webview = cx.new(|cx| {
            let webview = ui::wry::WebViewBuilder::new()
                .build_as_child(&window.raw_window_handle())
                .unwrap();
            WebView::new(webview, window, cx)
        });

        let address_input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text("https://google.com", window, cx);
            input
        });

        let url = address_input.read(cx).text();
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
                    InputEvent::PressEnter => {
                        let url = input.read(cx).text();
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
        let address_input = self.address_input.clone();

        v_flex()
            .p_2()
            .gap_3()
            .size_full()
            .child(h_flex().gap_2().items_center().child(address_input.clone()))
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

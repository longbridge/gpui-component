use gpui::*;
use gpui_component_assets::Assets;
use gpui_component_story::{Assets, WebViewStory};

pub struct Example {
    root: Entity<WebViewStory>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root = WebViewStory::view(window, cx);

        Self { root }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.root.clone())
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("GPUI_DISABLE_DIRECT_COMPOSITION", "true");
    }
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("WebView Example", Example::view, cx);
    });
}

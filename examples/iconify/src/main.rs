use gpui::{
    App, AppContext, Application, Bounds, Context, IntoElement, ParentElement, Render, Styled,
    Window, WindowBounds, WindowOptions, div, px, size, white,
};
use gpui_component::{Sizable, Theme, amber_500, green_500, iconify, red_500, theme};

#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "**/*.svg"]
pub struct IconAssets;

#[cfg(not(debug_assertions))]
impl gpui::AssetSource for IconAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow::anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

fn main() {
    #[cfg(debug_assertions)]
    let app = Application::new().with_http_client(std::sync::Arc::new(
        reqwest_client::ReqwestClient::user_agent("gpui-component-example-iconify").unwrap(),
    ));

    #[cfg(not(debug_assertions))]
    let app = Application::new().with_assets(IconAssets);

    app.run(|cx: &mut App| {
        theme::init(cx);

        let theme = Theme::global_mut(cx);
        if cfg!(debug_assertions) {
            theme.iconify.cache_dir = Some("assets".into());
        } else {
            theme.iconify.api_url = None;
            theme.iconify.cache_dir = None;
        }

        let bounds = Bounds::centered(None, size(px(300.0), px(300.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| IconExample),
        )
        .unwrap();
        cx.activate(true);
    });
}

struct IconExample;

impl Render for IconExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .justify_center()
            .gap_8()
            .bg(white())
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .gap_8()
                    .child(iconify().path("mdi:smiley-outline").text_color(red_500()))
                    .child(
                    iconify().path("lucide/smile").text_color(amber_500()).large(),
                    )
                    .child(
                        iconify().data(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 48 48"><g fill="none" stroke="currentColor" stroke-linejoin="round" stroke-width="4"><path d="M24 44c11.046 0 20-8.954 20-20S35.046 4 24 4S4 12.954 4 24s8.954 20 20 20Z"/><path stroke-linecap="round" d="M31 18v1m-14-1v1m14 12s-2 4-7 4s-7-4-7-4"/></g></svg>"#).text_color(green_500()).size_8(),
                    )
            )
    }
}

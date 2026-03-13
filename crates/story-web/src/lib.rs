use gpui::{prelude::*, *};
use gpui_component::Root;
use gpui_component_assets::Assets;
use gpui_component_story::{Gallery, StoryRoot};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // Initialize logging to browser console
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");

    // Also initialize tracing for WASM
    tracing_wasm::set_as_global_default();

    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    #[cfg(not(target_family = "wasm"))]
    let app = gpui_platform::application();
    #[cfg(target_family = "wasm")]
    let app = gpui_platform::single_threaded_web();

    app.with_assets(Assets::new(
        "https://longbridge.github.io/gpui-component/gallery/",
    ))
    .run(|cx: &mut App| {
        gpui_component_story::init(cx);

        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = Gallery::view(None, window, cx);
            let story_root = cx.new(|cx| StoryRoot::new("GPUI Component", view, window, cx));
            cx.new(|cx| Root::new(story_root, window, cx))
        })
        .expect("Failed to open window");
        cx.activate(true);
    });

    Ok(())
}

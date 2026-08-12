use std::borrow::Cow;
use std::cell::RefCell;

use gpui::{prelude::*, *};
use gpui_component::{Root, theme::Theme};
use gpui_component_assets::Assets;
use gpui_component_story::{Gallery, StoryRoot};
use wasm_bindgen::prelude::*;

thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn run(story: Option<String>) -> Result<(), JsValue> {
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

    let app = app.with_assets(Assets::new(
        "https://longbridge.github.io/gpui-component/gallery/",
    ));
    let launch = |cx: &mut App| {
        gpui_component_story::init(cx);

        // Load a compact, offline font stack for WASM, where host system fonts
        // are unavailable. Inter gives the UI a neutral system-font feel, while
        // the other fonts contain only glyphs used by the story application.
        let ui_font = Cow::Borrowed(include_bytes!("../fonts/Inter-Regular.ttf").as_slice());
        let cjk_font =
            Cow::Borrowed(include_bytes!("../fonts/NotoSansSC-Regular-subset.ttf").as_slice());
        let emoji_font = Cow::Borrowed(include_bytes!("../fonts/NotoEmoji-Regular.ttf").as_slice());
        let jetbrains_mono =
            Cow::Borrowed(include_bytes!("../fonts/JetBrainsMono-Regular.ttf").as_slice());
        cx.text_system()
            .add_fonts(vec![ui_font, cjk_font, emoji_font, jetbrains_mono])
            .expect("Failed to load fonts");

        cx.global_mut::<Theme>().font_family = "Inter Variable".into();
        cx.global_mut::<Theme>().mono_font_family = "JetBrains Mono".into();

        cx.open_window(WindowOptions::default(), move |window, cx| {
            let embedded = story.is_some();
            let view = match story.as_deref() {
                Some(story) => Gallery::embedded_view(story, window, cx),
                None => Gallery::view(None, window, cx),
            };
            let story_root = cx.new(|cx| {
                if embedded {
                    StoryRoot::embedded(view, window, cx)
                } else {
                    StoryRoot::new("GPUI Component", view, window, cx)
                }
            });
            cx.new(|cx| Root::new(story_root, window, cx))
        })
        .expect("Failed to open window");
        cx.activate(true);
    };

    #[cfg(target_family = "wasm")]
    APPLICATION.with(|application| {
        *application.borrow_mut() = Some(app.run_embedded(launch));
    });
    #[cfg(not(target_family = "wasm"))]
    app.run(launch);

    Ok(())
}

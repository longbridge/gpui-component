use std::borrow::Cow;

use gpui::{prelude::*, *};
use gpui_component::{theme::Theme, Root};
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
    let app = {
        let app = gpui_platform::single_threaded_web();

        // Temporary fix: intentionally leak the `Rc<AppCell>` to keep the application alive
        struct WasmApplication(std::rc::Rc<AppCell>);
        let wasm_app = unsafe { std::mem::transmute::<Application, WasmApplication>(app) };
        std::mem::forget(wasm_app.0.clone());
        unsafe { std::mem::transmute::<WasmApplication, Application>(wasm_app) }
    };

    app.with_assets(Assets::new(
        "https://longbridge.github.io/gpui-component/gallery/",
    ))
    .run(|cx: &mut App| {
        gpui_component_story::init(cx);

        // Load fonts for WASM (system fonts are not available in the browser).
        // - Noto Sans SC: subset covering GB2312 Level 1 (~3755 common Chinese characters) + Latin
        // - Noto Emoji: monochrome emoji glyphs
        // - JetBrains Mono: code editor monospace font for story examples.
        let cjk_font = Cow::Borrowed(
            include_bytes!("../fonts/NotoSansSC-Regular-subset.ttf").as_slice(),
        );
        let emoji_font = Cow::Borrowed(
            include_bytes!("../fonts/NotoEmoji-Regular.ttf").as_slice(),
        );
        let jetbrains_mono = Cow::Borrowed(
            include_bytes!("../fonts/JetBrainsMono-Regular.ttf").as_slice(),
        );
        cx.text_system()
            .add_fonts(vec![cjk_font, emoji_font, jetbrains_mono])
            .expect("Failed to load fonts");

        // Use Noto Sans SC as the default font family for unified CJK + Latin rendering.
        cx.global_mut::<Theme>().font_family = "Noto Sans SC".into();
        cx.global_mut::<Theme>().mono_font_family = "JetBrains Mono".into();

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

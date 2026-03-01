use gpui_component_assets::Assets;
use gpui_component_story::{Gallery, init, create_new_window};

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    // Parse `cargo run -- <story_name>`
    let name = std::env::args().nth(1);

    app.run(move |cx| {
        init(cx);
        cx.activate(true);

        create_new_window(
            "GPUI Component",
            move |window, cx| Gallery::view(name.as_deref(), window, cx),
            cx,
        );
    });
}

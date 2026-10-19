use gpui_kit::{AppContext, Focusable, WindowOptions};
use gpui_kit_nsh::nsh::{App, AppOptions, AppView, Bundle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gpui_kit_nsh::nsh::set_ui_sets(vec![nsh_ui_mobile::registry(), gpui_kit_nsh::ui_set()]);
    let path = std::env::args()
        .nth(1)
        .ok_or("expected a trusted NSH bundle path")?;
    // SAFETY: this development host loads the developer's locally built bundle.
    let bundle = unsafe { Bundle::read_trusted(std::path::Path::new(&path)) }?;
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(
                WindowOptions {
                    inactive_frame_interval: None,
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    cx.new(|cx| {
                        let view = AppView::new(
                            AppOptions {
                                components: gpui_kit_nsh::components(),
                                ..Default::default()
                            },
                            window,
                            cx,
                        );
                        view.on_error(|error| eprintln!("NSH: {error}"));
                        view.load(App::Js(bundle));
                        window.focus(&view.focus_handle(cx), cx);
                        view
                    })
                },
            )
            .expect("failed to open NSH window");
            cx.on_window_closed(|cx, _| cx.quit()).detach();
        });
    Ok(())
}

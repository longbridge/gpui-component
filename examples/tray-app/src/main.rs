use gpui::{
    AppContext as _, Application, Context, IntoElement, ParentElement as _, Render, Styled as _,
    Window, WindowOptions, div,
};
use gpui_component::{
    Root, StyledExt as _,
    button::{Button, ButtonVariants as _},
};
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuItem},
};

pub struct Example;
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn load_icon(path: &str) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

fn create_tray_icon() {
    let tray_menu = Menu::new();
    let _ = tray_menu.append(&MenuItem::new("item1", true, None));
    let _ = tray_menu.append(&MenuItem::new("item1", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_title("GPUI Tray App")
        .with_tooltip("Hello from GPUI Tray App")
        .with_menu(Box::new(tray_menu))
        .build()
        .unwrap();

    tray_icon
        .set_icon(Some(load_icon("examples/tray-app/icon.png")))
        .unwrap();
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        create_tray_icon();

        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                cx.hide();

                let view = cx.new(|_| Example);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

use gpui::*;
use gpui_component::{button::*, menu::ContextMenuExt, *};

actions!(class_menu, [Open, Delete, Export, Info]);

pub struct HelloWorld;

impl HelloWorld {
    fn show_modal(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.open_modal(cx, move |modal, _, _| {
            modal.title("Test Modal").child("Hello from Modal!")
        });
    }

    fn show_drawer(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.open_sheet(cx, move |drawer, _, _| {
            drawer.title("Test Drawer").child("Hello from Drawer!")
        });
    }
}

impl Render for HelloWorld {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(gpui::white())
            .size_full()
            .child(TitleBar::new().child("Modal & Drawer"))
            .child(
                div()
                    .p_8()
                    .v_flex()
                    .gap_2()
                    .size_full()
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Button::new("btn1")
                                    .outline()
                                    .label("Open Modal")
                                    .on_click(cx.listener(Self::show_modal)),
                            )
                            .child(
                                Button::new("btn2")
                                    .outline()
                                    .label("Open Drawer")
                                    .on_click(cx.listener(Self::show_drawer)),
                            ),
                    )
                    .child(
                        div()
                            .id("second-area")
                            .v_flex()
                            .h_40()
                            .border_1()
                            .border_dashed()
                            .border_color(gpui::black())
                            .items_center()
                            .justify_center()
                            .hover(|this| this.bg(gpui::yellow().opacity(0.2)))
                            .child("Hover test here.")
                            .child("Right click to show Context Menu")
                            .context_menu({
                                move |this, _, _| {
                                    this.separator()
                                        .menu("Open", Box::new(Open))
                                        .menu("Delete", Box::new(Delete))
                                        .menu("Export", Box::new(Export))
                                        .menu("Info", Box::new(Info))
                                        .separator()
                                }
                            }),
                    ),
            )
            .children(Root::render_modal_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|_| HelloWorld);
                    // This first level on the window, should be a Root.
                    cx.new(|cx| Root::new(view.into(), window, cx))
                },
            )?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

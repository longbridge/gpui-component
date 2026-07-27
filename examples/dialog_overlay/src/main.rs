use gpui::*;
use gpui_component::{button::*, menu::ContextMenuExt, text::TextView, *};
use gpui_component_assets::Assets;

actions!(class_menu, [Open, Delete, Export, Info]);

pub struct HelloWorld;

impl HelloWorld {
    fn show_dialog(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, move |dialog, _, _| {
            dialog.title("Selectable dialog").child(
                TextView::markdown(
                    "dialog-text",
                    "Select **this** text, then drag the mouse *out of the dialog* over \
                     the paragraph behind it. The text behind must NOT get selected",
                )
                .selectable(true),
            )
        });
    }

    fn show_sheet(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.open_sheet(cx, move |sheet, _, _| {
            sheet.title("Selectable Sheet").child(
                TextView::markdown(
                    "sheet-text",
                    "Select **this** text, then drag the mouse *out of the sheet* over \
                     the paragraph behind it. The text behind must NOT get selected",
                )
                .selectable(true),
            )
        });
    }
}

impl Render for HelloWorld {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(gpui::white())
            .size_full()
            .child(TitleBar::new().child("Dialog & Sheet"))
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
                                    .label("Open dialog")
                                    .on_click(cx.listener(Self::show_dialog)),
                            )
                            .child(
                                Button::new("btn2")
                                    .outline()
                                    .label("Open Sheet")
                                    .on_click(cx.listener(Self::show_sheet)),
                            ),
                    )
                    // Selectable text behind the modals. Open a dialog/sheet,
                    // select its text, drag the mouse out over this paragraph,
                    // and confirm this text is NOT selected.
                    .child(
                        TextView::markdown(
                            "behind-text",
                            "**Background text** behind the modals. While a dialog or \
                             sheet is open, a selection started inside it must not \
                             extend onto this paragraph.",
                        )
                        .selectable(true),
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
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

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
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}

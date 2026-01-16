use gpui::*;
use gpui_component::{scroll::ScrollableElement, *};

pub struct Example;
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size(px(500.))
            .id("scrollable-container")
            .overflow_y_scrollbar()
            .gap_4()
            .p_4()
            .child("Scrollable Content")
            .children((0..100).map(|i| {
                div()
                    .h(px(40.))
                    .w_full()
                    .bg(cx.theme().secondary)
                    .child(format!("Item {}", i))
            }))
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

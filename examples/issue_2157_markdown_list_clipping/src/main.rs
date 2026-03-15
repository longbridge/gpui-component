use gpui::*;
use gpui_component::{text::TextView, ActiveTheme as _, Root, v_flex};

struct ListTextClippingRepro;

impl Render for ListTextClippingRepro {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let markdown = r#"## Evaluate the Options based on Logic and Practicality:

- **Option A: Driving (Car)**
  - *Pros:* If it's raining/hot, air conditioning is nice; if the car has a sunroof/cam, you can watch the scenery.
  - *Cons:* Wait time at the car wash might be long; you have to drive around other cars while waiting; parking fees (maybe); need to park in a lot or street, then maybe move the car after washing? Actually, most car washes require you to leave your car there anyway. So driving is just extra work.
  - *Key realization:* You are already inside the car wash area (or close enough that you're about to park). If you drive 50 meters, you have to drive *into* the spot. But if you walk, you just enter the facility.
"#;

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .p_6()
            .child(
                div()
                    .w(px(520.))
                    .h(px(420.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().background)
                    .p_3()
                    .child(
                        TextView::markdown("list-text-clipping", markdown).selectable(true),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| ListTextClippingRepro);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

use gpui::*;
use gpui_component::{
    highlighter::Language,
    input::{Input, InputState, TabSize},
    resizable::h_resizable,
    text::TextView,
};
use gpui_component_story::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("./fixtures/test.html");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::Html)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(EXAMPLE)
                .placeholder("Enter your HTML here...")
        });

        let _subscribe = cx.subscribe(
            &input_state,
            |_, _, _: &gpui_component::input::InputEvent, cx| {
                cx.notify();
            },
        );

        Self {
            input_state,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_resizable("container")
            .child(
                div()
                    .id("source")
                    .size_full()
                    .font_family("Menlo")
                    .text_size(px(13.))
                    .child(
                        Input::new(&self.input_state)
                            .h_full()
                            .appearance(false)
                            .focus_bordered(false),
                    )
                    .into_any(),
            )
            .child(
                TextView::html(
                    "preview",
                    self.input_state.read(cx).value().clone(),
                    window,
                    cx,
                )
                .p_5()
                .scrollable(true)
                .selectable(true)
                .into_any(),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("HTML Render (native)", Example::view, cx);
    });
}

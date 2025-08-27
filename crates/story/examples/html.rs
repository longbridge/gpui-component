use gpui::*;
use gpui_component::{
    highlighter::Language,
    input::{InputState, TabSize, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    text::TextView,
};
use story::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    resizable_state: Entity<ResizableState>,
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

        let resizable_state = ResizableState::new(cx);

        let _subscribe = cx.subscribe(
            &input_state,
            |_, _, _: &gpui_component::input::InputEvent, cx| {
                cx.notify();
            },
        );

        Self {
            input_state,
            resizable_state,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_resizable("container", self.resizable_state.clone())
            .child(
                resizable_panel().child(
                    div()
                        .id("source")
                        .size_full()
                        .font_family("Menlo")
                        .text_size(px(13.))
                        .child(
                            TextInput::new(&self.input_state)
                                .h_full()
                                .appearance(false)
                                .focus_bordered(false),
                        ),
                ),
            )
            .child(
                resizable_panel().child(
                    div()
                        .id("preview")
                        .size_full()
                        .p_5()
                        .overflow_y_scroll()
                        .child(
                            TextView::html(
                                "preview",
                                self.input_state.read(cx).value().clone(),
                                window,
                                cx,
                            )
                            .selectable(),
                        ),
                ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("HTML Render (native)", Example::view, cx);
    });
}

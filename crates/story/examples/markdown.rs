use gpui::*;
use gpui_component::{
    highlighter::Language,
    input::{InputEvent, InputState, TabSize, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    text::TextView,
};
use story::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    resizable_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,
}

const EXAMPLE: &str = include_str!("./fixtures/test.md");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::Markdown)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    ..Default::default()
                })
                .searchable(true)
                .placeholder("Enter your Markdown here...")
                .default_value(EXAMPLE)
        });
        let resizable_state = ResizableState::new(cx);

        let _subscriptions = vec![cx.subscribe(&input_state, |_, _, _: &InputEvent, _| {})];

        Self {
            resizable_state,
            input_state,
            _subscriptions,
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
                        .font_family("Monaco")
                        .text_size(px(12.))
                        .child(
                            TextInput::new(&self.input_state)
                                .h_full()
                                .p_0()
                                .border_0()
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
                            TextView::markdown(
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

        story::create_new_window("Markdown Editor", Example::view, cx);
    });
}

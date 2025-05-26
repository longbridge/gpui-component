use gpui::*;
use gpui_component::{
    highlighter::{HighlightTheme, Highlighter},
    input::{InputState, TabSize, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    text::TextView,
    ActiveTheme as _,
};
use story::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    resizable_state: Entity<ResizableState>,
    is_dark: bool,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("./html.html");
const LANG: &str = "html";

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Some(LANG), &HighlightTheme::default_light())
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
            is_dark: false,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();
        if self.is_dark != is_dark {
            self.is_dark = is_dark;
            self.input_state.update(cx, |state, cx| {
                if is_dark {
                    state.set_highlighter(
                        Highlighter::new(Some(LANG), &HighlightTheme::default_dark()),
                        cx,
                    );
                } else {
                    state.set_highlighter(
                        Highlighter::new(Some(LANG), &HighlightTheme::default_light()),
                        cx,
                    );
                }
            });
        }

        h_resizable("container", self.resizable_state.clone())
            .child(
                resizable_panel().child(
                    div()
                        .id("source")
                        .size_full()
                        .font_family("Menlo")
                        .text_size(px(13.))
                        .child(TextInput::new(&self.input_state).h_full().appearance(false)),
                ),
            )
            .child(
                resizable_panel().child(
                    div()
                        .id("preview")
                        .size_full()
                        .p_5()
                        .overflow_y_scroll()
                        .child(TextView::html("preview", self.input_state.read(cx).value())),
                ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("HTML Example", Example::view, cx);
    });
}

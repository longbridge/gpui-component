use gpui::*;
use gpui_component::{
    highlighter::{HighlightTheme, Language},
    input::{InputEvent, InputState, Marker, MarkerSeverity, TabSize, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    text::{TextView, TextViewStyle},
    ActiveTheme as _,
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
                .placeholder("Enter your Markdown here...")
                .default_value(EXAMPLE)
        });
        let resizable_state = ResizableState::new(cx);

        let _subscriptions = vec![cx.subscribe(&input_state, |_, input, _: &InputEvent, cx| {
            // Subscribe to input changes and perform linting with AutoCorrect for markers example.
            let value = input.read(cx).value().clone();
            let result = autocorrect::lint_for(value.as_str(), "md");

            let mut markets = vec![];
            for item in result.lines.iter() {
                let severity = match item.severity {
                    autocorrect::Severity::Error => MarkerSeverity::Warning,
                    autocorrect::Severity::Warning => MarkerSeverity::Hint,
                    autocorrect::Severity::Pass => MarkerSeverity::Info,
                };

                let start = (item.line, item.col);
                let end = (item.line, item.col + item.old.chars().count());
                let message = format!("AutoCorrect: {}", item.new);
                let market = Marker::new(severity, start, end, message);
                markets.push(market);
            }

            input.update(cx, |state, cx| {
                state.set_markers(markets, cx);
            });

            cx.notify();
        })];

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
        let theme = if cx.theme().mode.is_dark() {
            HighlightTheme::default_dark()
        } else {
            HighlightTheme::default_light()
        };

        let is_dark = cx.theme().mode.is_dark();

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
                            TextView::markdown(
                                "preview",
                                self.input_state.read(cx).value().clone(),
                                window,
                                cx,
                            )
                            .selectable()
                            .style(TextViewStyle {
                                highlight_theme: theme.clone(),
                                is_dark,
                                ..Default::default()
                            }),
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

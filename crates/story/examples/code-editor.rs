use gpui::*;
use gpui_component::{
    checkbox::Checkbox,
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::Language,
    input::{InputEvent, InputState, TabSize, TextInput},
    v_flex,
};
use story::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    language_state: Entity<DropdownState<Vec<SharedString>>>,
    language: Language,
    line_number: bool,
    need_update: bool,
    _subscribes: Vec<Subscription>,
}

const LANGUAGES: [(Language, &'static str); 7] = [
    (Language::Rust, include_str!("./fixtures/test.rs")),
    (Language::JavaScript, include_str!("./fixtures/test.js")),
    (Language::Go, include_str!("./fixtures/test.go")),
    (Language::Python, include_str!("./fixtures/test.py")),
    (Language::Ruby, include_str!("./fixtures/test.rb")),
    (Language::Zig, include_str!("./fixtures/test.zig")),
    (Language::C, include_str!("./fixtures/test.c")),
];

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_language = LANGUAGES[0];
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(default_language.0.name())
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(default_language.1)
                .placeholder("Enter your code here...")
        });
        let language_state = cx.new(|cx| {
            DropdownState::new(
                LANGUAGES.iter().map(|s| s.0.name().into()).collect(),
                Some(0),
                window,
                cx,
            )
        });

        let _subscribes = vec![
            cx.subscribe(&input_state, |_, _, _: &InputEvent, cx| {
                cx.notify();
            }),
            cx.subscribe(
                &language_state,
                |this, state, _: &DropdownEvent<Vec<SharedString>>, cx| {
                    if let Some(val) = state.read(cx).selected_value() {
                        if let Some(language) = Language::from_str(&val) {
                            this.language = language;
                            this.need_update = true;
                        }
                        cx.notify();
                    }
                },
            ),
        ];

        Self {
            input_state,
            language_state,
            language: default_language.0,
            line_number: true,
            need_update: false,
            _subscribes,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn update_highlighter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.need_update {
            return;
        }

        let language = self.language;
        let code = LANGUAGES.iter().find(|s| s.0 == language).unwrap().1;
        self.input_state.update(cx, |state, cx| {
            state.set_value(code, window, cx);
            state.set_highlighter(language, cx);
        });

        self.need_update = false;
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_highlighter(window, cx);

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_4()
                    .pb_0()
                    .gap_4()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .child(Dropdown::new(&self.language_state).title_prefix("Language: "))
                    .child(
                        Checkbox::new("line-numbger")
                            .checked(self.line_number)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.line_number = *checked;
                                this.input_state.update(cx, |state, cx| {
                                    state.set_line_number(this.line_number, window, cx);
                                });
                                cx.notify();
                            }))
                            .label("Line Number"),
                    ),
            )
            .child(
                div()
                    .id("source")
                    .w_full()
                    .flex_1()
                    .font_family("Monaco")
                    .p_4()
                    .text_size(px(12.))
                    .child(TextInput::new(&self.input_state).h_full()),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Code Editor", Example::view, cx);
    });
}

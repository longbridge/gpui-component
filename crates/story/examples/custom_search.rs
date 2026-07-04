use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    highlighter::Language,
    input::{Input, InputEvent, InputState, Search, TabSize},
    v_flex,
};
use gpui_component_assets::Assets;

pub struct Example {
    editor: Entity<InputState>,
    search: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::Rust)
                .line_number(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .searchable(false)
                .default_value(include_str!("./fixtures/test.rs"))
                .placeholder("Enter your code here...")
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let _subscriptions = vec![cx.subscribe(
            &search,
            |this: &mut Self, search, ev: &InputEvent, cx| match ev {
                InputEvent::Change => {
                    let query = search.read(cx).value();

                    this.editor
                        .update(cx, |editor, cx| editor.set_search_query(query, true, cx));

                    cx.notify();
                }
                _ => {}
            },
        )];

        Self {
            editor,
            search,
            _subscriptions,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn search_label(&self, cx: &mut Context<Self>) -> String {
        let editor = self.editor.read(cx);
        let count = editor.search_match_count();
        let current = editor
            .current_search_match_index()
            .map(|ix| ix + 1)
            .unwrap_or(0);

        format!("{current}/{count}")
    }

    fn search_previous(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.search_previous(cx));

        cx.notify();
    }

    fn search_next(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| editor.search_next(cx));

        cx.notify();
    }

    fn clear_search(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.search
            .update(cx, |search, cx| search.set_value("", window, cx));
        self.editor.update(cx, |editor, cx| editor.clear_search(cx));

        cx.notify();
    }

    fn on_action_search(&mut self, _: &Search, window: &mut Window, cx: &mut Context<Self>) {
        self.search.focus_handle(cx).focus(window, cx);
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_matches = self.editor.read(cx).search_match_count() > 0;

        v_flex()
            .size_full()
            .p_4()
            .gap_2()
            .on_action(cx.listener(Self::on_action_search))
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        Input::new(&self.search)
                            .small()
                            .w_64()
                            .prefix(Icon::new(IconName::Search).small()),
                    )
                    .child(
                        Button::new("search-previous")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ChevronLeft)
                            .disabled(!has_matches)
                            .on_click(cx.listener(Self::search_previous)),
                    )
                    .child(
                        Button::new("search-next")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ChevronRight)
                            .disabled(!has_matches)
                            .on_click(cx.listener(Self::search_next)),
                    )
                    .child(div().min_w_16().text_sm().child(self.search_label(cx)))
                    .child(
                        Button::new("search-clear")
                            .xsmall()
                            .ghost()
                            .icon(IconName::Close)
                            .on_click(cx.listener(Self::clear_search)),
                    ),
            )
            .child(
                Input::new(&self.editor)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .flex_1()
                    .w_full(),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);

        cx.activate(true);

        gpui_component_story::create_new_window("Custom Search", Example::view, cx);
    });
}

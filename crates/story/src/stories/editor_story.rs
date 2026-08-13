use std::{ops::Range, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Entity, HighlightStyle, IntoElement, Render, SharedString,
    Styled, Window, rgba,
};

use gpui_component::{ActiveTheme, input::*};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

const EXAMPLE_CODE: &str = include_str!("./editor_story.rs");

pub struct EditorStory {
    editor_state: Entity<EditorState>,
}

impl super::Story for EditorStory {
    fn title() -> &'static str {
        "Editor"
    }

    fn description() -> &'static str {
        "Code editor with syntax highlighting by tree-sitter."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl EditorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor_state = cx.new(|cx| {
            EditorState::new("rust", window, cx)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(EXAMPLE_CODE)
        });
        let base_state = editor_state.read(cx).base_state().clone();
        base_state.update(cx, |state, cx| {
            state.set_highlighter_factory(
                Rc::new(|language| {
                    SyntectHighlighter::new(language)
                        .map(|highlighter| Box::new(highlighter) as Box<_>)
                }),
                cx,
            );
        });

        Self { editor_state }
    }
}

struct SyntectHighlighter {
    language: SharedString,
    syntax_set: SyntaxSet,
    theme: Theme,
    styles: Vec<(Range<usize>, HighlightStyle)>,
}

impl SyntectHighlighter {
    fn new(language: &str) -> Option<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        syntax_set
            .find_syntax_by_token(language)
            .or_else(|| syntax_set.find_syntax_by_extension(language))?;
        let theme = ThemeSet::load_defaults().themes.remove("InspiredGitHub")?;

        Some(Self {
            language: language.to_owned().into(),
            syntax_set,
            theme,
            styles: Vec::new(),
        })
    }
}

impl InputHighlighter for SyntectHighlighter {
    fn language(&self) -> SharedString {
        self.language.clone()
    }

    fn update(
        &mut self,
        _edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<InputBaseState>,
    ) {
        let text = text.to_string();
        let syntax = self
            .syntax_set
            .find_syntax_by_token(self.language.as_ref())
            .or_else(|| {
                self.syntax_set
                    .find_syntax_by_extension(self.language.as_ref())
            })
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut offset = 0;
        self.styles.clear();

        for line in LinesWithEndings::from(&text) {
            if let Ok(regions) = highlighter.highlight_line(line, &self.syntax_set) {
                for (style, token) in regions {
                    let end = offset + token.len();
                    self.styles
                        .push((offset..end, color_style(style.foreground)));
                    offset = end;
                }
            } else {
                offset += line.len();
            }
        }
    }

    fn styles(
        &self,
        range: &Range<usize>,
        _resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.styles
            .iter()
            .filter_map(|(token_range, style)| {
                let start = token_range.start.max(range.start);
                let end = token_range.end.min(range.end);
                (start < end).then(|| (start..end, *style))
            })
            .collect()
    }

    fn fold_ranges(&self, _text: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

fn color_style(color: Color) -> HighlightStyle {
    let packed = u32::from_be_bytes([color.r, color.g, color.b, color.a]);
    HighlightStyle {
        color: Some(rgba(packed).into()),
        ..Default::default()
    }
}

impl Render for EditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Editor::new(&self.editor_state)
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(cx.theme().mono_font_size)
            .size_full()
    }
}

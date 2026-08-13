use std::{ops::Range, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Entity, HighlightStyle, IntoElement, Render, SharedString,
    Styled, Window,
};

use gpui_component::{ActiveTheme, input::*};
use syntect::{
    parsing::{ParseState, ScopeStack, SyntaxSet},
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
        "Code editor with theme-aware syntax highlighting and folding."
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
                .folding(true)
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
    highlights: Vec<(Range<usize>, &'static str)>,
}

impl SyntectHighlighter {
    fn new(language: &str) -> Option<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        syntax_set
            .find_syntax_by_token(language)
            .or_else(|| syntax_set.find_syntax_by_extension(language))?;
        Some(Self {
            language: language.to_owned().into(),
            syntax_set,
            highlights: Vec::new(),
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
        let mut parser = ParseState::new(syntax);
        let mut scopes = ScopeStack::new();
        let mut offset = 0;
        self.highlights.clear();

        for line in LinesWithEndings::from(&text) {
            if let Ok(operations) = parser.parse_line(line, &self.syntax_set) {
                let mut cursor = 0;
                for (index, operation) in operations {
                    push_highlight(
                        &mut self.highlights,
                        offset + cursor..offset + index,
                        &scopes,
                    );
                    let _ = scopes.apply(&operation);
                    cursor = index;
                }
                push_highlight(
                    &mut self.highlights,
                    offset + cursor..offset + line.len(),
                    &scopes,
                );
            }
            offset += line.len();
        }
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.highlights
            .iter()
            .filter_map(|(token_range, name)| {
                let start = token_range.start.max(range.start);
                let end = token_range.end.min(range.end);
                (start < end)
                    .then(|| resolver.style(name).map(|style| (start..end, style)))
                    .flatten()
            })
            .collect()
    }

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        brace_fold_ranges(&text.to_string())
    }
}

fn push_highlight(
    highlights: &mut Vec<(Range<usize>, &'static str)>,
    range: Range<usize>,
    scopes: &ScopeStack,
) {
    if !range.is_empty() {
        if let Some(name) = semantic_name(scopes) {
            highlights.push((range, name));
        }
    }
}

fn semantic_name(scopes: &ScopeStack) -> Option<&'static str> {
    for scope in scopes.scopes.iter().rev() {
        let scope = scope.build_string();
        if scope.starts_with("comment") {
            return Some("comment");
        } else if scope.starts_with("constant.character.escape") {
            return Some("string.escape");
        } else if scope.starts_with("string") {
            return Some("string");
        } else if scope.starts_with("constant.numeric") {
            return Some("number");
        } else if scope.starts_with("constant.language.boolean") {
            return Some("boolean");
        } else if scope.starts_with("keyword.operator") {
            return Some("operator");
        } else if scope.starts_with("keyword") || scope.starts_with("storage") {
            return Some("keyword");
        } else if scope.starts_with("entity.name.function") || scope.starts_with("support.function")
        {
            return Some("function");
        } else if scope.starts_with("entity.name.type")
            || scope.starts_with("entity.name.class")
            || scope.starts_with("support.type")
        {
            return Some("type");
        } else if scope.starts_with("variable") {
            return Some("variable");
        } else if scope.starts_with("constant") {
            return Some("constant");
        } else if scope.starts_with("punctuation") {
            return Some("punctuation");
        }
    }
    None
}

fn brace_fold_ranges(text: &str) -> Vec<FoldRange> {
    let mut starts = Vec::new();
    let mut ranges = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        let mut chars = line.chars().peekable();
        let mut quoted = false;
        let mut escaped = false;
        while let Some(character) = chars.next() {
            if !quoted && character == '/' && chars.peek() == Some(&'/') {
                break;
            }
            if character == '"' && !escaped {
                quoted = !quoted;
            } else if !quoted && character == '{' {
                starts.push(line_number);
            } else if !quoted && character == '}' {
                if let Some(start_line) = starts.pop() {
                    if start_line < line_number {
                        ranges.push(FoldRange::new(start_line, line_number));
                    }
                }
            }
            escaped = quoted && character == '\\' && !escaped;
            if character != '\\' {
                escaped = false;
            }
        }
    }
    ranges
}

impl Render for EditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Editor::new(&self.editor_state)
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(cx.theme().mono_font_size)
            .size_full()
    }
}

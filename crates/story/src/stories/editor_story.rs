use std::{ops::Range, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Entity, HighlightStyle, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div,
};

use gpui_component::{ActiveTheme, input::*, tab::TabBar, v_flex};
use syntect::{
    parsing::{ParseState, ScopeStack, SyntaxSet},
    util::LinesWithEndings,
};

const EXAMPLE_CODE: &str = include_str!("./editor_preview.rs");

pub struct EditorStory {
    editor_state: Entity<EditorState>,
    decorations_state: Entity<EditorState>,
    _decorations: TextDecorationCollection,
    active_tab: usize,
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

        let decoration_text = "Decoration styles\nColor highlights important text.\nItalic adds emphasis.\nUnderline marks a review range.";
        let decorations_state =
            cx.new(|cx| EditorState::new("text", window, cx).default_value(decoration_text));
        decorations_state.update(cx, |state, cx| state.prepare(window, cx));

        let marker = "Decoration styles";
        let color_range = "Color";
        let italic_range = "Italic";
        let underline_range = "Underline";
        let marker_start = decoration_text.find(marker).unwrap_or_default();
        let color_start = decoration_text.find(color_range).unwrap_or_default();
        let italic_start = decoration_text.find(italic_range).unwrap_or_default();
        let underline_start = decoration_text.find(underline_range).unwrap_or_default();
        let decorations = decorations_state.update(cx, |state, cx| {
            state.create_decorations_collection(
                vec![
                    TextDecoration::new(
                        marker_start..marker_start + marker.len(),
                        HighlightStyle {
                            background_color: Some(cx.theme().warning.opacity(0.2)),
                            font_weight: Some(gpui::FontWeight::BOLD),
                            color: Some(cx.theme().danger),
                            ..Default::default()
                        },
                    ),
                    TextDecoration::new(
                        color_start..color_start + color_range.len(),
                        HighlightStyle {
                            color: Some(cx.theme().success),
                            font_weight: Some(gpui::FontWeight::BOLD),
                            font_style: Some(gpui::FontStyle::Italic),
                            ..Default::default()
                        },
                    ),
                    TextDecoration::new(
                        italic_start..italic_start + italic_range.len(),
                        HighlightStyle {
                            color: Some(cx.theme().info),
                            font_style: Some(gpui::FontStyle::Italic),
                            ..Default::default()
                        },
                    ),
                    TextDecoration::new(
                        underline_start..underline_start + underline_range.len(),
                        HighlightStyle {
                            underline: Some(gpui::UnderlineStyle {
                                color: Some(cx.theme().warning),
                                thickness: gpui::px(2.),
                                wavy: true,
                            }),
                            ..Default::default()
                        },
                    ),
                ],
                cx,
            )
        });

        Self {
            editor_state,
            decorations_state,
            _decorations: decorations,
            active_tab: 0,
        }
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
        resolve_styles(&self.highlights, range, resolver)
    }

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        brace_fold_ranges(&text.to_string())
    }
}

fn resolve_styles(
    highlights: &[(Range<usize>, &'static str)],
    range: &Range<usize>,
    resolver: &dyn HighlightStyleResolver,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut runs = Vec::new();
    let mut cursor = range.start;
    for (highlight_range, name) in highlights {
        let start = highlight_range.start.max(range.start);
        let end = highlight_range.end.min(range.end);
        if start >= end || end <= cursor {
            continue;
        }
        if cursor < start {
            runs.push((cursor..start, HighlightStyle::default()));
        }
        runs.push((start..end, resolver.style(name).unwrap_or_default()));
        cursor = end;
    }
    if cursor < range.end {
        runs.push((cursor..range.end, HighlightStyle::default()));
    }
    runs
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
        v_flex()
            .size_full()
            .gap_3()
            .child(
                TabBar::new("editor-story-tabs")
                    .w_64()
                    .underline()
                    .selected_index(self.active_tab)
                    .on_click(cx.listener(|this, selected: &usize, _, cx| {
                        this.active_tab = *selected;
                        cx.notify();
                    }))
                    .child("Code")
                    .child("Decorations"),
            )
            .child(div().min_h_0().flex_1().child(if self.active_tab == 0 {
                Editor::new(&self.editor_state)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .size_full()
                    .into_any_element()
            } else {
                Editor::new(&self.decorations_state)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .size_full()
                    .into_any_element()
            }))
    }
}

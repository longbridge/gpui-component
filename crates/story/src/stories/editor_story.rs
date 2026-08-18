use gpui::{
    App, AppContext as _, Context, Entity, HighlightStyle, IntoElement, ParentElement, Render,
    Styled, Window, div,
};

use gpui_component::{ActiveTheme, h_flex, input::*, switch::Switch, tab::TabBar, v_flex};

const EXAMPLE_CODE: &str = include_str!("./editor_preview.rs");

pub struct EditorStory {
    editor_state: Entity<EditorState>,
    decorations_state: Entity<EditorState>,
    _decorations: TextDecorationCollection,
    active_tab: usize,
    readonly: bool,
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
            EditorState::new(window, cx)
                .language("rust")
                .folding(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(EXAMPLE_CODE)
        });

        // WASM ships without tree-sitter grammars, so it swaps in the `syntect`
        // adapter below. Native keeps the built-in tree-sitter highlighter,
        // which parses incrementally on a background thread.
        #[cfg(target_family = "wasm")]
        {
            editor_state.update(cx, |state, cx| {
                state.set_highlighter_factory(
                    std::rc::Rc::new(|language| {
                        syntect_highlighter::SyntectHighlighter::new(language)
                            .map(|highlighter| Box::new(highlighter) as Box<_>)
                    }),
                    cx,
                );
            });
        }

        let decoration_text = "Decoration styles\nColor highlights important text.\nItalic adds emphasis.\nUnderline marks a review range.";
        let decorations_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("text")
                .default_value(decoration_text)
        });

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
            readonly: false,
        }
    }
}

impl Render for EditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
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
                    .child(
                        Switch::new("editor-read-only")
                            .label("Read only")
                            .checked(self.readonly)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.readonly = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(div().min_h_0().flex_1().child(if self.active_tab == 0 {
                Editor::new(&self.editor_state)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .readonly(self.readonly)
                    .size_full()
                    .into_any_element()
            } else {
                Editor::new(&self.decorations_state)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .readonly(self.readonly)
                    .size_full()
                    .into_any_element()
            }))
    }
}

/// A minimal [`InputHighlighter`] built on `syntect`, for WASM builds, which
/// ship without tree-sitter grammars.
#[cfg(target_family = "wasm")]
mod syntect_highlighter {
    use std::{collections::HashMap, ops::Range, sync::LazyLock};

    use gpui::{Context, HighlightStyle, SharedString, Window};
    use gpui_component::input::*;
    use syntect::{
        parsing::{ParseState, Scope, ScopeStack, SyntaxSet},
        util::LinesWithEndings,
    };

    /// Loading the default syntax definitions deserializes a few megabytes, so
    /// share one set across every highlighter instance.
    static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

    pub(super) struct SyntectHighlighter {
        language: SharedString,
        /// Non-overlapping highlights, ordered by start offset.
        highlights: Vec<(Range<usize>, &'static str)>,
        fold_ranges: Vec<FoldRange>,
        /// Scope ids are cheap to compare but expensive to stringify, so
        /// remember the semantic name each one maps to.
        semantic_names: HashMap<Scope, Option<&'static str>>,
    }

    impl SyntectHighlighter {
        pub(super) fn new(language: &str) -> Option<Self> {
            find_syntax(language)?;

            Some(Self {
                language: language.to_owned().into(),
                highlights: Vec::new(),
                fold_ranges: Vec::new(),
                semantic_names: HashMap::new(),
            })
        }

        fn push_highlight(&mut self, range: Range<usize>, scopes: &ScopeStack) {
            if range.is_empty() {
                return;
            }

            let name = scopes.scopes.iter().rev().find_map(|scope| {
                *self
                    .semantic_names
                    .entry(*scope)
                    .or_insert_with(|| semantic_name(*scope))
            });
            if let Some(name) = name {
                self.highlights.push((range, name));
            }
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
            folding: bool,
            _window: &mut Window,
            _cx: &mut Context<EditorState>,
        ) {
            // `syntect` has no incremental mode, so the whole document is
            // reparsed. Read the rope once and reuse it for folding too.
            let text = text.to_string();
            let syntax = find_syntax(self.language.as_ref())
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
            let mut parser = ParseState::new(syntax);
            let mut scopes = ScopeStack::new();
            let mut offset = 0;
            self.highlights.clear();

            for line in LinesWithEndings::from(&text) {
                if let Ok(operations) = parser.parse_line(line, &SYNTAX_SET) {
                    let mut cursor = 0;
                    for (index, operation) in operations {
                        self.push_highlight(offset + cursor..offset + index, &scopes);
                        let _ = scopes.apply(&operation);
                        cursor = index;
                    }
                    self.push_highlight(offset + cursor..offset + line.len(), &scopes);
                }
                offset += line.len();
            }

            self.fold_ranges = if folding {
                brace_fold_ranges(&text)
            } else {
                Vec::new()
            };
        }

        fn styles(
            &self,
            range: &Range<usize>,
            resolver: &dyn HighlightStyleResolver,
        ) -> Vec<(Range<usize>, HighlightStyle)> {
            resolve_styles(&self.highlights, range, resolver)
        }

        fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
            self.fold_ranges.clone()
        }

        fn fold_ranges_for_edit(&self, _: Range<usize>, _: &Rope) -> Vec<FoldRange> {
            self.fold_ranges.clone()
        }
    }

    fn find_syntax(language: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
        SYNTAX_SET
            .find_syntax_by_token(language)
            .or_else(|| SYNTAX_SET.find_syntax_by_extension(language))
    }

    /// Turn the highlights overlapping `range` into gap-free style runs.
    ///
    /// `highlights` is ordered and non-overlapping, so the first candidate is
    /// found by binary search instead of scanning the whole document on every
    /// frame.
    fn resolve_styles(
        highlights: &[(Range<usize>, &'static str)],
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let first = highlights.partition_point(|(highlight, _)| highlight.end <= range.start);
        let mut runs = Vec::new();
        let mut cursor = range.start;

        for (highlight_range, name) in &highlights[first..] {
            if highlight_range.start >= range.end {
                break;
            }

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

    fn semantic_name(scope: Scope) -> Option<&'static str> {
        let scope = scope.build_string();
        let name = if scope.starts_with("comment") {
            "comment"
        } else if scope.starts_with("constant.character.escape") {
            "string.escape"
        } else if scope.starts_with("string") {
            "string"
        } else if scope.starts_with("constant.numeric") {
            "number"
        } else if scope.starts_with("constant.language.boolean") {
            "boolean"
        } else if scope.starts_with("keyword.operator") {
            "operator"
        } else if scope.starts_with("keyword") || scope.starts_with("storage") {
            "keyword"
        } else if scope.starts_with("entity.name.function") || scope.starts_with("support.function")
        {
            "function"
        } else if scope.starts_with("entity.name.type")
            || scope.starts_with("entity.name.class")
            || scope.starts_with("support.type")
        {
            "type"
        } else if scope.starts_with("variable") {
            "variable"
        } else if scope.starts_with("constant") {
            "constant"
        } else if scope.starts_with("punctuation") {
            "punctuation"
        } else {
            return None;
        };
        Some(name)
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
}

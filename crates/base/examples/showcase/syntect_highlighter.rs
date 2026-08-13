use std::ops::Range;

use gpui::{Context, HighlightStyle, SharedString, Window, rgb};
use gpui_base::input::{
    FoldRange, HighlightStyleResolver, InputBaseState, InputEdit, InputHighlighter, Rope,
};
use syntect::{
    parsing::{ParseState, ScopeStack, SyntaxSet},
    util::LinesWithEndings,
};

/// A small, WASM-compatible example adapter for Base's parser-independent
/// highlighting API. Syntect identifies scopes; the application-owned resolver
/// supplies all colors and font styles.
pub(super) struct SyntectHighlighter {
    language: SharedString,
    syntax_set: SyntaxSet,
    highlights: Vec<(Range<usize>, &'static str)>,
}

impl SyntectHighlighter {
    pub(super) fn new(language: &str) -> Option<Self> {
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

#[derive(Default)]
pub(super) struct ShowcaseHighlightStyles;

impl HighlightStyleResolver for ShowcaseHighlightStyles {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        let color = match name.split('.').next()? {
            "comment" => 0x6a737d,
            "string" => 0x032f62,
            "number" | "boolean" | "constant" => 0x005cc5,
            "keyword" | "operator" => 0xd73a49,
            "function" => 0x6f42c1,
            "type" => 0x22863a,
            "variable" => 0x24292e,
            "punctuation" => 0x586069,
            _ => return None,
        };
        Some(HighlightStyle {
            color: Some(rgb(color).into()),
            ..Default::default()
        })
    }
}

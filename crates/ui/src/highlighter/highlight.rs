use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::LazyLock;

use gpui::{App, HighlightStyle, Hsla};
use syntect::{highlighting, parsing};

use crate::ActiveTheme as _;
use crate::ThemeMode;

static SYNTAXES: LazyLock<parsing::SyntaxSet> =
    LazyLock::new(parsing::SyntaxSet::load_defaults_nonewlines);

static THEMES: LazyLock<highlighting::ThemeSet> = LazyLock::new(|| {
    let mut themes = BTreeMap::new();

    let mut cursor = std::io::Cursor::new(include_bytes!("./themes/dark.tmTheme"));
    let dark_theme = highlighting::ThemeSet::load_from_reader(&mut cursor).unwrap();
    themes.insert("default-dark".to_string(), dark_theme);

    let mut cursor = std::io::Cursor::new(include_bytes!("./themes/light.tmTheme"));
    let light_theme = highlighting::ThemeSet::load_from_reader(&mut cursor).unwrap();
    themes.insert("default-light".to_string(), light_theme);

    highlighting::ThemeSet { themes }
});

/// Inspired by the `iced` crate's `Highlighter` struct.
///
/// https://github.com/iced-rs/iced/blob/master/highlighter/src/lib.rs#L24
pub struct Highlighter {
    syntax: &'static parsing::SyntaxReference,
}

/// Returns the default theme for the given theme mode.
pub fn default_theme(mode: ThemeMode) -> highlighting::Theme {
    if mode.is_dark() {
        THEMES.themes["default-dark"].clone()
    } else {
        THEMES.themes["default-light"].clone()
    }
}

impl Highlighter {
    pub fn new(language: &str, _: &App) -> Self {
        let syntax = SYNTAXES
            .find_syntax_by_token(&language)
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());

        Self { syntax }
    }

    /// Highlight a text and returns a vector of ranges and highlight styles
    pub fn highlight(&mut self, text: &str, cx: &App) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut parser = parsing::ParseState::new(self.syntax);
        let mut stack = parsing::ScopeStack::new();
        let highlighter = highlighting::Highlighter::new(&cx.theme().highlighter);

        let ops = parser.parse_line(text, &SYNTAXES).unwrap_or_default();

        ScopeRangeIterator {
            ops,
            line_length: text.len(),
            index: 0,
            last_str_index: 0,
        }
        .filter_map(move |(range, scope)| {
            let _ = stack.apply(&scope);
            if range.is_empty() {
                return None;
            } else {
                let style_mod = highlighter.style_mod_for_stack(&stack.scopes);
                let mut style = HighlightStyle::default();
                style.color = style_mod.foreground.map(color_to_hsla);
                style.background_color = style_mod.background.map(color_to_hsla);
                Some((range, style))
            }
        })
        .collect()
    }
}

fn color_to_hsla(color: highlighting::Color) -> Hsla {
    gpui::Rgba {
        r: color.r as f32 / 255.,
        g: color.g as f32 / 255.,
        b: color.b as f32 / 255.,
        a: color.a as f32 / 100.,
    }
    .into()
}

struct ScopeRangeIterator {
    ops: Vec<(usize, parsing::ScopeStackOp)>,
    line_length: usize,
    index: usize,
    last_str_index: usize,
}

impl Iterator for ScopeRangeIterator {
    type Item = (std::ops::Range<usize>, parsing::ScopeStackOp);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index > self.ops.len() {
            return None;
        }

        let next_str_i = if self.index == self.ops.len() {
            self.line_length
        } else {
            self.ops[self.index].0
        };

        let range = self.last_str_index..next_str_i;
        self.last_str_index = next_str_i;

        let op = if self.index == 0 {
            parsing::ScopeStackOp::Noop
        } else {
            self.ops[self.index - 1].1.clone()
        };

        self.index += 1;
        Some((range, op))
    }
}

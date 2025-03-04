use std::ops::Range;
use std::sync::LazyLock;

use gpui::App;
use gpui::FontWeight;
use gpui::HighlightStyle;
use gpui::Hsla;
use syntect::highlighting;
use syntect::parsing;

use crate::ActiveTheme;

static SYNTAXES: LazyLock<parsing::SyntaxSet> =
    LazyLock::new(parsing::SyntaxSet::load_defaults_nonewlines);

static THEMES: LazyLock<highlighting::ThemeSet> =
    LazyLock::new(highlighting::ThemeSet::load_defaults);

/// Inspired by the `iced` crate's `Highlighter` struct.
///
/// https://github.com/iced-rs/iced/blob/master/highlighter/src/lib.rs#L24
pub struct Highlighter {
    syntax: &'static parsing::SyntaxReference,
    highlighter: highlighting::Highlighter<'static>,
}

impl Highlighter {
    pub fn new(language: &str, cx: &App) -> Self {
        let syntax = SYNTAXES
            .find_syntax_by_token(&language)
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
        let highlighter = highlighting::Highlighter::new(&THEMES.themes[Self::default_theme(cx)]);

        Self {
            syntax,
            highlighter,
        }
    }

    fn default_theme(cx: &App) -> &'static str {
        if cx.theme().mode.is_dark() {
            "base16-ocean.dark"
        } else {
            "base16-ocean.light"
        }
    }

    /// Update the highlighter with the current theme
    ///
    /// Call this method when theme changes
    #[allow(dead_code)]
    pub fn update(&mut self, cx: &App) {
        let theme = Self::default_theme(cx);

        // Update the highlighter with the current theme
        self.highlighter = highlighting::Highlighter::new(&THEMES.themes[theme]);
    }

    /// Highlight a text and returns a vector of ranges and highlight styles
    pub fn highlight(&mut self, text: &str) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut parser = parsing::ParseState::new(self.syntax);
        let mut stack = parsing::ScopeStack::new();
        let highlighter = &self.highlighter;

        let ops = parser.parse_line(line, &SYNTAXES).unwrap_or_default();

        ScopeRangeIterator {
            ops,
            line_length: line.len(),
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
                if let Some(font_style) = style_mod.font_style {
                    if font_style.contains(highlighting::FontStyle::BOLD) {
                        style.font_weight = Some(FontWeight::BOLD);
                    }
                    if font_style.contains(highlighting::FontStyle::ITALIC) {
                        style.font_style = Some(gpui::FontStyle::Italic);
                    }
                }

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

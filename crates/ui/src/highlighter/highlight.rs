use gpui::{HighlightStyle, Hsla, SharedString};
use std::{
    ops::Range,
    sync::{Arc, LazyLock},
};
use syntect::{highlighting, parsing};

static SYNTAXES: LazyLock<parsing::SyntaxSet> =
    LazyLock::new(parsing::SyntaxSet::load_defaults_newlines);

static DEFAULT_LIGHT: LazyLock<Arc<highlighting::Theme>> = LazyLock::new(|| {
    let mut cursor = std::io::Cursor::new(include_bytes!("./themes/light.tmTheme"));
    Arc::new(highlighting::ThemeSet::load_from_reader(&mut cursor).unwrap())
});

static DEFAULT_DARK: LazyLock<Arc<highlighting::Theme>> = LazyLock::new(|| {
    let mut cursor = std::io::Cursor::new(include_bytes!("./themes/dark.tmTheme"));
    Arc::new(highlighting::ThemeSet::load_from_reader(&mut cursor).unwrap())
});

pub static LIGHT_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme {
    name: "default-light".into(),
    inner: DEFAULT_LIGHT.clone(),
});
pub static DARK_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme {
    name: "default-dark".into(),
    inner: DEFAULT_DARK.clone(),
});

/// Represents a theme for syntax highlighting.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightTheme {
    name: SharedString,
    inner: Arc<highlighting::Theme>,
}

impl HighlightTheme {
    /// Default light theme.
    pub fn default_light() -> &'static Self {
        &LIGHT_THEME
    }

    /// Default dark theme.
    pub fn default_dark() -> &'static Self {
        &DARK_THEME
    }

    /// Parse a theme from a string (tmTheme)
    pub fn parse(name: &str, theme_str: &str) -> anyhow::Result<Self> {
        let mut cursor = std::io::Cursor::new(theme_str);
        let theme = highlighting::ThemeSet::load_from_reader(&mut cursor)?;

        Ok(Self {
            name: SharedString::from(name.to_string()),
            inner: Arc::new(theme),
        })
    }

    pub fn settings(&self) -> &highlighting::ThemeSettings {
        &self.inner.settings
    }
}

/// Inspired by the `iced` crate's `Highlighter` struct.
///
/// https://github.com/iced-rs/iced/blob/master/highlighter/src/lib.rs#L24
pub struct Highlighter<'a> {
    syntax: &'static parsing::SyntaxReference,
    pub(crate) light_theme: &'a HighlightTheme,
    pub(crate) dark_theme: &'a HighlightTheme,
    light_highlighter: highlighting::Highlighter<'a>,
    dark_highlighter: highlighting::Highlighter<'a>,
}

impl<'a> Highlighter<'a> {
    pub fn new(
        lang: Option<&str>,
        light_theme: &'a HighlightTheme,
        dark_theme: &'a HighlightTheme,
    ) -> Self {
        let syntax = lang
            .and_then(|lang| SYNTAXES.find_syntax_by_token(&lang))
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
        let light_highlighter = highlighting::Highlighter::new(&light_theme.inner);
        let dark_highlighter = highlighting::Highlighter::new(&dark_theme.inner);

        Self {
            syntax,
            light_theme,
            dark_theme,
            light_highlighter,
            dark_highlighter,
        }
    }

    pub(crate) fn theme(&self, is_dark: bool) -> &HighlightTheme {
        if is_dark {
            self.dark_theme
        } else {
            self.light_theme
        }
    }

    fn highlighter(&self, is_dark: bool) -> &highlighting::Highlighter<'a> {
        if is_dark {
            &self.dark_highlighter
        } else {
            &self.light_highlighter
        }
    }

    /// Highlight a line and returns a vector of ranges and highlight styles
    pub fn highlight(&self, line: &str, is_dark: bool) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut parser = parsing::ParseState::new(self.syntax);
        let mut stack = parsing::ScopeStack::new();

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
                let style_mod = self.highlighter(is_dark).style_mod_for_stack(&stack.scopes);
                let mut style = HighlightStyle::default();
                style.color = style_mod.foreground.map(color_to_hsla);
                style.background_color = style_mod.background.map(color_to_hsla);
                Some((range, style))
            }
        })
        .collect()
    }
}

pub fn color_to_hsla(color: highlighting::Color) -> Hsla {
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

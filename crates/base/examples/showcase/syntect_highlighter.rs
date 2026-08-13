use std::ops::Range;

use gpui::{Context, HighlightStyle, SharedString, Window, rgba};
use gpui_base::input::{
    FoldRange, HighlightStyleResolver, InputBaseState, InputEdit, InputHighlighter, Rope,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

/// A small, WASM-compatible example adapter for Base's parser-independent
/// highlighting API. It deliberately reparses the short showcase document on
/// every edit; applications can retain incremental parser state instead.
pub(super) struct SyntectHighlighter {
    language: SharedString,
    syntax_set: SyntaxSet,
    theme: Theme,
    styles: Vec<(Range<usize>, HighlightStyle)>,
}

impl SyntectHighlighter {
    pub(super) fn new(language: &str) -> Option<Self> {
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

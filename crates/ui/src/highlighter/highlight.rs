use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    ops::{Deref, Range},
    rc::Rc,
    sync::LazyLock,
};
use tree_sitter_highlight::HighlightEvent;

use crate::ThemeMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct ThemeStyle {
    color: Option<Hsla>,
    font_style: Option<FontStyle>,
    font_weight: Option<FontWeight>,
}

impl From<ThemeStyle> for HighlightStyle {
    fn from(style: ThemeStyle) -> Self {
        HighlightStyle {
            color: style.color,
            font_weight: style.font_weight,
            font_style: style.font_style,
            ..Default::default()
        }
    }
}

/// Theme for Tree-sitter Highlight
///
/// https://docs.rs/tree-sitter-highlight/0.25.4/tree_sitter_highlight/
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct HighlightColors {
    attribute: Option<ThemeStyle>,
    comment: Option<ThemeStyle>,
    constant: Option<ThemeStyle>,
    #[serde(rename = "constant.builtin")]
    constant_builtin: Option<ThemeStyle>,
    constructor: Option<ThemeStyle>,
    embedded: Option<ThemeStyle>,
    function: Option<ThemeStyle>,
    #[serde(rename = "function.builtin")]
    function_builtin: Option<ThemeStyle>,
    keyword: Option<ThemeStyle>,
    module: Option<ThemeStyle>,
    number: Option<ThemeStyle>,
    operator: Option<ThemeStyle>,
    property: Option<ThemeStyle>,
    #[serde(rename = "property.builtin")]
    property_builtin: Option<ThemeStyle>,
    punctuation: Option<ThemeStyle>,
    #[serde(rename = "punctuation.bracket")]
    punctuation_bracket: Option<ThemeStyle>,
    #[serde(rename = "punctuation.delimiter")]
    punctuation_delimiter: Option<ThemeStyle>,
    #[serde(rename = "punctuation.special")]
    punctuation_special: Option<ThemeStyle>,
    string: Option<ThemeStyle>,
    #[serde(rename = "string.special")]
    string_special: Option<ThemeStyle>,
    tag: Option<ThemeStyle>,
    #[serde(rename = "type")]
    type_: Option<ThemeStyle>,
    #[serde(rename = "type.builtin")]
    type_builtin: Option<ThemeStyle>,
    variable: Option<ThemeStyle>,
    #[serde(rename = "variable.builtin")]
    variable_builtin: Option<ThemeStyle>,
    #[serde(rename = "variable.parameter")]
    variable_parameter: Option<ThemeStyle>,
}

impl HighlightColors {
    pub fn style(&self, name: &str) -> Option<HighlightStyle> {
        match name {
            "attribute" => Some(self.attribute),
            "comment" => Some(self.comment),
            "constant" => Some(self.constant),
            "constant.builtin" => Some(self.constant_builtin),
            "constructor" => Some(self.constructor),
            "embedded" => Some(self.embedded),
            "function" => Some(self.function),
            "function.builtin" => Some(self.function_builtin),
            "keyword" => Some(self.keyword),
            "module" => Some(self.module),
            "number" => Some(self.number),
            "operator" => Some(self.operator),
            "property" => Some(self.property),
            "property.builtin" => Some(self.property_builtin),
            "punctuation" => Some(self.punctuation),
            "punctuation.bracket" => Some(self.punctuation_bracket),
            "punctuation.delimiter" => Some(self.punctuation_delimiter),
            "punctuation.special" => Some(self.punctuation_special),
            "string" => Some(self.string),
            "string.special" => Some(self.string_special),
            "tag" => Some(self.tag),
            "type" => Some(self.type_),
            "type.builtin" => Some(self.type_builtin),
            "variable" => Some(self.variable),
            "variable.builtin" => Some(self.variable_builtin),
            "variable.parameter" => Some(self.variable_parameter),
            _ => None,
        }
        .and_then(|s| s.map(|s| s.into()))
    }

    pub fn style_for_index(&self, index: usize) -> Option<HighlightStyle> {
        HIGHLIGHT_NAMES.get(index).and_then(|name| self.style(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct HighlightTheme {
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub mode: ThemeMode,
    #[serde(rename = "current_line.background")]
    pub current_line: Option<Hsla>,
    pub syntax: HighlightColors,
}

impl Deref for HighlightTheme {
    type Target = HighlightColors;

    fn deref(&self) -> &Self::Target {
        &self.syntax
    }
}

const HIGHLIGHT_NAMES: [&str; 27] = [
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "embedded",
    "function",
    "function.builtin",
    "keyword",
    "module",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.special",
    "string.special.key",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

const DEFAULT_DARK: LazyLock<HighlightTheme> = LazyLock::new(|| {
    let json = include_str!("./themes/dark.json");
    serde_json::from_str(json).unwrap()
});
const DEFAULT_LIGHT: LazyLock<HighlightTheme> = LazyLock::new(|| {
    let json = include_str!("./themes/light.json");
    serde_json::from_str(json).unwrap()
});

impl HighlightTheme {
    pub fn default_dark() -> Self {
        DEFAULT_DARK.clone()
    }

    pub fn default_light() -> Self {
        DEFAULT_LIGHT.clone()
    }
}

/// Inspired by the `iced` crate's `Highlighter` struct.
///
/// https://github.com/iced-rs/iced/blob/master/highlighter/src/lib.rs#L24
pub struct Highlighter {
    highlighter: Rc<RefCell<tree_sitter_highlight::Highlighter>>,
    config: Option<Rc<tree_sitter_highlight::HighlightConfiguration>>,
    pub(crate) light_theme: Rc<HighlightTheme>,
    pub(crate) dark_theme: Rc<HighlightTheme>,
}

impl Highlighter {
    pub fn new(config: Option<tree_sitter_highlight::HighlightConfiguration>) -> Self {
        let highlighter = tree_sitter_highlight::Highlighter::new();

        let config = config.map(|config| {
            let mut config = config;
            config.configure(&HIGHLIGHT_NAMES);
            Rc::new(config)
        });

        Self {
            highlighter: Rc::new(RefCell::new(highlighter)),
            config,
            light_theme: Rc::new(HighlightTheme::default_light()),
            dark_theme: Rc::new(HighlightTheme::default_dark()),
        }
    }

    pub fn set_theme(&mut self, light_theme: &HighlightTheme, dark_theme: &HighlightTheme) {
        self.light_theme = Rc::new(light_theme.clone());
        self.dark_theme = Rc::new(dark_theme.clone());
    }

    pub(crate) fn theme(&self, is_dark: bool) -> &Rc<HighlightTheme> {
        if is_dark {
            &self.dark_theme
        } else {
            &self.light_theme
        }
    }

    /// Highlight a line and returns a vector of ranges and highlight styles.
    pub fn highlight(&self, line: &str, is_dark: bool) -> Vec<(Range<usize>, HighlightStyle)> {
        let default_styles = vec![(0..line.len(), HighlightStyle::default())];
        let Some(config) = self.config.as_ref() else {
            return default_styles;
        };

        let theme = self.theme(is_dark).clone();
        let mut highlighter = self.highlighter.borrow_mut();
        let Ok(highlights) = highlighter.highlight(config, line.as_bytes(), None, |_| None) else {
            return default_styles;
        };

        let mut styles = vec![];
        let mut last_range = 0..0;
        let mut current_range = None;
        let mut current_style = None;
        for event in highlights {
            if let Ok(event) = event {
                match event {
                    HighlightEvent::Source { start, end } => {
                        current_range = Some(start..end);
                    }
                    HighlightEvent::HighlightStart(scope) => {
                        if let Some(style) = theme.syntax.style_for_index(scope.0) {
                            current_style = Some(style);
                        }
                    }
                    HighlightEvent::HighlightEnd => {
                        if let (Some(range), Some(style)) = (current_range, current_style) {
                            if last_range.end < range.start {
                                styles
                                    .push((last_range.end..range.start, HighlightStyle::default()))
                            }

                            styles.push((range.clone(), style));
                            last_range = range;
                        }

                        current_range = None;
                        current_style = None;
                    }
                }
            }
        }

        // Append to first and end to let ranges to covert line len.
        if last_range.end < line.len() {
            styles.push((last_range.end..line.len(), HighlightStyle::default()));
        }

        styles
    }
}

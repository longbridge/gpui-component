use gpui::{App, FontWeight, HighlightStyle, Hsla};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, LazyLock},
};

use super::LanguageConfig;
use crate::ThemeMode;

pub(super) const HIGHLIGHT_NAMES: [&str; 40] = [
    "attribute",
    "boolean",
    "comment",
    "comment.doc",
    "constant",
    "constructor",
    "embedded",
    "emphasis",
    "emphasis.strong",
    "enum",
    "function",
    "hint",
    "keyword",
    "label",
    "link_text",
    "link_uri",
    "number",
    "operator",
    "predictive",
    "preproc",
    "primary",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.list_marker",
    "punctuation.special",
    "string",
    "string.escape",
    "string.regex",
    "string.special",
    "string.special.symbol",
    "tag",
    "tag.doctype",
    "text.literal",
    "title",
    "type",
    "variable",
    "variable.special",
    "variant",
];

const DEFAULT_DARK: LazyLock<HighlightTheme> = LazyLock::new(|| {
    let json = include_str!("./themes/dark.json");
    serde_json::from_str(json).unwrap()
});
const DEFAULT_LIGHT: LazyLock<HighlightTheme> = LazyLock::new(|| {
    let json = include_str!("./themes/light.json");
    serde_json::from_str(json).unwrap()
});

/// Theme for Tree-sitter Highlight
///
/// https://docs.rs/tree-sitter-highlight/0.25.4/tree_sitter_highlight/
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct SyntaxColors {
    pub attribute: Option<ThemeStyle>,
    pub boolean: Option<ThemeStyle>,
    pub comment: Option<ThemeStyle>,
    pub comment_doc: Option<ThemeStyle>,
    pub constant: Option<ThemeStyle>,
    pub constructor: Option<ThemeStyle>,
    pub embedded: Option<ThemeStyle>,
    pub emphasis: Option<ThemeStyle>,
    #[serde(rename = "emphasis.strong")]
    pub emphasis_strong: Option<ThemeStyle>,
    #[serde(rename = "enum")]
    pub enum_: Option<ThemeStyle>,
    pub function: Option<ThemeStyle>,
    pub hint: Option<ThemeStyle>,
    pub keyword: Option<ThemeStyle>,
    pub label: Option<ThemeStyle>,
    #[serde(rename = "link_text")]
    pub link_text: Option<ThemeStyle>,
    #[serde(rename = "link_uri")]
    pub link_uri: Option<ThemeStyle>,
    pub number: Option<ThemeStyle>,
    pub operator: Option<ThemeStyle>,
    pub predictive: Option<ThemeStyle>,
    pub preproc: Option<ThemeStyle>,
    pub primary: Option<ThemeStyle>,
    pub property: Option<ThemeStyle>,
    pub punctuation: Option<ThemeStyle>,
    #[serde(rename = "punctuation.bracket")]
    pub punctuation_bracket: Option<ThemeStyle>,
    #[serde(rename = "punctuation.delimiter")]
    pub punctuation_delimiter: Option<ThemeStyle>,
    #[serde(rename = "punctuation.list_marker")]
    pub punctuation_list_marker: Option<ThemeStyle>,
    #[serde(rename = "punctuation.special")]
    pub punctuation_special: Option<ThemeStyle>,
    pub string: Option<ThemeStyle>,
    #[serde(rename = "string.escape")]
    pub string_escape: Option<ThemeStyle>,
    #[serde(rename = "string.regex")]
    pub string_regex: Option<ThemeStyle>,
    #[serde(rename = "string.special")]
    pub string_special: Option<ThemeStyle>,
    #[serde(rename = "string.special.symbol")]
    pub string_special_symbol: Option<ThemeStyle>,
    pub tag: Option<ThemeStyle>,
    #[serde(rename = "tag.doctype")]
    pub tag_doctype: Option<ThemeStyle>,
    #[serde(rename = "text.literal")]
    pub text_literal: Option<ThemeStyle>,
    pub title: Option<ThemeStyle>,
    #[serde(rename = "type")]
    pub type_: Option<ThemeStyle>,
    pub variable: Option<ThemeStyle>,
    #[serde(rename = "variable.special")]
    pub variable_special: Option<ThemeStyle>,
    pub variant: Option<ThemeStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

impl From<FontStyle> for gpui::FontStyle {
    fn from(style: FontStyle) -> Self {
        match style {
            FontStyle::Normal => gpui::FontStyle::Normal,
            FontStyle::Italic => gpui::FontStyle::Italic,
        }
    }
}

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
            font_style: style.font_style.map(Into::into),
            ..Default::default()
        }
    }
}

impl SyntaxColors {
    pub fn style(&self, name: &str) -> Option<HighlightStyle> {
        if name.is_empty() {
            return None;
        }

        match name {
            "attribute" => self.attribute,
            "boolean" => self.boolean,
            "comment" => self.comment,
            "comment.doc" => self.comment_doc,
            "constant" => self.constant,
            "constructor" => self.constructor,
            "embedded" => self.embedded,
            "emphasis" => self.emphasis,
            "emphasis.strong" => self.emphasis_strong,
            "enum" => self.enum_,
            "function" => self.function,
            "hint" => self.hint,
            "keyword" => self.keyword,
            "label" => self.label,
            "link_text" => self.link_text,
            "link_uri" => self.link_uri,
            "number" => self.number,
            "operator" => self.operator,
            "predictive" => self.predictive,
            "preproc" => self.preproc,
            "primary" => self.primary,
            "property" => self.property,
            "punctuation" => self.punctuation,
            "punctuation.bracket" => self.punctuation_bracket,
            "punctuation.delimiter" => self.punctuation_delimiter,
            "punctuation.list_marker" => self.punctuation_list_marker,
            "punctuation.special" => self.punctuation_special,
            "string" => self.string,
            "string.escape" => self.string_escape,
            "string.regex" => self.string_regex,
            "string.special" => self.string_special,
            "string.special.symbol" => self.string_special_symbol,
            "tag" => self.tag,
            "tag.doctype" => self.tag_doctype,
            "text.literal" => self.text_literal,
            "title" => self.title,
            "type" => self.type_,
            "variable" => self.variable,
            "variable.special" => self.variable_special,
            "variant" => self.variant,
            _ => self.variable,
        }
        .map(|s| s.into())
    }

    #[inline]
    pub fn style_for_index(&self, index: usize) -> Option<HighlightStyle> {
        HIGHLIGHT_NAMES.get(index).and_then(|name| self.style(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct HighlightThemeStyle {
    #[serde(rename = "editor.background")]
    pub background: Option<Hsla>,
    #[serde(rename = "editor.foreground")]
    pub foreground: Option<Hsla>,
    #[serde(rename = "editor.active_line.background")]
    pub active_line: Option<Hsla>,
    #[serde(rename = "editor.line_number")]
    pub line_number: Option<Hsla>,
    #[serde(rename = "editor.active_line_number")]
    pub active_line_number: Option<Hsla>,
    pub syntax: SyntaxColors,
}

/// Theme for Tree-sitter Highlight from JSON theme file.
///
/// This json is compatible with the Zed theme format.
///
/// https://zed.dev/docs/extensions/languages#syntax-highlighting
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct HighlightTheme {
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub appearance: ThemeMode,
    pub style: HighlightThemeStyle,
}

impl Deref for HighlightTheme {
    type Target = SyntaxColors;

    fn deref(&self) -> &Self::Target {
        &self.style.syntax
    }
}

impl HighlightTheme {
    pub fn default_dark() -> Self {
        DEFAULT_DARK.clone()
    }

    pub fn default_light() -> Self {
        DEFAULT_LIGHT.clone()
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(LanguageRegistry::new());
}

/// Registry for code highlighter languages.
#[derive(Clone)]
pub struct LanguageRegistry {
    languages: HashMap<String, LanguageConfig>,
    pub(crate) light_theme: Arc<HighlightTheme>,
    pub(crate) dark_theme: Arc<HighlightTheme>,
}

impl gpui::Global for LanguageRegistry {}

impl LanguageRegistry {
    pub fn global(cx: &App) -> &LanguageRegistry {
        cx.global::<LanguageRegistry>()
    }

    pub fn global_mut(cx: &mut App) -> &mut LanguageRegistry {
        cx.global_mut::<LanguageRegistry>()
    }

    pub fn new() -> Self {
        Self {
            languages: HashMap::new(),
            light_theme: Arc::new(HighlightTheme::default_light()),
            dark_theme: Arc::new(HighlightTheme::default_dark()),
        }
    }

    pub fn register(&mut self, lang: &str, config: &LanguageConfig) {
        self.languages.insert(lang.to_string(), config.clone());
    }

    #[allow(unused)]
    pub(crate) fn set_theme(&mut self, light_theme: &HighlightTheme, dark_theme: &HighlightTheme) {
        self.light_theme = Arc::new(light_theme.clone());
        self.dark_theme = Arc::new(dark_theme.clone());
    }

    #[allow(unused)]
    pub(crate) fn theme(&self, is_dark: bool) -> &Arc<HighlightTheme> {
        if is_dark {
            &self.dark_theme
        } else {
            &self.light_theme
        }
    }
}

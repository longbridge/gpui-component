use std::rc::Rc;

use gpui::{px, rems, App, ElementId, IntoElement, Pixels, Rems, RenderOnce, SharedString, Window};

use crate::highlighter::HighlightTheme;

use super::{html::HtmlElement, markdown::MarkdownElement};

/// A text view that can render Markdown or HTML.
///
/// ## Goals
///
/// - Provide a rich text rendering component for such as Markdown or HTML,
/// used to display rich text in GPUI application (e.g., Help messages, Release notes)
/// - Support Markdown GFM and HTML (Simple HTML like Safari Reader Mode) for showing most common used markups.
/// - Support Heading, Paragraph, Bold, Italic, StrikeThrough, Code, Link, Image, Blockquote, List, Table, HorizontalRule, CodeBlock ...
///
/// ## Not Goals
///
/// - Customization of the complex style (some simple styles will be supported)
/// - As a Markdown editor or viewer (If you want to like this, you must fork your version).
/// - As a HTML viewer, we not support CSS, we only support basic HTML tags for used to as a content reader.
///
/// See also [`MarkdownElement`], [`HtmlElement`]
#[allow(private_interfaces)]
#[derive(IntoElement, Clone)]
pub enum TextView {
    Markdown(MarkdownElement),
    Html(HtmlElement),
}

#[derive(IntoElement, Clone)]
pub enum Text {
    String(SharedString),
    TextView(TextView),
}

impl From<SharedString> for Text {
    fn from(s: SharedString) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self::String(SharedString::from(s.to_string()))
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self::String(s.into())
    }
}

impl From<TextView> for Text {
    fn from(e: TextView) -> Self {
        Self::TextView(e)
    }
}

impl RenderOnce for Text {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::String(s) => s.into_any_element(),
            Self::TextView(e) => e.into_any_element(),
        }
    }
}

/// TextViewStyle used to customize the style for [`TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Gap of each paragraphs, default is 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size for headings, default is 14px.
    pub heading_base_font_size: Pixels,
    /// Highlight theme for code blocks. Default: [`HighlightTheme::default_light()`]
    pub highlight_theme: Rc<HighlightTheme>,
    pub is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.heading_base_font_size == other.heading_base_font_size
            && self.highlight_theme == other.highlight_theme
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            highlight_theme: Rc::new(HighlightTheme::default_light().clone()),
            is_dark: false,
        }
    }
}

impl TextViewStyle {
    /// Set paragraph gap, default is 1 rem.
    pub fn paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }
}

impl TextView {
    /// Create a new markdown text view.
    pub fn markdown(id: impl Into<ElementId>, raw: impl Into<SharedString>) -> Self {
        Self::Markdown(MarkdownElement::new(id, raw))
    }

    /// Create a new html text view.
    pub fn html(id: impl Into<ElementId>, raw: impl Into<SharedString>) -> Self {
        Self::Html(HtmlElement::new(id, raw))
    }

    /// Set the source text of the text view.
    pub fn text(self, raw: impl Into<SharedString>) -> Self {
        match self {
            Self::Markdown(el) => Self::Markdown(el.text(raw)),
            Self::Html(el) => Self::Html(el.text(raw)),
        }
    }

    /// Set [`TextViewStyle`].
    pub fn style(self, style: TextViewStyle) -> Self {
        match self {
            Self::Markdown(el) => Self::Markdown(el.style(style)),
            Self::Html(el) => Self::Html(el.style(style)),
        }
    }
}

impl RenderOnce for TextView {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::Markdown(el) => el.into_any_element(),
            Self::Html(el) => el.into_any_element(),
        }
    }
}

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};
use html::HtmlElement;
use markdown::MarkdownElement;

mod element;
mod html;
mod markdown;
mod utils;

/// A text view that can render Markdown or HTML.
#[allow(private_interfaces)]
#[derive(IntoElement)]
pub enum TextView {
    Markdown(MarkdownElement),
    Html(HtmlElement),
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
}

impl RenderOnce for TextView {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::Markdown(el) => el.into_any_element(),
            Self::Html(el) => el.into_any_element(),
        }
    }
}

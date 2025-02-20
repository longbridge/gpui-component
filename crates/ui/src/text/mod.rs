use gpui::{App, AppContext, Entity, IntoElement, Render, SharedString};
use html::HtmlView;
use markdown::MarkdownView;

mod element;
mod html;
mod markdown;
mod utils;

#[allow(private_interfaces)]
pub enum TextView {
    Markdown(Entity<MarkdownView>),
    Html(Entity<HtmlView>),
}

impl TextView {
    /// Create a new markdown text view.
    pub fn markdown(raw: impl Into<SharedString>, cx: &mut App) -> Self {
        Self::Markdown(cx.new(|_| MarkdownView::new(raw)))
    }

    /// Create a new html text view.
    pub fn html(raw: impl Into<SharedString>, cx: &mut App) -> Self {
        Self::Html(cx.new(|_| HtmlView::new(raw)))
    }

    /// Set the source text of the text view.
    pub fn set_text(&mut self, raw: impl Into<SharedString>, cx: &mut App) {
        match self {
            Self::Markdown(view) => view.update(cx, |this, cx| this.set_text(raw, cx)),
            Self::Html(view) => view.update(cx, |this, cx| this.set_text(raw, cx)),
        }
    }
}

impl Render for TextView {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        match self {
            Self::Markdown(view) => view.clone().into_any_element(),
            Self::Html(view) => view.clone().into_any_element(),
        }
    }
}

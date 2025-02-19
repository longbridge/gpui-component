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
    pub fn markdown(source: impl Into<SharedString>, cx: &mut App) -> Self {
        Self::Markdown(cx.new(|_| MarkdownView::new(source)))
    }

    /// Create a new html text view.
    pub fn html(source: impl Into<SharedString>, cx: &mut App) -> Self {
        Self::Html(cx.new(|_| HtmlView::new(source)))
    }

    /// Set the source of the text view.
    pub fn set_source(&mut self, source: impl Into<SharedString>, cx: &mut App) {
        match self {
            Self::Markdown(view) => view.update(cx, |this, cx| this.set_source(source, cx)),
            Self::Html(view) => view.update(cx, |this, cx| this.set_source(source, cx)),
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

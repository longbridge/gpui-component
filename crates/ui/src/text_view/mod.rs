use gpui::{App, AppContext, Entity, IntoElement, Render, SharedString};
use markdown::MarkdownView;

mod element;
mod markdown;
mod utils;

#[allow(private_interfaces)]
pub enum TextView {
    Markdown(Entity<MarkdownView>),
}

impl TextView {
    /// Create a new markdown text view.
    pub fn markdown(source: impl Into<SharedString>, cx: &mut App) -> Self {
        Self::Markdown(cx.new(|_| MarkdownView::new(source)))
    }

    /// Set the source of the text view.
    pub fn set_source(&mut self, source: impl Into<SharedString>, cx: &mut App) {
        match self {
            Self::Markdown(view) => view.update(cx, |this, cx| this.set_source(source, cx)),
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
            Self::Markdown(view) => view.clone(),
        }
    }
}

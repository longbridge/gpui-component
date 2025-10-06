use anyhow::Result;
use gpui::{App, Context, Task, Window};
use lsp_types::ColorInformation;
use ropey::Rope;

use crate::input::{InputState, Lsp};

pub trait DocumentColorProvider {
    /// Fetches document colors for the specified range.
    ///
    /// textDocument/documentColor
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentColor
    fn document_colors(
        &self,
        _text: &Rope,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<ColorInformation>>>;
}

impl Lsp {
    pub(crate) fn update_document_colors(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.document_color_provider.as_ref() else {
            return;
        };

        let task = provider.document_colors(text, window, cx);
        self._hover_task = cx.spawn_in(window, async move |editor, cx| {
            let colors = task.await?;

            editor.update(cx, |editor, cx| {
                if colors == editor.lsp.color_informations {
                    return;
                }

                editor.lsp.color_informations = colors;
                cx.notify();
            })?;

            Ok(())
        });
    }
}

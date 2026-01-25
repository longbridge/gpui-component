use std::time::Duration;

use anyhow::Result;
use gpui::{App, Context, Task, Window};
use ropey::Rope;

use crate::input::{InputState, RopeExt, popovers::HoverPopover};

/// Hover provider
///
/// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
pub trait HoverProvider {
  /// textDocument/hover
  ///
  /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
  fn hover(
    &self,
    _text: &Rope,
    _offset: usize,
    _window: &mut Window,
    _cx: &mut App,
  ) -> Task<Result<Option<lsp_types::Hover>>>;
}

impl InputState {
  /// Handle hover trigger LSP request.
  pub(super) fn handle_hover_popover(
    &mut self,
    offset: usize,
    window: &mut Window,
    cx: &mut Context<InputState>,
  ) {
    if self.selecting {
      return;
    }

    let Some(provider) = self.lsp.hover_provider.clone() else {
      return;
    };

    if let Some(hover_popover) = self.hover_popover.as_ref() {
      if hover_popover.read(cx).is_same(offset) {
        return;
      }
    }

    // Currently not implemented.
    let task = provider.hover(&self.text, offset, window, cx);
    let mut symbol_range = self.text.word_range(offset).unwrap_or(offset..offset);
    let editor = cx.entity();
    let should_delay = self.hover_popover.is_none();
    self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
      if should_delay {
        cx.background_executor()
          .timer(Duration::from_millis(150))
          .await;
      }

      let result = task.await?;

      _ = editor.update(cx, |editor, cx| match result {
        Some(hover) => {
          if let Some(range) = hover.range {
            let start = editor.text.position_to_offset(&range.start);
            let end = editor.text.position_to_offset(&range.end);
            symbol_range = start..end;
          }
          let hover_popover = HoverPopover::new(cx.entity(), symbol_range, &hover, cx);
          editor.hover_popover = Some(hover_popover);
        }
        None => {
          editor.hover_popover = None;
        }
      });

      Ok(())
    });
  }
}

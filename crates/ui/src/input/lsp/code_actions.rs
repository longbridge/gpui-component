use anyhow::Result;
use gpui::{App, Context, Entity, SharedString, Task, Window};
use lsp_types::CodeAction;
use std::ops::Range;

use crate::input::{
  InputState, ToggleCodeActions,
  popovers::{CodeActionItem, CodeActionMenu, ContextMenu},
};

pub trait CodeActionProvider {
  /// The id for this CodeAction.
  fn id(&self) -> SharedString;

  /// Fetches code actions for the specified range.
  ///
  /// textDocument/codeAction
  ///
  /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeAction
  fn code_actions(
    &self,
    state: Entity<InputState>,
    range: Range<usize>,
    window: &mut Window,
    cx: &mut App,
  ) -> Task<Result<Vec<CodeAction>>>;

  /// Performs the specified code action.
  fn perform_code_action(
    &self,
    state: Entity<InputState>,
    action: CodeAction,
    push_to_history: bool,
    window: &mut Window,
    cx: &mut App,
  ) -> Task<Result<()>>;
}

impl InputState {
  pub(crate) fn on_action_toggle_code_actions(
    &mut self,
    _: &ToggleCodeActions,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.handle_code_action_trigger(window, cx)
  }

  /// Show code actions for the cursor.
  pub(crate) fn handle_code_action_trigger(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let providers = self.lsp.code_action_providers.clone();
    let menu = match self.context_menu.as_ref() {
      Some(ContextMenu::CodeAction(menu)) => Some(menu),
      _ => None,
    };

    let menu = match menu {
      Some(menu) => menu.clone(),
      None => {
        let menu = CodeActionMenu::new(cx.entity(), window, cx);
        self.context_menu = Some(ContextMenu::CodeAction(menu.clone()));
        menu
      }
    };

    let range = self.selected_range.start..self.selected_range.end;

    let state = cx.entity();
    self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
      let mut provider_responses = vec![];
      _ = cx.update(|window, cx| {
        for provider in providers {
          let task = provider.code_actions(state.clone(), range.clone(), window, cx);
          provider_responses.push((provider.id(), task));
        }
      });

      let mut code_actions: Vec<CodeActionItem> = vec![];
      for (provider_id, provider_responses) in provider_responses {
        if let Some(responses) = provider_responses.await.ok() {
          code_actions.extend(responses.into_iter().map(|action| CodeActionItem {
            provider_id: provider_id.clone(),
            action,
          }))
        }
      }

      if code_actions.is_empty() {
        _ = menu.update(cx, |menu, cx| {
          menu.hide(cx);
          cx.notify();
        });

        return Ok(());
      }
      editor
        .update_in(cx, |editor, window, cx| {
          if !editor.focus_handle.is_focused(window) {
            return;
          }

          _ = menu.update(cx, |menu, cx| {
            menu.show(editor.cursor(), code_actions, window, cx);
          });

          cx.notify();
        })
        .ok();

      Ok(())
    });
  }

  pub(crate) fn perform_code_action(
    &mut self,
    item: &CodeActionItem,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let providers = self.lsp.code_action_providers.clone();
    let Some(provider) = providers
      .iter()
      .find(|provider| provider.id() == item.provider_id)
    else {
      return;
    };

    let state = cx.entity();
    let task = provider.perform_code_action(state, item.action.clone(), true, window, cx);

    cx.spawn_in(window, async move |_, _| {
      let _ = task.await;
    })
    .detach();
  }
}

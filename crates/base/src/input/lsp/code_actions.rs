use super::*;

pub trait CodeActionProvider {
    fn id(&self) -> SharedString;

    fn code_actions(
        &self,
        state: Entity<InputState>,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>>;

    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>>;
}

#[derive(Clone, Debug)]
pub struct CodeActionItem {
    pub provider_id: SharedString,
    pub action: CodeAction,
}

impl InputState {
    pub(crate) fn on_action_toggle_code_actions(
        &mut self,
        _: &crate::input::ToggleCodeActions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let providers = self.lsp.code_action_providers.clone();
        let range = self.selected_range.start..self.selected_range.end;
        let state = cx.entity();
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut tasks = Vec::new();
            let _ = cx.update(|window, cx| {
                tasks.extend(providers.iter().map(|provider| {
                    (
                        provider.id(),
                        provider.code_actions(state.clone(), range.clone(), window, cx),
                    )
                }));
            });
            let mut items = Vec::new();
            for (provider_id, task) in tasks {
                if let Ok(actions) = task.await {
                    items.extend(actions.into_iter().map(|action| CodeActionItem {
                        provider_id: provider_id.clone(),
                        action,
                    }));
                }
            }
            editor.update_in(cx, |editor, window, cx| {
                if !editor.focus_handle.is_focused(window) {
                    return;
                }
                editor.code_action_session.items = items;
                editor.code_action_session.open = !editor.code_action_session.items.is_empty();
                cx.notify();
            })?;
            Ok(())
        });
    }

    pub fn perform_code_action(
        &mut self,
        item: &CodeActionItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self
            .lsp
            .code_action_providers
            .iter()
            .find(|provider| provider.id() == item.provider_id)
            .cloned()
        else {
            return;
        };
        let task = provider.perform_code_action(cx.entity(), item.action.clone(), true, window, cx);
        cx.spawn_in(window, async move |_, _| {
            let _ = task.await;
        })
        .detach();
    }
}

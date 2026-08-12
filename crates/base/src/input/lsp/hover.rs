use super::*;

pub trait HoverProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>>;
}

impl InputState {
    pub(crate) fn handle_hover_popover(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selecting {
            return;
        }
        let Some(provider) = self.lsp.hover_provider.clone() else {
            return;
        };
        if self
            .hover_session
            .as_ref()
            .is_some_and(|session| session.symbol_range.contains(&offset))
        {
            return;
        }
        let task = provider.hover(&self.text, offset, window, cx);
        let fallback = self.text.word_range(offset).unwrap_or(offset..offset);
        let editor = cx.entity();
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let hover = task.await?;
            editor.update(cx, |editor, cx| {
                editor.hover_session = hover.map(|hover| {
                    let symbol_range = hover
                        .range
                        .map(|range| {
                            editor.text.position_to_offset(&range.start)
                                ..editor.text.position_to_offset(&range.end)
                        })
                        .unwrap_or(fallback);
                    HoverSession {
                        symbol_range,
                        hover,
                    }
                });
                cx.notify();
            });
            Ok(())
        });
    }

    pub(crate) fn handle_mouse_move(
        &mut self,
        offset: usize,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.modifiers.secondary() {
            self.handle_hover_definition(offset, window, cx);
        } else {
            self.hover_definition.clear();
            self.handle_hover_popover(offset, window, cx);
        }
        cx.notify();
    }

    pub fn clear_hover_state(&mut self, cx: &mut Context<Self>) {
        let changed = !self.hover_definition.is_empty() || self.hover_session.is_some();
        self.hover_definition.clear();
        self.hover_session = None;
        self.lsp._hover_task = Task::ready(Ok(()));
        if changed {
            cx.notify();
        }
    }
}

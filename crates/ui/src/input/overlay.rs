use std::collections::HashMap;

use gpui::{
    AnyElement, App, Entity, EntityId, Global, IntoElement, ParentElement as _, Styled as _,
    WeakEntity, Window, deferred, div, px,
};
use ropey::Rope;

use super::{
    InputState,
    popovers::{CodeActionMenu, CompletionMenu, DiagnosticPopover, HoverPopover},
    search::SearchPanel,
};

#[derive(Default)]
struct InputOverlayRegistry {
    hosts: HashMap<EntityId, (WeakEntity<InputState>, InputOverlayHost)>,
}

impl Global for InputOverlayRegistry {}

struct InputOverlayHost {
    search: Entity<SearchPanel>,
    completion: Entity<CompletionMenu>,
    code_actions: Entity<CodeActionMenu>,
    hover: Option<Entity<HoverPopover>>,
    diagnostic: Option<Entity<DiagnosticPopover>>,
    search_signature: (bool, bool),
    completion_signature: String,
    code_action_signature: String,
    hover_signature: String,
    diagnostic_signature: String,
}

impl InputOverlayHost {
    fn new(state: Entity<InputState>, window: &mut Window, cx: &mut App) -> Self {
        let search = SearchPanel::new(state.clone(), window, cx);
        let completion = CompletionMenu::new(state.clone(), window, cx);
        let code_actions = CodeActionMenu::new(state.clone(), window, cx);
        Self {
            search,
            completion,
            code_actions,
            hover: None,
            diagnostic: None,
            search_signature: (false, false),
            completion_signature: String::new(),
            code_action_signature: String::new(),
            hover_signature: String::new(),
            diagnostic_signature: String::new(),
        }
    }

    fn sync(
        &mut self,
        state: &Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        let (
            search_open,
            replace_mode,
            completion_open,
            completion_start,
            completion_query,
            completion_items,
            code_action_open,
            code_action_items,
            hover,
            diagnostic,
            cursor,
            search_session,
        ) = {
            let state = state.read(cx);
            let search = state.search_session();
            let completion = state.completion_session();
            let code_actions = state.code_action_session();
            (
                search.open,
                search.replace_mode,
                completion.open,
                completion.trigger_start_offset,
                completion.query.clone(),
                completion.items.clone(),
                code_actions.open,
                code_actions.items.clone(),
                state.hover_session().cloned(),
                state.diagnostic_overlay(),
                state.cursor(),
                search.clone(),
            )
        };

        self.search
            .update(cx, |panel, _| panel.sync_session(&search_session));

        let search_signature = (search_open, replace_mode);
        if search_signature != self.search_signature {
            self.search_signature = search_signature;
            self.search.update(cx, |panel, cx| {
                if search_open {
                    let selected = Rope::from(search_session.query.clone());
                    let visible = search_session.anchor_offset.map(|offset| offset..offset);
                    panel.show_with_focus(
                        &selected,
                        replace_mode,
                        visible,
                        !cfg!(test),
                        window,
                        cx,
                    );
                } else {
                    panel.hide_with_focus(!cfg!(test), window, cx);
                }
            });
        }

        let completion_signature = format!(
            "{completion_open}:{completion_start:?}:{completion_query}:{completion_items:?}"
        );
        if completion_signature != self.completion_signature {
            self.completion_signature = completion_signature;
            self.completion.update(cx, |menu, cx| {
                if completion_open {
                    menu.update_query(completion_start.unwrap_or(cursor), completion_query);
                    menu.show(cursor, completion_items, window, cx);
                } else {
                    menu.hide(cx);
                }
            });
        }

        let code_action_signature = format!("{code_action_open}:{code_action_items:?}");
        if code_action_signature != self.code_action_signature {
            self.code_action_signature = code_action_signature;
            self.code_actions.update(cx, |menu, cx| {
                if code_action_open {
                    menu.show(cursor, code_action_items, window, cx);
                } else {
                    menu.hide(cx);
                }
            });
        }

        let hover_signature = format!("{hover:?}");
        if hover_signature != self.hover_signature {
            self.hover_signature = hover_signature;
            self.hover = hover.map(|session| {
                HoverPopover::new(state.clone(), session.symbol_range, &session.hover, cx)
            });
        }

        let diagnostic_signature = format!("{diagnostic:?}");
        if diagnostic_signature != self.diagnostic_signature {
            self.diagnostic_signature = diagnostic_signature;
            self.diagnostic = diagnostic
                .as_deref()
                .map(|entry| DiagnosticPopover::new(entry, state.clone(), cx));
        }

        let mut overlays = Vec::with_capacity(5);
        if search_open {
            overlays.push(
                deferred(
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .w(px(600.))
                        .max_w_full()
                        .child(self.search.clone()),
                )
                .into_any_element(),
            );
        }
        if completion_open {
            overlays.push(self.completion.clone().into_any_element());
        }
        if code_action_open {
            overlays.push(self.code_actions.clone().into_any_element());
        }
        if let Some(hover) = self.hover.as_ref() {
            overlays.push(hover.clone().into_any_element());
        }
        if let Some(diagnostic) = self.diagnostic.as_ref() {
            overlays.push(diagnostic.clone().into_any_element());
        }
        overlays
    }
}

pub(super) fn render_overlays(
    state: &Entity<InputState>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    install_action_handler(state, cx);
    let has_overlay = {
        let state = state.read(cx);
        state.search_session().open
            || state.completion_session().open
            || state.code_action_session().open
            || state.hover_session().is_some()
            || state.diagnostic_overlay().is_some()
    };
    if !has_overlay {
        if cx.has_global::<InputOverlayRegistry>() {
            let registry = cx.global_mut::<InputOverlayRegistry>();
            registry.hosts.remove(&state.entity_id());
            registry
                .hosts
                .retain(|_, (owner, _)| owner.upgrade().is_some());
        }
        return Vec::new();
    }

    if !cx.has_global::<InputOverlayRegistry>() {
        cx.set_global(InputOverlayRegistry::default());
    }

    let id = state.entity_id();
    let mut host = cx
        .global_mut::<InputOverlayRegistry>()
        .hosts
        .remove(&id)
        .map(|(_, host)| host)
        .unwrap_or_else(|| InputOverlayHost::new(state.clone(), window, cx));
    let overlays = host.sync(state, window, cx);
    cx.global_mut::<InputOverlayRegistry>()
        .hosts
        .insert(id, (state.downgrade(), host));
    overlays
}

fn install_action_handler(state: &Entity<InputState>, cx: &mut App) {
    let id = state.entity_id();
    state.update(cx, move |state, _| {
        state.set_overlay_action_handler(move |kind, action, window, cx| {
            let menus = cx
                .try_global::<InputOverlayRegistry>()
                .and_then(|registry| registry.hosts.get(&id))
                .map(|(_, host)| (host.completion.clone(), host.code_actions.clone()));
            let Some((completion, code_actions)) = menus else {
                return false;
            };
            match kind {
                gpui_base::input::InputOverlayKind::Completion => {
                    completion.update(cx, |menu, cx| menu.handle_action(action, window, cx))
                }
                gpui_base::input::InputOverlayKind::CodeAction => {
                    code_actions.update(cx, |menu, cx| menu.handle_action(action, window, cx))
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Render, SharedString, div};
    use gpui_base::highlighter::DiagnosticEntry;
    use gpui_base::input::CodeActionItem;
    use lsp_types::{CodeAction, CompletionItem, Hover, HoverContents, MarkedString};

    struct OverlayProbe {
        state: Entity<InputState>,
    }

    impl Render for OverlayProbe {
        fn render(
            &mut self,
            _window: &mut Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn facade_materializes_all_base_overlay_sessions(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let (probe, cx) = cx.add_window_view(|window, cx| OverlayProbe {
            state: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .searchable(true)
            }),
        });
        let state = probe.read_with(cx, |probe, _| probe.state.clone());
        cx.update(|window, cx| {
            assert!(render_overlays(&state, window, cx).is_empty());
            assert!(state.read(cx).has_overlay_action_handler());
            state.update(cx, |state, cx| {
                state.set_value("foo bar foo", window, cx);
                state.set_selected_range(4..7, cx);
                state.open_search(true, cx);
                assert_eq!(state.search_session().query, "bar");
                state.present_completion_items(
                    0,
                    "f",
                    vec![CompletionItem {
                        label: "foo".into(),
                        ..Default::default()
                    }],
                    cx,
                );
                state.present_code_actions(
                    vec![CodeActionItem {
                        provider_id: SharedString::from("test"),
                        action: CodeAction {
                            title: "Fix".into(),
                            ..Default::default()
                        },
                    }],
                    cx,
                );
                state.present_hover(
                    0..1,
                    Hover {
                        contents: HoverContents::Scalar(MarkedString::String("docs".into())),
                        range: None,
                    },
                    cx,
                );
                state.present_diagnostic(DiagnosticEntry::default(), cx);
            });

            let mut host = InputOverlayHost::new(state.clone(), window, cx);
            assert_eq!(host.sync(&state, window, cx).len(), 5);
            assert_eq!(render_overlays(&state, window, cx).len(), 5);
            assert!(
                cx.global::<InputOverlayRegistry>()
                    .hosts
                    .contains_key(&state.entity_id())
            );

            state.update(cx, |state, cx| {
                assert!(state.route_overlay_action(Box::new(super::super::Escape), window, cx));
                assert!(!state.completion_session().open);
                state.dismiss_code_action_overlay(cx);
                state.close_search(cx);
                state.clear_hover_state(cx);
                state.clear_diagnostic_overlay(cx);
            });
            assert!(host.sync(&state, window, cx).is_empty());
            assert!(render_overlays(&state, window, cx).is_empty());
            assert!(
                !cx.global::<InputOverlayRegistry>()
                    .hosts
                    .contains_key(&state.entity_id())
            );
        });

        let dropped_owner = cx.update(|window, cx| {
            let ephemeral = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .searchable(true)
            });
            ephemeral.update(cx, |state, cx| state.open_search(false, cx));
            assert_eq!(render_overlays(&ephemeral, window, cx).len(), 1);
            ephemeral.update(cx, |state, cx| state.close_search(cx));
            assert!(render_overlays(&ephemeral, window, cx).is_empty());
            assert!(
                !cx.global::<InputOverlayRegistry>()
                    .hosts
                    .contains_key(&ephemeral.entity_id())
            );
            let owner = ephemeral.downgrade();
            drop(ephemeral);
            owner
        });
        cx.run_until_parked();
        assert!(dropped_owner.upgrade().is_none());
    }
}

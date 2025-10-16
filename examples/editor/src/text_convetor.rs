use std::{ops::Range, str::FromStr};

use anyhow::Ok;
use gpui::*;
use gpui_component::input::{CodeActionProvider, InputState, RopeExt};
use lsp_types::{CodeAction, CodeActionKind, TextEdit, WorkspaceEdit};

pub struct TextConvertor;

impl CodeActionProvider for TextConvertor {
    fn id(&self) -> SharedString {
        "TextConvertor".into()
    }

    fn code_actions(
        &self,
        state: Entity<InputState>,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let mut actions = vec![];
        if range.is_empty() {
            return Task::ready(Ok(actions));
        }

        let state = state.read(cx);
        let document_uri = lsp_types::Uri::from_str("file://example").unwrap();

        let old_text = state.text().slice(range.clone()).to_string();
        let start = state.text().offset_to_position(range.start);
        let end = state.text().offset_to_position(range.end);
        let range = lsp_types::Range { start, end };

        actions.push(CodeAction {
            title: "Convert to Uppercase".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text.to_uppercase(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Convert to Lowercase".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range: range.clone(),
                            new_text: old_text.to_lowercase(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Titleize".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range: range.clone(),
                            new_text: old_text
                                .split_whitespace()
                                .map(|word| {
                                    let mut chars = word.chars();
                                    chars
                                        .next()
                                        .map(|c| c.to_uppercase().collect::<String>())
                                        .unwrap_or_default()
                                        + chars.as_str()
                                })
                                .collect::<Vec<_>>()
                                .join(" "),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Capitalize".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .chars()
                                .enumerate()
                                .map(|(i, c)| {
                                    if i == 0 {
                                        c.to_uppercase().to_string()
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .collect(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        // snake_case
        actions.push(CodeAction {
            title: "Convert to snake_case".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .chars()
                                .enumerate()
                                .map(|(i, c)| {
                                    if c.is_uppercase() {
                                        if i != 0 {
                                            format!("_{}", c.to_lowercase())
                                        } else {
                                            c.to_lowercase().to_string()
                                        }
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .collect(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let Some(edit) = action.edit else {
            return Task::ready(Ok(()));
        };

        let changes = if let Some(changes) = edit.changes {
            changes
        } else {
            return Task::ready(Ok(()));
        };

        let Some((_, text_edits)) = changes.into_iter().next() else {
            return Task::ready(Ok(()));
        };

        let state = state.downgrade();
        window.spawn(cx, async move |cx| {
            state.update_in(cx, |state, window, cx| {
                state.apply_lsp_edits(&text_edits, window, cx);
            })
        })
    }
}

use std::collections::HashSet;

use crate::home_tab::HomePage;
use gpui::{
    App, Context, Entity, InteractiveElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Task, Window, div, px,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, Sizable,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    list::{ListDelegate, ListItem, ListState},
    tooltip::Tooltip,
};
use one_core::storage::{StoredConnection, Workspace};

#[derive(Clone)]
struct WorkspaceFilterItem {
    id: i64,
    name: String,
    count: usize,
    checked: bool,
}

pub(crate) struct WorkspaceFilterDelegate {
    parent: Entity<HomePage>,
    items: Vec<WorkspaceFilterItem>,
    search_query: String,
}

impl WorkspaceFilterDelegate {
    pub(crate) fn new(parent: Entity<HomePage>) -> Self {
        Self {
            parent,
            items: Vec::new(),
            search_query: String::new(),
        }
    }

    pub(crate) fn update_items_with_data(
        &mut self,
        workspaces: &[Workspace],
        connections: &[StoredConnection],
        filtered_ids: &HashSet<i64>,
    ) {
        self.items = workspaces
            .iter()
            .filter_map(|ws| {
                let id = ws.id?;
                let count = connections
                    .iter()
                    .filter(|c| c.workspace_id == Some(id))
                    .count();
                let checked = filtered_ids.is_empty() || filtered_ids.contains(&id);

                if self.search_query.is_empty()
                    || ws
                        .name
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                {
                    Some(WorkspaceFilterItem {
                        id,
                        name: ws.name.clone(),
                        count,
                        checked,
                    })
                } else {
                    None
                }
            })
            .collect();
    }

    fn filtered_items(&self) -> &[WorkspaceFilterItem] {
        &self.items
    }
}

impl ListDelegate for WorkspaceFilterDelegate {
    type Item = ListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.search_query = query.to_string();
        let parent = self.parent.read(cx);
        self.update_items_with_data(
            &parent.workspaces,
            &parent.connections,
            &parent.filtered_workspace_ids,
        );
        cx.notify();
        Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.filtered_items().get(ix.row)?.clone();
        let parent = self.parent.clone();
        let parent_for_edit = self.parent.clone();
        let parent_for_delete = self.parent.clone();
        let item_id = item.id;
        let item_id_for_edit = item.id;
        let item_id_for_delete = item.id;
        let group_name: SharedString = format!("workspace-item-{}", item.id).into();

        Some(
            ListItem::new(ix)
                .px_3()
                .py_2()
                .rounded(px(4.0))
                .on_click(move |_, _, cx| {
                    parent.update(cx, |this, cx| {
                        this.toggle_workspace_filter(item_id, cx);
                    });
                })
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_2()
                        .group(group_name.clone())
                        .child(
                            Checkbox::new(SharedString::from(format!("ws-check-{}", item.id)))
                                .checked(item.checked),
                        )
                        .child({
                            let tooltip_text =
                                SharedString::from(format!("{} ({})", item.name, item.count));
                            h_flex()
                                .id(SharedString::from(format!("ws-name-{}", item.id)))
                                .flex_1()
                                .min_w_0()
                                .text_sm()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(item.name.clone())
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("({})", item.count)),
                                )
                                .tooltip(move |window, cx| {
                                    Tooltip::new(tooltip_text.clone()).build(window, cx)
                                })
                        })
                        .child(
                            h_flex()
                                .gap_0p5()
                                .invisible()
                                .group_hover(group_name, |this| this.visible())
                                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .child(
                                    Button::new(SharedString::from(format!("ws-edit-{}", item.id)))
                                        .icon(IconName::Edit)
                                        .primary()
                                        .xsmall()
                                        .on_click(window.listener_for(
                                            &parent_for_edit,
                                            move |this, _, window, cx| {
                                                this.show_workspace_form(
                                                    Some(item_id_for_edit),
                                                    window,
                                                    cx,
                                                );
                                            },
                                        )),
                                )
                                .child(
                                    Button::new(SharedString::from(format!(
                                        "ws-delete-{}",
                                        item.id
                                    )))
                                    .icon(IconName::Remove)
                                    .danger()
                                    .xsmall()
                                    .on_click(
                                        window.listener_for(
                                            &parent_for_delete,
                                            move |this, _, window, cx| {
                                                this.delete_workspace(
                                                    item_id_for_delete,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ),
                                    ),
                                ),
                        ),
                ),
        )
    }

    fn set_selected_index(
        &mut self,
        _ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
    }

    fn cancel(&mut self, _window: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.parent.update(cx, |this, cx| {
            this.workspace_filter_open = false;
            cx.notify();
        });
    }
}

use crate::home_tab::HomePage;
use gpui::{App, Context, Entity, ParentElement, SharedString, Styled, Task, Window, div, px};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt, h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use one_core::storage::DatabaseType;
use rust_i18n::t;

/// 新建连接对话框中的连接类型选项
#[derive(Clone)]
enum NewConnectionKind {
    Workspace,
    Ssh,
    Terminal,
    Redis,
    MongoDB,
    Serial,
    Database(DatabaseType),
}

impl NewConnectionKind {
    fn label(&self) -> String {
        match self {
            NewConnectionKind::Workspace => t!("Workspace.label").to_string(),
            NewConnectionKind::Ssh => "SSH".to_string(),
            NewConnectionKind::Terminal => "Terminal".to_string(),
            NewConnectionKind::Redis => "Redis".to_string(),
            NewConnectionKind::MongoDB => "MongoDB".to_string(),
            NewConnectionKind::Serial => t!("Serial.new").to_string(),
            NewConnectionKind::Database(db_type) => db_type.as_str().to_string(),
        }
    }

    fn category(&self) -> &'static str {
        match self {
            NewConnectionKind::Workspace => "工作区",
            NewConnectionKind::Ssh | NewConnectionKind::Terminal | NewConnectionKind::Serial => {
                "终端"
            }
            NewConnectionKind::Redis | NewConnectionKind::MongoDB => "NoSQL",
            NewConnectionKind::Database(_) => "数据库",
        }
    }

    /// 在 HomePage 上执行对应的操作
    fn execute(&self, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        match self {
            NewConnectionKind::Workspace => {
                home.show_workspace_form(None, window, cx);
            }
            NewConnectionKind::Ssh => {
                home.editing_connection_id = None;
                home.show_ssh_form(window, cx);
            }
            NewConnectionKind::Terminal => {
                home.add_terminal_tab(window, cx);
            }
            NewConnectionKind::Redis => {
                home.editing_connection_id = None;
                home.show_redis_form(window, cx);
            }
            NewConnectionKind::MongoDB => {
                home.editing_connection_id = None;
                home.show_mongodb_form(window, cx);
            }
            NewConnectionKind::Serial => {
                home.editing_connection_id = None;
                home.show_serial_form(window, cx);
            }
            NewConnectionKind::Database(db_type) => {
                home.editing_connection_id = None;
                home.show_connection_form(*db_type, window, cx);
            }
        }
    }
}

pub(crate) struct NewConnectionDelegate {
    parent: Entity<HomePage>,
    items: Vec<NewConnectionKind>,
    filtered_items: Vec<NewConnectionKind>,
    selected_index: Option<IndexPath>,
    search_query: String,
}

impl NewConnectionDelegate {
    pub(crate) fn new(parent: Entity<HomePage>) -> Self {
        let mut items = vec![
            NewConnectionKind::Workspace,
            NewConnectionKind::Ssh,
            NewConnectionKind::Terminal,
            NewConnectionKind::Redis,
            NewConnectionKind::MongoDB,
            NewConnectionKind::Serial,
        ];

        for db_type in DatabaseType::all() {
            items.push(NewConnectionKind::Database(*db_type));
        }

        let filtered_items = items.clone();

        Self {
            parent,
            items,
            filtered_items,
            selected_index: None,
            search_query: String::new(),
        }
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_items = self.items.clone();
            return;
        }
        let query = self.search_query.to_lowercase();
        self.filtered_items = self
            .items
            .iter()
            .filter(|kind| {
                kind.label().to_lowercase().contains(&query)
                    || kind.category().to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
    }
}

impl ListDelegate for NewConnectionDelegate {
    type Item = ListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.search_query = query.to_string();
        self.apply_filter();
        cx.notify();
        Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let kind = self.filtered_items.get(ix.row)?.clone();
        let parent = self.parent.clone();
        let label = kind.label();
        let category = kind.category();

        Some(
            ListItem::new(ix)
                .px_3()
                .py_2()
                .rounded(px(6.0))
                .on_click(move |_, window, cx| {
                    parent.update(cx, |this, cx| {
                        kind.execute(this, window, cx);
                    });
                    window.close_dialog(cx);
                })
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_sm()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(SharedString::from(label)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(SharedString::from(category)),
                        ),
                ),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        if let Some(ix) = self.selected_index {
            if let Some(kind) = self.filtered_items.get(ix.row).cloned() {
                let parent = self.parent.clone();
                parent.update(cx, |this, cx| {
                    kind.execute(this, window, cx);
                });
                window.close_dialog(cx);
            }
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        window.close_dialog(cx);
    }
}

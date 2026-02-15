//! 命令历史面板
//!
//! 显示终端执行过的历史命令，支持搜索、保存片段和重新执行

use gpui::{
    div, px, uniform_list, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ListSizingBehavior, MouseButton, ParentElement, Render,
    SharedString, Styled, UniformListScrollHandle, Window,
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    v_flex, ActiveTheme, h_flex, Icon, IconName, Sizable, Size,
};
use one_core::storage::{traits::Repository, GlobalStorageState, TerminalCommand, TerminalCommandRepository};
use std::ops::Range;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 命令历史面板事件
#[derive(Clone, Debug)]
pub enum CommandHistoryEvent {
    /// 关闭面板
    Close,
    /// 执行命令（粘贴到终端）
    ExecuteCommand(String),
}

/// 命令历史面板组件
pub struct CommandHistoryPanel {
    /// 搜索输入框状态
    search_input_state: Entity<InputState>,
    /// 命令历史列表
    commands: Vec<TerminalCommand>,
    /// 过滤后的命令列表
    filtered_commands: Vec<TerminalCommand>,
    /// 连接 ID
    connection_id: Option<i64>,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 订阅
    _subscriptions: Vec<gpui::Subscription>,
    /// 是否正在加载
    is_loading: bool,
    /// 搜索关键词
    search_query: String,
    /// 列表滚动句柄
    scroll_handle: UniformListScrollHandle,
}

impl CommandHistoryPanel {
    pub fn new(
        connection_id: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search")
        });

        let mut subscriptions = Vec::new();

        // 订阅搜索输入事件
        let input_entity = search_input_state.clone();
        subscriptions.push(cx.subscribe_in(
            &search_input_state,
            window,
            move |this, _state, event, _window, cx| {
                if let InputEvent::Change = event {
                    let value = input_entity.read(cx).value().to_string();
                    this.search_query = value;
                    this.filter_commands();
                    cx.notify();
                }
            },
        ));

        let mut panel = Self {
            search_input_state,
            commands: Vec::new(),
            filtered_commands: Vec::new(),
            connection_id,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
            is_loading: false,
            search_query: String::new(),
            scroll_handle: UniformListScrollHandle::new(),
        };

        // 初始加载历史
        panel.load_history(cx);

        panel
    }

    /// 加载命令历史
    pub fn load_history(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;

        let storage = cx.global::<GlobalStorageState>().storage.clone();
        let connection_id = self.connection_id;

        let repo = match storage.get::<TerminalCommandRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("TerminalCommandRepository not found");
                self.is_loading = false;
                cx.notify();
                return;
            }
        };

        let result = if let Some(conn_id) = connection_id {
            repo.list_by_connection(conn_id, 100)
        } else {
            repo.list_unique_commands(None, 100).map(|unique_cmds| {
                unique_cmds
                    .into_iter()
                    .map(|cmd| TerminalCommand {
                        id: None,
                        session_id: None,
                        connection_id: None,
                        command: cmd,
                        working_directory: None,
                        executed_at: 0,
                        exit_code: None,
                        created_at: None,
                    })
                    .collect()
            })
        };

        match result {
            Ok(commands) => {
                self.commands = commands;
                self.filter_commands();
            }
            Err(e) => {
                tracing::error!("Failed to load history: {}", e);
            }
        }

        self.is_loading = false;
        cx.notify();
    }

    /// 添加命令到历史
    pub fn add_command(&mut self, command: String, cx: &mut Context<Self>) {
        if command.trim().is_empty() {
            return;
        }

        let connection_id = self.connection_id;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;

        let storage = cx.global::<GlobalStorageState>().storage.clone();

        let repo = match storage.get::<TerminalCommandRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("TerminalCommandRepository not found");
                return;
            }
        };

        let mut new_command = TerminalCommand {
            id: None,
            session_id: None,
            connection_id,
            command: command.clone(),
            working_directory: None,
            executed_at: now,
            exit_code: None,
            created_at: Some(now),
        };

        match repo.insert(&mut new_command) {
            Ok(_) => {
                self.commands.insert(0, new_command);
                self.filter_commands();
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to add command: {}", e);
            }
        }
    }

    /// 过滤命令列表
    fn filter_commands(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_commands = self.commands.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_commands = self.commands
                .iter()
                .filter(|cmd| cmd.command.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
    }

    /// 执行命令（粘贴到终端）
    fn paste_command(&self, command: String, cx: &mut Context<Self>) {
        cx.emit(CommandHistoryEvent::ExecuteCommand(command));
    }

    /// 渲染头部
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;
        let muted_bg = cx.theme().muted;
        let fg = cx.theme().foreground;

        h_flex()
            .flex_shrink_0()
            .w_full()
            .h(px(40.0))
            .px_3()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(border)
            .bg(muted_bg)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::new(IconName::BookOpen)
                            .with_size(Size::Small)
                            .text_color(fg)
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(fg)
                            .child("History")
                    )
            )
            .child(
                Button::new("close-history-panel")
                    .icon(IconName::Close)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_this, _, _, cx| {
                        cx.emit(CommandHistoryEvent::Close);
                    }))
            )
    }

    /// 渲染搜索栏
    fn render_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_query = !self.search_query.is_empty();
        let border = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;

        h_flex()
            .flex_shrink_0()
            .h_8()
            .px_2()
            .gap_2()
            .items_center()
            .border_b_1()
            .border_color(border)
            .child(
                Icon::new(IconName::Search)
                    .xsmall()
                    .text_color(muted_fg),
            )
            .child(
                div().flex_1().child(
                    Input::new(&self.search_input_state)
                        .xsmall()
                        .appearance(false)
                        .cleanable(has_query),
                ),
            )
    }

    /// 渲染命令项
    fn render_command_item(&self, index: usize, cmd: &TerminalCommand, cx: &mut Context<Self>) -> impl IntoElement {
        let command = cmd.command.clone();
        let command_for_paste = command.clone();
        let command_for_paste2 = command.clone();
        let item_id = SharedString::from(format!("cmd-item-{}", index));
        let group_name = SharedString::from(format!("cmd-group-{}", index));

        let highlight_color = cx.theme().accent;
        let muted_bg = cx.theme().muted;

        div()
            .id(item_id)
            .group(group_name.clone())
            .w_full()
            .px_3()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(muted_bg).text_color(highlight_color))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.paste_command(command_for_paste.clone(), cx);
            }))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(command)
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap_1()
                            .ml_2()
                            .invisible()
                            .group_hover(group_name, |s| s.visible())
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .child(
                                Button::new(SharedString::from(format!("paste-{}", index)))
                                    .label("PASTE")
                                    .ghost()
                                    .xsmall()
                                    .text_color(highlight_color)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.paste_command(command_for_paste2.clone(), cx);
                                    }))
                            )
                    )
            )
    }

    /// 渲染空状态
    fn render_empty_state(&self, cx: &App) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        let search_empty = self.search_query.is_empty();

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                Icon::new(IconName::BookOpen)
                    .with_size(Size::Large)
                    .text_color(muted_fg)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted_fg)
                    .child(if search_empty {
                        "No commands yet"
                    } else {
                        "No matching commands"
                    })
            )
    }

    /// 渲染加载状态
    fn render_loading_state(&self, cx: &App) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                Icon::new(IconName::Loader)
                    .with_size(Size::Medium)
                    .text_color(muted_fg)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted_fg)
                    .child("Loading...")
            )
    }
}

impl EventEmitter<CommandHistoryEvent> for CommandHistoryPanel {}

impl Focusable for CommandHistoryPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandHistoryPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_loading = self.is_loading;
        let commands_empty = self.filtered_commands.is_empty();
        let item_count = self.filtered_commands.len();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_header(cx))
            .child(self.render_search_bar(cx))
            .when(is_loading, |this| {
                this.child(self.render_loading_state(cx))
            })
            .when(!is_loading && commands_empty, |this| {
                this.child(self.render_empty_state(cx))
            })
            .when(!is_loading && !commands_empty, |this| {
                this.child(
                    uniform_list("command-history-list", item_count, {
                        cx.processor(move |state: &mut Self, range: Range<usize>, _window, cx| {
                            range
                                .map(|ix| {
                                    let cmd = state.filtered_commands[ix].clone();
                                    state.render_command_item(ix, &cmd, cx)
                                })
                                .collect()
                        })
                    })
                    .flex_1()
                    .size_full()
                    .px_2()
                    .py_1()
                    .track_scroll(&self.scroll_handle)
                    .with_sizing_behavior(ListSizingBehavior::Auto),
                )
            })
    }
}

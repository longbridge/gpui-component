//! 快捷命令面板
//!
//! 支持命令的新增、置顶、删除功能

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
use one_core::storage::{traits::Repository, GlobalStorageState, QuickCommand, QuickCommandRepository};
use std::ops::Range;

/// 快捷命令面板事件
#[derive(Clone, Debug)]
pub enum QuickCommandPanelEvent {
    /// 关闭面板
    Close,
    /// 执行命令（粘贴到终端）
    ExecuteCommand(String),
}

/// 快捷命令面板组件
pub struct QuickCommandPanel {
    /// 搜索输入框状态
    search_input_state: Entity<InputState>,
    /// 新增命令输入框状态
    add_input_state: Entity<InputState>,
    /// 快捷命令列表
    commands: Vec<QuickCommand>,
    /// 过滤后的命令列表
    filtered_commands: Vec<QuickCommand>,
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
    /// 是否显示新增输入框
    show_add_input: bool,
    /// 列表滚动句柄
    scroll_handle: UniformListScrollHandle,
}

impl QuickCommandPanel {
    pub fn new(
        connection_id: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search")
        });

        let add_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter command...")
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
            add_input_state,
            commands: Vec::new(),
            filtered_commands: Vec::new(),
            connection_id,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
            is_loading: false,
            search_query: String::new(),
            show_add_input: false,
            scroll_handle: UniformListScrollHandle::new(),
        };

        // 初始加载
        panel.load_commands(cx);

        panel
    }

    /// 加载快捷命令
    pub fn load_commands(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;

        let storage = cx.global::<GlobalStorageState>().storage.clone();
        let connection_id = self.connection_id;

        let repo = match storage.get::<QuickCommandRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("QuickCommandRepository not found");
                self.is_loading = false;
                cx.notify();
                return;
            }
        };

        match repo.list_by_connection(connection_id) {
            Ok(commands) => {
                self.commands = commands;
                self.filter_commands();
            }
            Err(e) => {
                tracing::error!("Failed to load commands: {}", e);
            }
        }

        self.is_loading = false;
        cx.notify();
    }

    /// 添加快捷命令
    fn add_command(&mut self, command: String, cx: &mut Context<Self>) {
        if command.trim().is_empty() {
            return;
        }

        let connection_id = self.connection_id;
        let storage = cx.global::<GlobalStorageState>().storage.clone();

        // 创建新命令
        let mut new_command = QuickCommand::new(command.clone());
        new_command.connection_id = connection_id;

        // 持久化
        let repo = match storage.get::<QuickCommandRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("QuickCommandRepository not found");
                return;
            }
        };

        let next_order = repo.next_sort_order(connection_id).unwrap_or(0);
        new_command.sort_order = next_order;

        match repo.insert(&mut new_command) {
            Ok(_) => {
                // 添加到列表前面
                self.commands.insert(0, new_command);
                self.filter_commands();
                self.show_add_input = false;
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to add command: {}", e);
            }
        }
    }

    /// 从外部添加快捷命令（例如右键菜单）
    pub fn add_command_external(&mut self, command: String, cx: &mut Context<Self>) {
        self.add_command(command, cx);
    }

    /// 删除快捷命令
    fn delete_command(&mut self, id: i64, cx: &mut Context<Self>) {
        let storage = cx.global::<GlobalStorageState>().storage.clone();

        let repo = match storage.get::<QuickCommandRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("QuickCommandRepository not found");
                return;
            }
        };

        match repo.delete(id) {
            Ok(_) => {
                self.commands.retain(|cmd| cmd.id != Some(id));
                self.filter_commands();
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to delete command: {}", e);
            }
        }
    }

    /// 切换置顶状态
    fn toggle_pin(&mut self, id: i64, cx: &mut Context<Self>) {
        let storage = cx.global::<GlobalStorageState>().storage.clone();

        let repo = match storage.get::<QuickCommandRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("QuickCommandRepository not found");
                return;
            }
        };

        match repo.toggle_pin(id) {
            Ok(_) => {
                if let Some(cmd) = self.commands.iter_mut().find(|c| c.id == Some(id)) {
                    cmd.pinned = !cmd.pinned;
                }

                self.commands.sort_by(|a, b| {
                    match (a.pinned, b.pinned) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.sort_order.cmp(&b.sort_order),
                    }
                });

                self.filter_commands();
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to toggle pin: {}", e);
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
                .filter(|cmd| {
                    cmd.command.to_lowercase().contains(&query)
                        || cmd.name.as_ref().map(|n| n.to_lowercase().contains(&query)).unwrap_or(false)
                        || cmd.description.as_ref().map(|d| d.to_lowercase().contains(&query)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    /// 执行命令（粘贴到终端）
    fn paste_command(&self, command: String, cx: &mut Context<Self>) {
        cx.emit(QuickCommandPanelEvent::ExecuteCommand(command));
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
                        Icon::new(IconName::SquareTerminal)
                            .with_size(Size::Small)
                            .text_color(fg)
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(fg)
                            .child("Quick Commands")
                    )
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("add-command")
                            .icon(IconName::Plus)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_add_input = true;
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("close-quick-command-panel")
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|_this, _, _, cx| {
                                cx.emit(QuickCommandPanelEvent::Close);
                            }))
                    )
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

    /// 渲染新增命令输入框
    fn render_add_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let add_input = self.add_input_state.clone();
        let border = cx.theme().border;
        let muted_bg = cx.theme().muted;

        h_flex()
            .flex_shrink_0()
            .h_8()
            .px_2()
            .gap_1()
            .items_center()
            .border_b_1()
            .border_color(border)
            .bg(muted_bg)
            .child(
                div().flex_1().child(
                    Input::new(&self.add_input_state)
                        .appearance(false)
                        .xsmall(),
                ),
            )
            .child(
                Button::new("cancel-add")
                    .label("Cancel")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_input_state.update(cx, |state, cx| {
                            state.set_value("", window, cx);
                        });
                        this.show_add_input = false;
                        cx.notify();
                    }))
            )
            .child(
                Button::new("confirm-add")
                    .label("Add")
                    .primary()
                    .xsmall()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let value = add_input.read(cx).value().to_string();
                        if !value.trim().is_empty() {
                            this.add_command(value, cx);
                            add_input.update(cx, |state, cx| {
                                state.set_value("", window, cx);
                            });
                        }
                    }))
            )
    }

    /// 渲染单个命令项（供 uniform_list 使用）
    fn render_command_item(&self, index: usize, cmd: &QuickCommand, cx: &mut Context<Self>) -> impl IntoElement {
        let command = cmd.command.clone();
        let command_for_paste = command.clone();
        let command_for_paste2 = command.clone();
        let id = cmd.id.unwrap_or(0);
        let is_pinned = cmd.pinned;
        let item_id = SharedString::from(format!("quick-cmd-item-{}", index));
        let group_name = SharedString::from(format!("quick-cmd-group-{}", index));

        let highlight_color = cx.theme().accent;
        let pin_color = cx.theme().warning;
        let muted_bg = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;

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
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .items_center()
                            .when(is_pinned, |this| {
                                this.child(
                                    Icon::new(IconName::Star)
                                        .with_size(Size::XSmall)
                                        .text_color(pin_color)
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(command)
                            )
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
                                Button::new(SharedString::from(format!("pin-{}", index)))
                                    .icon(if is_pinned { IconName::StarOff } else { IconName::Star })
                                    .ghost()
                                    .xsmall()
                                    .when(is_pinned, |this| this.text_color(pin_color))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.toggle_pin(id, cx);
                                    }))
                            )
                            .child(
                                Button::new(SharedString::from(format!("delete-{}", index)))
                                    .icon(IconName::Remove)
                                    .ghost()
                                    .xsmall()
                                    .text_color(muted_fg)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.delete_command(id, cx);
                                    }))
                            )
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
                Icon::new(IconName::SquareTerminal)
                    .with_size(Size::Large)
                    .text_color(muted_fg)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted_fg)
                    .child(if search_empty {
                        "No commands yet. Click + to add one."
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

impl EventEmitter<QuickCommandPanelEvent> for QuickCommandPanel {}

impl Focusable for QuickCommandPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuickCommandPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_add = self.show_add_input;
        let is_loading = self.is_loading;
        let commands_empty = self.filtered_commands.is_empty();
        let item_count = self.filtered_commands.len();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_header(cx))
            .child(self.render_search_bar(cx))
            .when(show_add, |this| {
                this.child(self.render_add_input(cx))
            })
            .when(is_loading, |this| {
                this.child(self.render_loading_state(cx))
            })
            .when(!is_loading && commands_empty, |this| {
                this.child(self.render_empty_state(cx))
            })
            .when(!is_loading && !commands_empty, |this| {
                this.child(
                    uniform_list("quick-command-list", item_count, {
                        cx.processor(move |state: &mut Self, range: Range<usize>, _window, _cx| {
                            range
                                .map(|ix| {
                                    let cmd = state.filtered_commands[ix].clone();
                                    state.render_command_item(ix, &cmd, _cx)
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

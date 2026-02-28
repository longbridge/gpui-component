//! Redis 键值视图

use crate::{
    GlobalRedisState, HashField, KeyInfo, KeyValueContent, KeyValueDetail, RedisKeyType, ZSetMember,
};
use gpui::{
    App, AppContext, AsyncApp, ClipboardItem, Context, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Task, Window, div, prelude::FluentBuilder, px, relative,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Sizable, Size, WindowExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    dialog::DialogButtonProps,
    h_flex,
    highlighter::Language,
    input::{Input, InputEvent, InputState},
    radio::Radio,
    select::{Select, SelectEvent, SelectItem, SelectState},
    spinner::Spinner,
    v_flex,
};
use one_core::gpui_tokio::Tokio;
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;

/// 键值视图事件
#[derive(Clone, Debug)]
pub enum KeyValueViewEvent {
    /// 值已更新
    ValueUpdated { key: String },
    /// 值已删除
    ValueDeleted { key: String },
}

/// 加载状态
#[derive(Clone, Debug, PartialEq)]
enum LoadState {
    Empty,
    Loading,
    Loaded,
    Error(String),
}

/// 查看格式
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ViewFormat {
    #[default]
    Raw,
    Json,
    Hex,
    Binary,
}

impl ViewFormat {
    pub fn all() -> Vec<Self> {
        vec![
            ViewFormat::Raw,
            ViewFormat::Json,
            ViewFormat::Hex,
            ViewFormat::Binary,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ViewFormat::Raw => "Raw",
            ViewFormat::Json => "JSON",
            ViewFormat::Hex => "Hex",
            ViewFormat::Binary => "Binary",
        }
    }
}

impl SelectItem for ViewFormat {
    type Value = ViewFormat;

    fn title(&self) -> SharedString {
        self.display_name().into()
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

/// List 插入位置
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListInsertPosition {
    /// 头部插入 (LPUSH)
    #[default]
    Head,
    /// 尾部插入 (RPUSH)
    Tail,
}

/// 排序方向
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// ZSet 排序字段
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZSetSortBy {
    #[default]
    Score,
    Member,
}

/// 键值视图
pub struct KeyValueView {
    /// 当前连接 ID
    connection_id: Option<String>,
    /// 当前数据库索引
    db_index: u8,
    /// 当前键名
    current_key: Option<String>,
    /// 键信息
    key_info: Option<KeyInfo>,
    /// 键值内容
    value_content: Option<KeyValueContent>,
    /// 加载状态
    load_state: LoadState,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 是否修改过
    is_dirty: bool,
    /// 当前查看格式
    view_format: ViewFormat,
    /// 查看格式选择器状态
    format_select: Entity<SelectState<Vec<ViewFormat>>>,
    /// String 值编辑器状态
    string_editor: Entity<InputState>,
    /// 待设置的编辑器值（异步加载完成后设置）
    pending_editor_value: Option<String>,

    // === 筛选功能 ===
    /// 筛选输入框状态
    filter_input: Entity<InputState>,
    /// 当前筛选文本
    filter_text: String,
    /// 是否全文匹配
    filter_exact_match: bool,

    // === 排序功能 ===
    /// 排序方向
    sort_order: SortOrder,
    /// ZSet 排序字段
    zset_sort_by: ZSetSortBy,

    // === List 插入位置 ===
    list_insert_position: ListInsertPosition,
    /// 是否允许关闭标签页
    closeable: bool,
}

impl KeyValueView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_closeable(false, window, cx)
    }

    pub fn new_with_closeable(
        closeable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let format_select = cx.new(|cx| {
            SelectState::new(
                ViewFormat::all(),
                Some(IndexPath::default().row(0)),
                window,
                cx,
            )
        });

        let string_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor(Language::from_str("text"))
                .line_number(true)
                .searchable(true)
                .soft_wrap(true)
                .placeholder(t!("KeyValueView.select_key_placeholder").to_string())
        });

        let filter_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("KeyValueView.filter_placeholder").to_string())
        });

        cx.subscribe_in(&format_select, window, Self::on_format_changed)
            .detach();

        cx.subscribe_in(&string_editor, window, Self::on_editor_changed)
            .detach();

        cx.subscribe_in(&filter_input, window, Self::on_filter_changed)
            .detach();

        Self {
            connection_id: None,
            db_index: 0,
            current_key: None,
            key_info: None,
            value_content: None,
            load_state: LoadState::Empty,
            focus_handle: cx.focus_handle(),
            is_dirty: false,
            view_format: ViewFormat::Raw,
            format_select,
            string_editor,
            pending_editor_value: None,
            filter_input,
            filter_text: String::new(),
            filter_exact_match: false,
            sort_order: SortOrder::Asc,
            zset_sort_by: ZSetSortBy::Score,
            list_insert_position: ListInsertPosition::Tail,
            closeable,
        }
    }

    /// 格式选择器变化处理
    fn on_format_changed(
        &mut self,
        _select: &Entity<SelectState<Vec<ViewFormat>>>,
        event: &SelectEvent<Vec<ViewFormat>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let SelectEvent::Confirm(Some(format)) = event {
            self.view_format = *format.value();
            self.update_editor_content(window, cx);
            self.update_editor_highlighter(cx);
            cx.notify();
        }
    }

    /// 编辑器内容变化处理
    fn on_editor_changed(
        &mut self,
        _editor: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            self.is_dirty = true;
            cx.notify();
        }
    }

    /// 筛选输入变化处理
    fn on_filter_changed(
        &mut self,
        _input: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            self.filter_text = self.filter_input.read(cx).text().to_string();
            cx.notify();
        }
    }

    /// 应用筛选到字符串列表
    fn apply_filter(&self, items: &[String]) -> Vec<(usize, String)> {
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if self.filter_text.is_empty() {
                    return true;
                }
                if self.filter_exact_match {
                    item.contains(&self.filter_text)
                } else {
                    item.to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                }
            })
            .map(|(idx, s)| (idx, s.clone()))
            .collect()
    }

    /// 应用筛选到 Hash 字段
    fn apply_filter_hash(&self, items: &[HashField]) -> Vec<(usize, HashField)> {
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if self.filter_text.is_empty() {
                    return true;
                }
                let search_text = format!("{} {}", item.field, item.value);
                if self.filter_exact_match {
                    search_text.contains(&self.filter_text)
                } else {
                    search_text
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                }
            })
            .map(|(idx, f)| (idx, f.clone()))
            .collect()
    }

    /// 应用筛选到 ZSet 成员
    fn apply_filter_zset(&self, items: &[ZSetMember]) -> Vec<(usize, ZSetMember)> {
        let mut filtered: Vec<(usize, ZSetMember)> = items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if self.filter_text.is_empty() {
                    return true;
                }
                if self.filter_exact_match {
                    item.member.contains(&self.filter_text)
                } else {
                    item.member
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                }
            })
            .map(|(idx, m)| (idx, m.clone()))
            .collect();

        // 应用排序
        match self.zset_sort_by {
            ZSetSortBy::Score => {
                filtered.sort_by(|a, b| {
                    if self.sort_order == SortOrder::Asc {
                        a.1.score
                            .partial_cmp(&b.1.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        b.1.score
                            .partial_cmp(&a.1.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }
                });
            }
            ZSetSortBy::Member => {
                filtered.sort_by(|a, b| {
                    if self.sort_order == SortOrder::Asc {
                        a.1.member.cmp(&b.1.member)
                    } else {
                        b.1.member.cmp(&a.1.member)
                    }
                });
            }
        }
        filtered
    }

    /// 切换排序方向
    fn toggle_sort_order(&mut self, cx: &mut Context<Self>) {
        self.sort_order = match self.sort_order {
            SortOrder::Asc => SortOrder::Desc,
            SortOrder::Desc => SortOrder::Asc,
        };
        cx.notify();
    }

    /// 切换 ZSet 排序字段
    fn toggle_zset_sort_by(&mut self, cx: &mut Context<Self>) {
        self.zset_sort_by = match self.zset_sort_by {
            ZSetSortBy::Score => ZSetSortBy::Member,
            ZSetSortBy::Member => ZSetSortBy::Score,
        };
        cx.notify();
    }

    /// 更新编辑器内容（根据格式转换）
    fn update_editor_content(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(KeyValueContent::String(ref value)) = self.value_content else {
            return;
        };

        let formatted = self.format_value(value);

        self.string_editor.update(cx, |state, cx| {
            state.set_value(formatted, window, cx);
        });
    }

    /// 更新编辑器高亮语言
    fn update_editor_highlighter(&mut self, cx: &mut Context<Self>) {
        let language = match self.view_format {
            ViewFormat::Json => Language::Json,
            _ => Language::from_str("text"),
        };

        self.string_editor.update(cx, |state, cx| {
            state.set_highlighter(language, cx);
        });
    }

    /// 格式化值
    fn format_value(&self, value: &str) -> String {
        match self.view_format {
            ViewFormat::Raw => value.to_string(),
            ViewFormat::Json => match serde_json::from_str::<serde_json::Value>(value) {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| value.to_string()),
                Err(_) => value.to_string(),
            },
            ViewFormat::Hex => value
                .bytes()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" "),
            ViewFormat::Binary => value
                .bytes()
                .map(|b| format!("{:08b}", b))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// 加载键
    pub fn load_key(
        &mut self,
        connection_id: String,
        db_index: u8,
        key: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_id = Some(connection_id.clone());
        self.db_index = db_index;
        self.current_key = Some(key.clone());
        self.load_state = LoadState::Loading;
        self.is_dirty = false;
        self.pending_editor_value = None;
        cx.notify();

        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                async move { Self::fetch_key_value(&global_state, &connection_id, &key).await }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                match result {
                    Ok(detail) => {
                        if let KeyValueContent::String(ref value) = detail.value {
                            view.pending_editor_value = Some(value.clone());
                        }
                        view.key_info = Some(detail.key_info);
                        view.value_content = Some(detail.value);
                        view.load_state = LoadState::Loaded;
                    }
                    Err(e) => {
                        view.load_state = LoadState::Error(e.to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 在 render 中应用待设置的编辑器值
    fn apply_pending_editor_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_editor_value.take() {
            let formatted = self.format_value(&value);
            self.string_editor.update(cx, |state, cx| {
                state.set_value(formatted, window, cx);
            });
        }
    }

    /// 获取键值
    async fn fetch_key_value(
        global_state: &GlobalRedisState,
        connection_id: &str,
        key: &str,
    ) -> anyhow::Result<KeyValueDetail> {
        let conn = global_state
            .get_connection(connection_id)
            .ok_or_else(|| anyhow::anyhow!("{}", t!("KeyValueView.connection_missing")))?;

        let guard = conn.read().await;
        guard
            .get_key_value_detail(key)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 获取编辑器内容
    fn get_editor_content(&self, cx: &App) -> String {
        self.string_editor.read(cx).text().to_string()
    }

    /// 渲染键信息面板（工具栏）
    fn render_key_info(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(info) = &self.key_info else {
            return div().into_any_element();
        };

        let key_name = info.name.clone();
        let key_type = info.key_type;
        let key_type_display = info.key_type.display_name();
        let ttl_display = info.ttl_display();
        let view = cx.entity().clone();
        let key_for_copy = key_name.clone();
        let editor_content = self.get_editor_content(cx);
        let can_add_element = matches!(
            key_type,
            RedisKeyType::List | RedisKeyType::Set | RedisKeyType::ZSet | RedisKeyType::Hash
        );
        let is_string = matches!(key_type, RedisKeyType::String);
        let is_zset = matches!(key_type, RedisKeyType::ZSet);
        let zset_sort_label = match self.zset_sort_by {
            ZSetSortBy::Score => t!("KeyValueView.sort_by_score"),
            ZSetSortBy::Member => t!("KeyValueView.sort_by_member"),
        };
        let zset_sort_text = t!("KeyValueView.sort_by", by = zset_sort_label);

        v_flex()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted)
            // 第一行：类型 + 键名 + 操作按钮
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .gap_2()
                    .items_center()
                    // 类型徽章
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded(px(4.0))
                            .bg(cx.theme().primary)
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(cx.theme().primary_foreground)
                            .child(key_type_display),
                    )
                    // 键名
                    .child(div().flex_1().text_sm().truncate().child(key_name.clone()))
                    // 刷新按钮
                    .child(
                        Button::new("refresh-key")
                            .icon(IconName::Refresh)
                            .ghost()
                            .with_size(Size::Medium)
                            .on_click({
                                let view = view.clone();
                                move |_, _, cx| {
                                    view.update(cx, |view, cx| {
                                        if let (Some(conn_id), Some(key)) =
                                            (view.connection_id.clone(), view.current_key.clone())
                                        {
                                            view.load_key(conn_id, view.db_index, key, cx);
                                        }
                                    });
                                }
                            }),
                    )
                    // 复制键名按钮
                    .child(
                        Button::new("copy-key")
                            .icon(IconName::Copy)
                            .ghost()
                            .with_size(Size::Medium)
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    key_for_copy.clone(),
                                ));
                            }),
                    )
                    // TTL 显示（可点击编辑）
                    .child(
                        Button::new("ttl-display")
                            .ghost()
                            .with_size(Size::Medium)
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(cx.theme().secondary)
                                    .child(
                                        Icon::new(IconName::Calendar)
                                            .with_size(Size::Small)
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().secondary_foreground)
                                            .child(ttl_display),
                                    ),
                            )
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    view.update(cx, |view, cx| {
                                        view.show_ttl_dialog(window, cx);
                                    });
                                }
                            }),
                    )
                    // 重命名按钮
                    .child(
                        Button::new("rename-key")
                            .icon(IconName::Edit)
                            .ghost()
                            .with_size(Size::Medium)
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    view.update(cx, |view, cx| {
                                        view.show_rename_dialog(window, cx);
                                    });
                                }
                            }),
                    )
                    // 删除按钮
                    .child(
                        Button::new("delete-key")
                            .icon(IconName::Remove)
                            .ghost()
                            .with_size(Size::Medium)
                            .on_click({
                                let view = view.clone();
                                move |_, _, cx| {
                                    view.update(cx, |view, cx| {
                                        cx.emit(KeyValueViewEvent::ValueDeleted {
                                            key: view.current_key.clone().unwrap_or_default(),
                                        });
                                    });
                                }
                            }),
                    ),
            )
            // 第二行：筛选 + 操作按钮
            .child(
                h_flex()
                    .w_full()
                    .px_2()
                    .pb_2()
                    .gap_2()
                    .items_center()
                    // 左侧：筛选输入框（集合类型）或格式选择器（String 类型）
                    .child(
                        h_flex()
                            .flex_1()
                            .gap_2()
                            .items_center()
                            .when(is_string, |this| {
                                this.child(
                                    div()
                                        .text_base()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(t!("KeyValueView.view_mode").to_string()),
                                )
                                .child(
                                    Select::new(&self.format_select)
                                        .with_size(Size::Medium)
                                        .w(px(100.)),
                                )
                            })
                            .when(can_add_element, |this| {
                                this.child(
                                    Input::new(&self.filter_input)
                                        .with_size(Size::Medium)
                                        .w(px(200.)),
                                )
                                .child(
                                    Checkbox::new("exact-match")
                                        .label(t!("KeyValueView.exact_match").to_string())
                                        .with_size(Size::Medium)
                                        .checked(self.filter_exact_match)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |view, cx| {
                                                    view.filter_exact_match =
                                                        !view.filter_exact_match;
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                            }),
                    )
                    // 右侧按钮组
                    .child(
                        h_flex()
                            .gap_2()
                            // 排序按钮（集合类型）
                            .when(can_add_element, |this| {
                                this.child(
                                    Button::new("sort-order")
                                        .icon(if self.sort_order == SortOrder::Asc {
                                            IconName::SortAscending
                                        } else {
                                            IconName::SortDescending
                                        })
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |view, cx| {
                                                    view.toggle_sort_order(cx);
                                                });
                                            }
                                        }),
                                )
                            })
                            .when(is_zset, |this| {
                                this.child(
                                    Button::new("zset-sort-by")
                                        .icon(IconName::ChevronsUpDown)
                                        .label(zset_sort_text.to_string())
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |view, cx| {
                                                    view.toggle_zset_sort_by(cx);
                                                });
                                            }
                                        }),
                                )
                            })
                            // 插入行按钮（仅对集合类型显示）
                            .when(can_add_element, |this| {
                                this.child(
                                    Button::new("add-element")
                                        .icon(IconName::Plus)
                                        .label(t!("KeyValueView.insert_row").to_string())
                                        .primary()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, window, cx| {
                                                view.update(cx, |view, cx| {
                                                    view.show_add_dialog(window, cx);
                                                });
                                            }
                                        }),
                                )
                            })
                            // 复制值按钮
                            .child(
                                Button::new("copy-value")
                                    .icon(IconName::Copy)
                                    .label(t!("KeyValueView.copy_value").to_string())
                                    .ghost()
                                    .with_size(Size::Medium)
                                    .on_click({
                                        let content = editor_content.clone();
                                        move |_, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                content.clone(),
                                            ));
                                        }
                                    }),
                            )
                            // 保存按钮（仅 String 类型显示）
                            .when(is_string && self.is_dirty, |this| {
                                this.child(
                                    Button::new("save-value")
                                        .icon(IconName::Check)
                                        .label(t!("KeyValueView.save").to_string())
                                        .success()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |view, cx| {
                                                    view.save_string_value(cx);
                                                });
                                            }
                                        }),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    /// 显示新增对话框（根据当前键类型分发）
    fn show_add_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(info) = &self.key_info else { return };
        match info.key_type {
            RedisKeyType::List => self.show_list_add_dialog(window, cx),
            RedisKeyType::Set => self.show_set_add_dialog(window, cx),
            RedisKeyType::ZSet => self.show_zset_add_dialog(window, cx),
            RedisKeyType::Hash => self.show_hash_add_dialog(window, cx),
            _ => {}
        }
    }

    // === List 对话框 ===

    /// 显示 List 添加对话框
    fn show_list_add_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("KeyValueView.input_value_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8)
        });
        let view = cx.entity().downgrade();
        let position = self.list_insert_position;

        // 在打开对话框前设置焦点，避免闪烁
        value_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = value_input.clone();
            let view_for_ok = view.clone();

            dialog
                .title(t!("KeyValueView.list_insert_title").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            h_flex()
                                .gap_4()
                                .child(
                                    Radio::new("insert-head")
                                        .label(t!("KeyValueView.list_insert_head").to_string())
                                        .checked(position == ListInsertPosition::Head),
                                )
                                .child(
                                    Radio::new("insert-tail")
                                        .label(t!("KeyValueView.list_insert_tail").to_string())
                                        .checked(position == ListInsertPosition::Tail),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.value_label").to_string()),
                                )
                                .child(Input::new(&value_input).w_full()),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.add").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let value = input_for_ok.read(cx).text().to_string();
                    if value.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.add_list_element(value, position, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 显示 List 编辑对话框
    fn show_list_edit_dialog(
        &mut self,
        index: usize,
        current_value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.input_value_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8);
            state.set_value(current_value, window, cx);
            state
        });
        let view = cx.entity().downgrade();

        // 在打开对话框前设置焦点，避免闪烁
        value_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = value_input.clone();
            let view_for_ok = view.clone();

            dialog
                .title(t!("KeyValueView.edit_list_item", index = index + 1).to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .child(t!("KeyValueView.value_label").to_string()),
                        )
                        .child(Input::new(&value_input).w_full()),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.save").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let value = input_for_ok.read(cx).text().to_string();
                    if value.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.edit_list_element(index, value, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 添加 List 元素
    fn add_list_element(
        &mut self,
        value: String,
        position: ListInsertPosition,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let value = value.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    match position {
                        ListInsertPosition::Head => {
                            guard
                                .lpush(&key, &[value.as_str()])
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                        }
                        ListInsertPosition::Tail => {
                            guard
                                .rpush(&key, &[value.as_str()])
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                        }
                    }
                    Ok::<(), anyhow::Error>(())
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 编辑 List 元素（通过 LSET）
    fn edit_list_element(&mut self, index: usize, new_value: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let new_value = new_value.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .lset(&key, index as i64, &new_value)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    // === Set 对话框 ===

    /// 显示 Set 添加对话框
    fn show_set_add_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("KeyValueView.set_member_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8)
        });
        let view = cx.entity().downgrade();

        // 在打开对话框前设置焦点，避免闪烁
        value_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = value_input.clone();
            let view_for_ok = view.clone();

            dialog
                .title(t!("KeyValueView.add_set_member").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .child(t!("KeyValueView.member_label").to_string()),
                        )
                        .child(Input::new(&value_input).w_full()),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.add").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let member = input_for_ok.read(cx).text().to_string();
                    if member.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.add_set_member(member, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 添加 Set 成员
    fn add_set_member(&mut self, member: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let member = member.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .sadd(&key, &[member.as_str()])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    // === ZSet 对话框 ===

    /// 显示 ZSet 添加对话框
    fn show_zset_add_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let score_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.score_placeholder").to_string());
            state.set_value("0", window, cx);
            state
        });
        let member_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("KeyValueView.set_member_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8)
        });
        let view = cx.entity().downgrade();

        // 在打开对话框前设置焦点，避免闪烁
        score_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let score_for_ok = score_input.clone();
            let member_for_ok = member_input.clone();
            let view_for_ok = view.clone();

            dialog
                .title(t!("KeyValueView.add_zset_member").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.score_label").to_string()),
                                )
                                .child(Input::new(&score_input).w_full()),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.member_label").to_string()),
                                )
                                .child(Input::new(&member_input).w_full()),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.add").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let member = member_for_ok.read(cx).text().to_string();
                    let score_str = score_for_ok.read(cx).text().to_string();
                    let score: f64 = score_str.parse().unwrap_or(0.0);
                    if member.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.add_zset_member(member, score, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 显示 ZSet 编辑对话框
    fn show_zset_edit_dialog(
        &mut self,
        member: String,
        current_score: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let score_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.score_placeholder").to_string());
            state.set_value(format!("{}", current_score), window, cx);
            state
        });
        let member_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.zset_member_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8);
            state.set_value(member.clone(), window, cx);
            state
        });
        let view = cx.entity().downgrade();
        let old_member = member;

        // 在打开对话框前设置焦点，避免闪烁
        score_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let score_for_ok = score_input.clone();
            let member_for_ok = member_input.clone();
            let view_for_ok = view.clone();
            let old_member_for_ok = old_member.clone();

            dialog
                .title(t!("KeyValueView.edit_zset_member").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.score_label").to_string()),
                                )
                                .child(Input::new(&score_input).w_full()),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.member_label").to_string()),
                                )
                                .child(Input::new(&member_input).w_full()),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.save").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let new_member = member_for_ok.read(cx).text().to_string();
                    let score_str = score_for_ok.read(cx).text().to_string();
                    let score: f64 = score_str.parse().unwrap_or(0.0);
                    if new_member.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.update_zset_member(old_member_for_ok.clone(), new_member, score, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 添加 ZSet 成员
    fn add_zset_member(&mut self, member: String, score: f64, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let member = member.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .zadd(&key, &[(score, member.as_str())])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 更新 ZSet 成员（删除旧成员后添加新成员）
    fn update_zset_member(
        &mut self,
        old_member: String,
        new_member: String,
        score: f64,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let old_member = old_member.clone();
                let new_member = new_member.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    // 如果成员名变了，先删除旧的
                    if old_member != new_member {
                        guard
                            .zrem(&key, &[old_member.as_str()])
                            .await
                            .map_err(|e| anyhow::anyhow!("{}", e))?;
                    }
                    guard
                        .zadd(&key, &[(score, new_member.as_str())])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    Ok::<(), anyhow::Error>(())
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    // === Hash 对话框 ===

    /// 显示 Hash 添加对话框
    fn show_hash_add_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let field_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("KeyValueView.hash_field_placeholder").to_string())
        });
        let value_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("KeyValueView.input_value_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8)
        });
        let view = cx.entity().downgrade();

        // 在打开对话框前设置焦点，避免闪烁
        field_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let field_for_ok = field_input.clone();
            let value_for_ok = value_input.clone();
            let view_for_ok = view.clone();

            dialog
                .title(t!("KeyValueView.add_hash_field").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.field_label").to_string()),
                                )
                                .child(Input::new(&field_input).w_full()),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.value_label").to_string()),
                                )
                                .child(Input::new(&value_input).w_full()),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.add").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let field = field_for_ok.read(cx).text().to_string();
                    let value = value_for_ok.read(cx).text().to_string();
                    if field.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.set_hash_field(field, value, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 显示 Hash 编辑对话框
    fn show_hash_edit_dialog(
        &mut self,
        field: String,
        current_value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let field_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.hash_field_placeholder_edit").to_string());
            state.set_value(field.clone(), window, cx);
            state
        });
        let value_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.input_value_placeholder").to_string())
                .multi_line(true)
                .auto_grow(3, 8);
            state.set_value(current_value, window, cx);
            state
        });
        let view = cx.entity().downgrade();
        let old_field = field;

        // 在打开对话框前设置焦点，避免闪烁
        value_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let field_for_ok = field_input.clone();
            let value_for_ok = value_input.clone();
            let view_for_ok = view.clone();
            let old_field_for_ok = old_field.clone();

            dialog
                .title(t!("KeyValueView.edit_hash_field").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.field_label").to_string()),
                                )
                                .child(Input::new(&field_input).w_full()),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.value_label").to_string()),
                                )
                                .child(Input::new(&value_input).w_full()),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("KeyValueView.save").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let new_field = field_for_ok.read(cx).text().to_string();
                    let value = value_for_ok.read(cx).text().to_string();
                    if new_field.is_empty() {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        // 如果字段名变了，先删除旧字段
                        if old_field_for_ok != new_field {
                            v.delete_hash_field_then_set(
                                old_field_for_ok.clone(),
                                new_field.clone(),
                                value.clone(),
                                cx,
                            );
                        } else {
                            v.set_hash_field(new_field, value, cx);
                        }
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 设置 Hash 字段
    fn set_hash_field(&mut self, field: String, value: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let field = field.clone();
                let value = value.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .hset(&key, &field, &value)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 删除旧 Hash 字段并设置新字段
    fn delete_hash_field_then_set(
        &mut self,
        old_field: String,
        new_field: String,
        value: String,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .hdel(&key, &[old_field.as_str()])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    guard
                        .hset(&key, &new_field, &value)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    Ok::<(), anyhow::Error>(())
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    // === TTL 对话框 ===

    /// 显示 TTL 设置对话框
    fn show_ttl_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_ttl = self.key_info.as_ref().map(|i| i.ttl).unwrap_or(-1);
        let is_permanent = current_ttl == -1;

        let ttl_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.ttl_placeholder").to_string());
            if current_ttl > 0 {
                state.set_value(current_ttl.to_string(), window, cx);
            }
            state
        });
        let view = cx.entity().downgrade();

        window.open_dialog(cx, move |dialog, _window, cx| {
            let ttl_for_ok = ttl_input.clone();
            let view_for_ok = view.clone();

            dialog
                .title(t!("KeyValueView.ttl_title").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(if is_permanent {
                                    t!("KeyValueView.ttl_current_permanent").to_string()
                                } else {
                                    t!("KeyValueView.ttl_current", ttl = current_ttl).to_string()
                                }),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("KeyValueView.ttl_label").to_string()),
                                )
                                .child(Input::new(&ttl_input).w_full()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("KeyValueView.ttl_hint").to_string()),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.ok").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let ttl_str = ttl_for_ok.read(cx).text().to_string();
                    let ttl: Option<i64> = if ttl_str.is_empty() || ttl_str == "-1" {
                        None // 永久
                    } else {
                        ttl_str.parse().ok()
                    };
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.set_key_ttl(ttl, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 设置键的 TTL
    fn set_key_ttl(&mut self, ttl: Option<i64>, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    match ttl {
                        Some(seconds) if seconds > 0 => {
                            guard
                                .expire(&key, seconds)
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                        }
                        _ => {
                            guard
                                .persist(&key)
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                        }
                    }
                    Ok::<(), anyhow::Error>(())
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    // === 重命名键 ===

    /// 显示重命名对话框
    fn show_rename_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current_name) = self.current_key.clone() else {
            return;
        };
        let name_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("KeyValueView.new_key_name_placeholder").to_string());
            state.set_value(current_name.clone(), window, cx);
            state
        });
        let view = cx.entity().downgrade();

        // 在打开对话框前设置焦点，避免闪烁
        name_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = name_input.clone();
            let view_for_ok = view.clone();
            let old_name = current_name.clone();

            dialog
                .title(t!("KeyValueView.rename_key_title").to_string())
                .w(px(400.))
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .child(t!("KeyValueView.new_key_name_label").to_string()),
                        )
                        .child(Input::new(&name_input).w_full()),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.ok").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let new_name = input_for_ok.read(cx).text().to_string();
                    if new_name.is_empty() || new_name == old_name {
                        return false;
                    }
                    let _ = view_for_ok.update(cx, |v, cx| {
                        v.rename_key(new_name, cx);
                        window.close_dialog(cx);
                    });
                    false
                })
        });
    }

    /// 重命名键
    fn rename_key(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(old_name) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();
        let db_index = self.db_index;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let old_name = old_name.clone();
                let new_name = new_name.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .rename(&old_name, &new_name)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.current_key = Some(new_name.clone());
                    view.load_key(connection_id, db_index, new_name, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 保存 String 值
    fn save_string_value(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let value = self.get_editor_content(cx);
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let value = value.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .set(&key, &value, None)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.is_dirty = false;
                    cx.emit(KeyValueViewEvent::ValueUpdated { key });
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 通用删除操作的异步模板
    fn reload_after_operation(&mut self, cx: &mut Context<Self>) {
        if let (Some(conn_id), Some(key)) = (self.connection_id.clone(), self.current_key.clone()) {
            self.load_key(conn_id, self.db_index, key, cx);
        }
    }

    /// 删除 List 元素
    fn delete_list_element(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();
        let delete_marker = "__DELETED_ELEMENT_MARKER__";

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .lset(&key, index as i64, delete_marker)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    guard
                        .execute_command(&format!("LREM {} 1 {}", key, delete_marker))
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    Ok::<(), anyhow::Error>(())
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 删除 Set 元素
    fn delete_set_element(&mut self, member: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let member = member.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .srem(&key, &[member.as_str()])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 删除 ZSet 元素
    fn delete_zset_element(&mut self, member: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let member = member.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .zrem(&key, &[member.as_str()])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 删除 Hash 字段
    fn delete_hash_field(&mut self, field: String, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(key) = self.current_key.clone() else {
            return;
        };
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let field = field.clone();
                async move {
                    let conn = global_state.get_connection(&connection_id).ok_or_else(|| {
                        anyhow::anyhow!("{}", t!("KeyValueView.connection_missing"))
                    })?;
                    let guard = conn.read().await;
                    guard
                        .hdel(&key, &[field.as_str()])
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            _ = this.update(cx, |view, cx| {
                if result.is_ok() {
                    view.reload_after_operation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 渲染值编辑器
    fn render_value_editor(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let Some(content) = &self.value_content else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(t!("KeyValueView.select_key_placeholder").to_string())
                .into_any_element();
        };

        match content {
            KeyValueContent::String(_) => self.render_string_editor(cx).into_any_element(),
            KeyValueContent::List(items) => self.render_list_view(items, cx).into_any_element(),
            KeyValueContent::Set(items) => self.render_set_view(items, cx).into_any_element(),
            KeyValueContent::ZSet(items) => self.render_zset_view(items, cx).into_any_element(),
            KeyValueContent::Hash(items) => self.render_hash_view(items, cx).into_any_element(),
            KeyValueContent::Stream(entries) => {
                self.render_stream_view(entries, cx).into_any_element()
            }
            KeyValueContent::None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(t!("KeyValueView.empty_value").to_string())
                .into_any_element(),
        }
    }

    /// 渲染 String 编辑器（使用 Input 组件）
    fn render_string_editor(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.string_editor).size_full().cleanable(false)
    }

    /// 渲染底部状态栏
    fn render_status_bar(&self, cx: &App) -> impl IntoElement {
        let Some(info) = &self.key_info else {
            return div().into_any_element();
        };

        let size = info.size.unwrap_or(0);
        let content_len = match &self.value_content {
            Some(KeyValueContent::String(s)) => s.len(),
            Some(KeyValueContent::List(v)) => v.len(),
            Some(KeyValueContent::Set(v)) => v.len(),
            Some(KeyValueContent::ZSet(v)) => v.len(),
            Some(KeyValueContent::Hash(v)) => v.len(),
            Some(KeyValueContent::Stream(v)) => v.len(),
            _ => 0,
        };

        let memory_display = info
            .memory_usage
            .map(|m| t!("KeyValueView.status_memory", memory = m).to_string())
            .unwrap_or_default();

        h_flex()
            .w_full()
            .h(px(24.0))
            .px_3()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted)
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        div()
                            .text_base()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                t!("KeyValueView.status_length", count = content_len).to_string(),
                            ),
                    )
                    .when(size > 0, |this| {
                        this.child(
                            div()
                                .text_base()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("KeyValueView.status_size", size = size).to_string()),
                        )
                    })
                    .when(!memory_display.is_empty(), |this| {
                        this.child(
                            div()
                                .text_base()
                                .text_color(cx.theme().muted_foreground)
                                .child(memory_display.clone()),
                        )
                    }),
            )
            .child(
                h_flex().gap_2().child(
                    div()
                        .text_base()
                        .text_color(cx.theme().muted_foreground)
                        .child(self.view_format.display_name()),
                ),
            )
            .into_any_element()
    }

    /// 渲染 List 视图
    fn render_list_view(&self, items: &[String], cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        // 应用筛选
        let items = self.apply_filter(items);

        v_flex()
            .id("list-value-scroll")
            .size_full()
            .overflow_scroll()
            .child(self.render_table_header(
                vec![
                    (t!("KeyValueView.column_index").to_string(), 60.0),
                    (t!("KeyValueView.column_value").to_string(), 0.0),
                    (t!("KeyValueView.column_action").to_string(), 120.0),
                ],
                cx,
            ))
            .children(items.into_iter().map({
                let view = view.clone();
                move |(idx, item)| {
                    let view = view.clone();
                    let value_for_copy = item.clone();
                    let value_for_edit = item.clone();

                    h_flex()
                        .id(("list-row", idx))
                        .group("list-row")
                        .w_full()
                        .min_h(px(40.0))
                        .px_2()
                        .items_center()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .hover(|this| this.bg(cx.theme().muted))
                        .child(
                            div()
                                .w(px(60.0))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{}", idx + 1)),
                        )
                        .child(div().flex_1().text_base().truncate().child(item.clone()))
                        .child(
                            h_flex()
                                .w(px(120.0))
                                .justify_end()
                                .gap_1()
                                .opacity(0.)
                                .group_hover("list-row", |this| this.opacity(1.))
                                .child(
                                    Button::new(("copy-list", idx))
                                        .icon(IconName::Copy)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let value = value_for_copy.clone();
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    value.clone(),
                                                ));
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("edit-list", idx))
                                        .icon(IconName::Edit)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            let value = value_for_edit.clone();
                                            move |_, window, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.show_list_edit_dialog(
                                                        idx,
                                                        value.clone(),
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("delete-list", idx))
                                        .icon(IconName::Remove)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.delete_list_element(idx, cx);
                                                });
                                            }
                                        }),
                                ),
                        )
                }
            }))
    }

    /// 渲染 Set 视图
    fn render_set_view(&self, items: &[String], cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        // 应用筛选
        let items = self.apply_filter(items);

        v_flex()
            .id("set-value-scroll")
            .size_full()
            .overflow_scroll()
            .child(self.render_table_header(
                vec![
                    (t!("KeyValueView.column_member").to_string(), 0.0),
                    (t!("KeyValueView.column_action").to_string(), 120.0),
                ],
                cx,
            ))
            .children(items.into_iter().map({
                let view = view.clone();
                move |(idx, item)| {
                    let view = view.clone();
                    let value_for_copy = item.clone();
                    let value_for_delete = item.clone();

                    h_flex()
                        .id(("set-row", idx))
                        .group("set-row")
                        .w_full()
                        .min_h(px(40.0))
                        .px_2()
                        .items_center()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .hover(|this| this.bg(cx.theme().muted))
                        .child(
                            h_flex()
                                .flex_1()
                                .gap_2()
                                .items_center()
                                .child(
                                    Icon::new(IconName::Minus)
                                        .with_size(Size::Small)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(div().flex_1().text_base().truncate().child(item.clone())),
                        )
                        .child(
                            h_flex()
                                .w(px(120.0))
                                .justify_end()
                                .gap_1()
                                .opacity(0.)
                                .group_hover("set-row", |this| this.opacity(1.))
                                .child(
                                    Button::new(("copy-set", idx))
                                        .icon(IconName::Copy)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let value = value_for_copy.clone();
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    value.clone(),
                                                ));
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("delete-set", idx))
                                        .icon(IconName::Remove)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            let member = value_for_delete.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.delete_set_element(member.clone(), cx);
                                                });
                                            }
                                        }),
                                ),
                        )
                }
            }))
    }

    /// 渲染 ZSet 视图
    fn render_zset_view(&self, items: &[ZSetMember], cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        // 应用筛选和排序
        let filtered_items = self.apply_filter_zset(items);

        // 计算分数范围用于可视化
        let (min_score, max_score) = if filtered_items.is_empty() {
            (0.0, 1.0)
        } else {
            let min = filtered_items
                .iter()
                .map(|(_, m)| m.score)
                .fold(f64::INFINITY, f64::min);
            let max = filtered_items
                .iter()
                .map(|(_, m)| m.score)
                .fold(f64::NEG_INFINITY, f64::max);
            if (max - min).abs() < f64::EPSILON {
                (min - 1.0, max + 1.0)
            } else {
                (min, max)
            }
        };

        v_flex()
            .id("zset-value-scroll")
            .size_full()
            .overflow_scroll()
            .child(self.render_table_header(
                vec![
                    (t!("KeyValueView.column_rank").to_string(), 50.0),
                    (t!("KeyValueView.column_score").to_string(), 140.0),
                    (t!("KeyValueView.column_member").to_string(), 0.0),
                    (t!("KeyValueView.column_action").to_string(), 120.0),
                ],
                cx,
            ))
            .children(filtered_items.into_iter().enumerate().map({
                let view = view.clone();
                move |(display_idx, (original_idx, item))| {
                    let view = view.clone();
                    let value_for_copy = format!("{}: {}", item.score, item.member);
                    let member_for_edit = item.member.clone();
                    let score_for_edit = item.score;
                    let member_for_delete = item.member.clone();

                    // 计算分数百分比用于可视化柱状图 (0.0-1.0)
                    let score_ratio = if (max_score - min_score).abs() < f64::EPSILON {
                        0.5
                    } else {
                        ((item.score - min_score) / (max_score - min_score)).clamp(0.05, 1.0)
                    };

                    // 排名徽章
                    let rank_display = match display_idx {
                        0 => "🥇".to_string(),
                        1 => "🥈".to_string(),
                        2 => "🥉".to_string(),
                        n => format!("{}", n + 1),
                    };

                    h_flex()
                        .id(("zset-row", original_idx))
                        .group("zset-row")
                        .w_full()
                        .min_h(px(40.0))
                        .px_2()
                        .items_center()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .hover(|this| this.bg(cx.theme().muted))
                        // 排名徽章
                        .child(
                            div()
                                .w(px(50.0))
                                .text_sm()
                                .font_weight(if display_idx < 3 {
                                    gpui::FontWeight::BOLD
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .text_color(if display_idx < 3 {
                                    cx.theme().primary
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .child(rank_display),
                        )
                        // 分数可视化柱状图
                        .child(
                            h_flex()
                                .w(px(140.0))
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .h(px(16.0))
                                        .w(px(60.0))
                                        .rounded(px(2.0))
                                        .bg(cx.theme().muted)
                                        .child(
                                            div()
                                                .h_full()
                                                .w(relative(score_ratio as f32))
                                                .rounded(px(2.0))
                                                .bg(cx.theme().primary),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().primary)
                                        .child(format!("{:.2}", item.score)),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_base()
                                .truncate()
                                .child(item.member.clone()),
                        )
                        .child(
                            h_flex()
                                .w(px(120.0))
                                .justify_end()
                                .gap_1()
                                .opacity(0.)
                                .group_hover("zset-row", |this| this.opacity(1.))
                                .child(
                                    Button::new(("copy-zset", original_idx))
                                        .icon(IconName::Copy)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let value = value_for_copy.clone();
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    value.clone(),
                                                ));
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("edit-zset", original_idx))
                                        .icon(IconName::Edit)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            let member = member_for_edit.clone();
                                            let score = score_for_edit;
                                            move |_, window, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.show_zset_edit_dialog(
                                                        member.clone(),
                                                        score,
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("delete-zset", original_idx))
                                        .icon(IconName::Remove)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            let member = member_for_delete.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.delete_zset_element(member.clone(), cx);
                                                });
                                            }
                                        }),
                                ),
                        )
                }
            }))
    }

    /// 渲染 Hash 视图
    fn render_hash_view(&self, items: &[HashField], cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        // 应用筛选
        let items = self.apply_filter_hash(items);

        v_flex()
            .id("hash-value-scroll")
            .size_full()
            .overflow_scroll()
            .child(self.render_table_header(
                vec![
                    (t!("KeyValueView.column_field").to_string(), 150.0),
                    (t!("KeyValueView.column_value").to_string(), 0.0),
                    (t!("KeyValueView.column_action").to_string(), 120.0),
                ],
                cx,
            ))
            .children(items.into_iter().map({
                let view = view.clone();
                move |(idx, item)| {
                    let view = view.clone();
                    let field_for_copy = format!("{}: {}", item.field, item.value);
                    let field_for_edit = item.field.clone();
                    let value_for_edit = item.value.clone();
                    let field_for_delete = item.field.clone();

                    h_flex()
                        .id(("hash-row", idx))
                        .group("hash-row")
                        .w_full()
                        .min_h(px(40.0))
                        .px_2()
                        .items_center()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .hover(|this| this.bg(cx.theme().muted))
                        .child(
                            div()
                                .w(px(150.0))
                                .text_base()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .truncate()
                                .child(item.field.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_base()
                                .truncate()
                                .child(item.value.clone()),
                        )
                        .child(
                            h_flex()
                                .w(px(120.0))
                                .justify_end()
                                .gap_1()
                                .opacity(0.)
                                .group_hover("hash-row", |this| this.opacity(1.))
                                .child(
                                    Button::new(("copy-hash", idx))
                                        .icon(IconName::Copy)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let value = field_for_copy.clone();
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    value.clone(),
                                                ));
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("edit-hash", idx))
                                        .icon(IconName::Edit)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            let field = field_for_edit.clone();
                                            let value = value_for_edit.clone();
                                            move |_, window, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.show_hash_edit_dialog(
                                                        field.clone(),
                                                        value.clone(),
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new(("delete-hash", idx))
                                        .icon(IconName::Remove)
                                        .ghost()
                                        .with_size(Size::Medium)
                                        .on_click({
                                            let view = view.clone();
                                            let field = field_for_delete.clone();
                                            move |_, _, cx| {
                                                view.update(cx, |v, cx| {
                                                    v.delete_hash_field(field.clone(), cx);
                                                });
                                            }
                                        }),
                                ),
                        )
                }
            }))
    }

    /// 渲染表格头部
    fn render_table_header(
        &self,
        columns: Vec<(String, f32)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut header = h_flex()
            .w_full()
            .h(px(36.0))
            .px_2()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted);

        let last_index = columns.len().saturating_sub(1);
        for (index, (name, width)) in columns.into_iter().enumerate() {
            let col = div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD);

            if width > 0.0 {
                let col = col.w(px(width));
                if index == last_index {
                    header = header.child(col.text_right().child(name));
                } else {
                    header = header.child(col.child(name));
                }
            } else {
                header = header.child(col.flex_1().child(name));
            }
        }

        header
    }

    /// 渲染 Stream 视图
    fn render_stream_view(&self, entries: &[crate::StreamEntry], cx: &App) -> impl IntoElement {
        v_flex()
            .id("stream-value-scroll")
            .size_full()
            .p_2()
            .gap_2()
            .overflow_scroll()
            .children(entries.iter().map(|entry| {
                v_flex()
                    .w_full()
                    .p_2()
                    .rounded(px(4.0))
                    .bg(cx.theme().muted)
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(cx.theme().accent)
                            .child(format!("ID: {}", entry.id)),
                    )
                    .children(entry.fields.iter().map(|(k, v)| {
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .w(px(100.0))
                                    .child(k.clone()),
                            )
                            .child(div().text_sm().child(v.clone()))
                    }))
            }))
    }

    /// 渲染空状态
    fn render_empty_state(&self, cx: &App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                Icon::new(IconName::Database.color())
                    .with_size(Size::Large)
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("KeyValueView.select_key_placeholder").to_string()),
            )
    }

    /// 渲染加载状态
    fn render_loading_state(&self, _cx: &App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(Spinner::new())
    }

    /// 渲染错误状态
    fn render_error_state(&self, error: &str, cx: &App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                Icon::new(IconName::TriangleAlert)
                    .with_size(Size::Large)
                    .text_color(cx.theme().danger),
            )
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().danger)
                    .child(t!("KeyValueView.load_failed", error = error).to_string()),
            )
    }
}

impl EventEmitter<KeyValueViewEvent> for KeyValueView {}

impl Focusable for KeyValueView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KeyValueView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 应用异步加载后的待设置值
        self.apply_pending_editor_value(window, cx);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .when(matches!(self.load_state, LoadState::Empty), |this| {
                this.child(self.render_empty_state(cx))
            })
            .when(matches!(self.load_state, LoadState::Loading), |this| {
                this.child(self.render_loading_state(cx))
            })
            .when(matches!(self.load_state, LoadState::Error(_)), |this| {
                if let LoadState::Error(ref e) = self.load_state {
                    this.child(self.render_error_state(e, cx))
                } else {
                    this
                }
            })
            .when(matches!(self.load_state, LoadState::Loaded), |this| {
                this.child(self.render_key_info(cx))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(self.render_value_editor(window, cx)),
                    )
                    .child(self.render_status_bar(cx))
            })
    }
}

impl EventEmitter<TabContentEvent> for KeyValueView {}

impl TabContent for KeyValueView {
    fn content_key(&self) -> &'static str {
        "KeyValue"
    }

    fn title(&self, _cx: &App) -> SharedString {
        self.current_key
            .clone()
            .unwrap_or_else(|| t!("KeyValueView.tab_title_default").to_string())
            .into()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Key).with_size(Size::Medium))
    }

    fn closeable(&self, _cx: &App) -> bool {
        self.closeable
    }

    fn try_close(
        &mut self,
        _tab_id: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Task<bool> {
        Task::ready(true)
    }
}

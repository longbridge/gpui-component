//! Redis CLI 终端视图
//!
//! 提供类似于 redis-cli 的交互式命令行界面
//! 参考 terminal_view 的实现，实现完整的交互式终端体验

use gpui::{
    App, AsyncApp, Bounds, ClipboardItem, Context, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, FontFallbacks, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, Render, ScrollWheelEvent, SharedString, Styled, Task,
    UTF16Selection, Window, actions, canvas, div, px, rgb, size,
};
use gpui_component::{
    Icon, IconName, Sizable, Size,
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
};
use one_core::gpui_tokio::Tokio;
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;
use crate::{GlobalRedisState, RedisValue};
use crate::redis_cli_element::{
    cell_column_for_char_index, cell_len, char_index_for_cell_column, CliLine, CliLineType,
    CliTheme, RedisCliElement, SelectionType, TextPosition, TextSelection,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

actions!(
    redis_cli,
    [
        ExecuteCommand,
        PreviousCommand,
        NextCommand,
        ClearOutput,
        Copy,
        Paste,
        SelectAll,
        ClearSelection,
        MoveLeft,
        MoveRight,
        MoveToStart,
        MoveToEnd,
        DeleteBackward,
        DeleteForward,
        CompleteCommand,
        // Shift+方向键选择
        SelectLeft,
        SelectRight,
        SelectToStart,
        SelectToEnd,
    ]
);

const REDIS_CLI_CONTEXT: &str = "RedisCli";
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
/// 双击判定时间（毫秒）
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 500;

struct CommandHint {
    name: &'static str,
    usage: &'static str,
}

const COMMAND_HINTS: &[CommandHint] = &[
    CommandHint { name: "PING", usage: "PING" },
    CommandHint { name: "INFO", usage: "INFO [section]" },
    CommandHint { name: "DBSIZE", usage: "DBSIZE" },
    CommandHint { name: "SELECT", usage: "SELECT <index>" },
    CommandHint { name: "KEYS", usage: "KEYS <pattern>" },
    CommandHint { name: "SCAN", usage: "SCAN <cursor> [MATCH pattern] [COUNT count]" },
    CommandHint { name: "GET", usage: "GET <key>" },
    CommandHint { name: "SET", usage: "SET <key> <value>" },
    CommandHint { name: "DEL", usage: "DEL <key> [key ...]" },
    CommandHint { name: "EXISTS", usage: "EXISTS <key> [key ...]" },
    CommandHint { name: "TTL", usage: "TTL <key>" },
    CommandHint { name: "EXPIRE", usage: "EXPIRE <key> <seconds>" },
    CommandHint { name: "PERSIST", usage: "PERSIST <key>" },
    CommandHint { name: "HGET", usage: "HGET <key> <field>" },
    CommandHint { name: "HSET", usage: "HSET <key> <field> <value>" },
    CommandHint { name: "HDEL", usage: "HDEL <key> <field> [field ...]" },
    CommandHint { name: "LRANGE", usage: "LRANGE <key> <start> <stop>" },
    CommandHint { name: "LPUSH", usage: "LPUSH <key> <value> [value ...]" },
    CommandHint { name: "RPUSH", usage: "RPUSH <key> <value> [value ...]" },
    CommandHint { name: "SADD", usage: "SADD <key> <member> [member ...]" },
    CommandHint { name: "SMEMBERS", usage: "SMEMBERS <key>" },
    CommandHint { name: "ZADD", usage: "ZADD <key> <score> <member> [score member ...]" },
    CommandHint { name: "ZRANGE", usage: "ZRANGE <key> <start> <stop> [WITHSCORES]" },
    CommandHint { name: "ZREM", usage: "ZREM <key> <member> [member ...]" },
    CommandHint { name: "XADD", usage: "XADD <key> <id> <field> <value> [field value ...]" },
    CommandHint { name: "XLEN", usage: "XLEN <key>" },
    CommandHint { name: "XRANGE", usage: "XRANGE <key> <start> <end> [COUNT count]" },
    CommandHint { name: "HELP", usage: "HELP" },
];

/// 鼠标状态
#[derive(Default)]
struct MouseState {
    /// 是否正在拖拽选择
    selecting: bool,
    /// 上次点击位置
    last_click_position: Option<TextPosition>,
    /// 连续点击次数
    click_count: u32,
    /// 上次点击时间
    last_click_time: Option<Instant>,
}

/// 注册键绑定
pub fn init(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("ctrl-l", ClearOutput, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("cmd-c", Copy, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("cmd-v", Paste, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("cmd-a", SelectAll, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("escape", ClearSelection, Some(REDIS_CLI_CONTEXT)),
        // 光标移动
        gpui::KeyBinding::new("left", MoveLeft, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("right", MoveRight, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("ctrl-a", MoveToStart, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("ctrl-e", MoveToEnd, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("cmd-left", MoveToStart, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("cmd-right", MoveToEnd, Some(REDIS_CLI_CONTEXT)),
        // Shift+方向键选择
        gpui::KeyBinding::new("shift-left", SelectLeft, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("shift-right", SelectRight, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("shift-cmd-left", SelectToStart, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("shift-cmd-right", SelectToEnd, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("shift-ctrl-a", SelectToStart, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("shift-ctrl-e", SelectToEnd, Some(REDIS_CLI_CONTEXT)),
        gpui::KeyBinding::new("tab", CompleteCommand, Some(REDIS_CLI_CONTEXT)),
    ]);
}

/// 命令执行结果条目
#[derive(Clone, Debug)]
struct CliEntry {
    /// 执行的命令
    command: String,
    /// 命令结果
    result: CliResult,
}

/// CLI 结果类型
#[derive(Clone, Debug)]
enum CliResult {
    /// 成功的结果
    Success(RedisValue),
    /// 错误信息
    Error(String),
}

/// Redis CLI 视图事件
#[derive(Clone, Debug)]
pub enum RedisCliViewEvent {
    /// 命令已执行
    CommandExecuted { command: String },
}

/// 光标闪烁管理器
struct BlinkManager {
    blink_epoch: usize,
    blinking_paused: bool,
    visible: bool,
    enabled: bool,
}

impl BlinkManager {
    fn new() -> Self {
        Self {
            blink_epoch: 0,
            blinking_paused: false,
            visible: true,
            enabled: false,
        }
    }

    fn enable(&mut self) {
        self.enabled = true;
        self.visible = true;
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.visible = true;
    }

    fn pause_blinking(&mut self) {
        self.visible = true;
        self.blinking_paused = true;
    }

    fn resume_blinking(&mut self, epoch: usize) {
        if epoch == self.blink_epoch {
            self.blinking_paused = false;
        }
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn next_epoch(&mut self) -> usize {
        self.blink_epoch += 1;
        self.blink_epoch
    }

    fn blink(&mut self, epoch: usize) -> bool {
        if epoch == self.blink_epoch && self.enabled && !self.blinking_paused {
            self.visible = !self.visible;
            true
        } else {
            false
        }
    }
}

/// Redis CLI 视图
pub struct RedisCliView {
    /// 当前连接 ID
    connection_id: String,
    /// 当前数据库索引
    db_index: u8,
    /// 当前输入文本
    input_text: String,
    /// 光标位置
    cursor_pos: usize,
    /// 命令历史记录
    command_history: Vec<String>,
    /// 当前历史索引（用于上下键导航）
    history_index: Option<usize>,
    /// 临时保存的输入（在浏览历史时）
    temp_input: String,
    /// 输出条目列表
    output_entries: Vec<CliEntry>,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 滚动偏移（行数）
    scroll_offset: f32,
    /// 是否正在执行命令
    is_executing: bool,
    /// 最大历史记录数
    max_history: usize,
    /// 最大输出条目数
    max_output_entries: usize,
    /// 光标闪烁管理器
    blink_manager: BlinkManager,
    /// 终端边界
    terminal_bounds: Bounds<Pixels>,
    /// 主题
    theme: CliTheme,
    /// 文本选择状态
    selection: Option<TextSelection>,
    /// 鼠标状态
    mouse_state: MouseState,
    /// 精确计算的字符宽度
    cell_width: Pixels,
    /// IME 标记文本范围（UTF16）
    ime_marked_range: Option<std::ops::Range<usize>>,
}

impl RedisCliView {
    pub fn new(
        connection_id: String,
        db_index: u8,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        // 启动闪烁定时器
        let mut this = Self {
            connection_id,
            db_index,
            input_text: String::new(),
            cursor_pos: 0,
            command_history: Vec::new(),
            history_index: None,
            temp_input: String::new(),
            output_entries: Vec::new(),
            focus_handle: focus_handle.clone(),
            scroll_offset: 0.0,
            is_executing: false,
            max_history: 1000,
            max_output_entries: 500,
            blink_manager: BlinkManager::new(),
            terminal_bounds: Bounds::default(),
            theme: CliTheme::default(),
            selection: None,
            mouse_state: MouseState::default(),
            cell_width: px(7.8), // 默认值，会在渲染时更新
            ime_marked_range: None,
        };

        this.start_blink_timer(cx);
        this
    }

    /// 更新字符宽度（精确计算）
    /// 复用 terminal_view 的方法：使用 text_system().advance() 获取精确的字符宽度
    fn update_cell_width(&mut self, window: &mut Window) {
        let features = gpui::FontFeatures(Arc::new(vec![("calt".to_string(), 0)]));
        let fallbacks = if self.theme.font_fallbacks.is_empty() {
            None
        } else {
            Some(FontFallbacks::from_fonts(
                self.theme
                    .font_fallbacks
                    .iter()
                    .map(|fallback| fallback.to_string())
                    .collect::<Vec<_>>(),
            ))
        };
        let font = gpui::Font {
            family: self.theme.font_family.clone(),
            features,
            fallbacks,
            weight: gpui::FontWeight::NORMAL,
            style: gpui::FontStyle::Normal,
        };
        let font_id = window.text_system().resolve_font(&font);
        // 使用 advance('m').width 计算字符宽度
        // 对于等宽字体，这比 em_width 更准确
        let new_cell_width = window
            .text_system()
            .advance(font_id, self.theme.font_size, 'm')
            .map(|advance| advance.width)
            .unwrap_or(self.theme.font_size * 0.6);

        if self.cell_width != new_cell_width {
            self.cell_width = new_cell_width;
        }
    }

    /// 启动光标闪烁定时器
    fn start_blink_timer(&mut self, cx: &mut Context<Self>) {
        let epoch = self.blink_manager.next_epoch();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CURSOR_BLINK_INTERVAL).await;
                let should_continue = this.update(cx, |view, cx| {
                    if view.blink_manager.blink(epoch) {
                        cx.notify();
                        true
                    } else {
                        false
                    }
                });
                match should_continue {
                    Ok(true) => continue,
                    _ => break,
                }
            }
        })
        .detach();
    }

    /// 获取提示符
    fn get_prompt(&self) -> String {
        format!("{}:db{}>", "redis", self.db_index)
    }

    /// 处理键盘按下事件
    /// 只处理特殊键，普通文本输入由 EntityInputHandler 处理
    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 暂停闪烁
        self.blink_manager.pause_blinking();
        let epoch = self.blink_manager.next_epoch();

        // 延迟恢复闪烁
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(500)).await;
            let _ = this.update(cx, |view, cx| {
                view.blink_manager.resume_blinking(epoch);
                view.start_blink_timer(cx);
            });
        })
        .detach();

        // 只处理特殊键，普通字符输入由 EntityInputHandler::replace_text_in_range 处理
        match event.keystroke.key.as_str() {
            "enter" => {
                self.execute_current_command(window, cx);
            }
            "up" => {
                self.previous_command(cx);
            }
            "down" => {
                self.next_command(cx);
            }
            "backspace" => {
                self.delete_backward(cx);
            }
            "delete" => {
                self.delete_forward(cx);
            }
            _ => {
                // 普通字符输入由 EntityInputHandler 处理，这里不做任何处理
            }
        }
    }

    /// 插入文本
    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        // 如果有选择，先删除选中内容
        if self.selection.is_some() {
            self.delete_selection(cx);
        }

        // 在光标位置插入文本
        let byte_pos = self.input_text
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.input_text.len());

        self.input_text.insert_str(byte_pos, text);
        self.cursor_pos += text.chars().count();
        self.history_index = None;
        self.ime_marked_range = None;
        cx.notify();
    }

    /// 删除光标前一个字符
    fn delete_backward(&mut self, cx: &mut Context<Self>) {
        // 如果有选择，删除选中内容
        if self.selection.is_some() {
            self.delete_selection(cx);
            return;
        }

        if self.cursor_pos > 0 {
            let char_indices: Vec<_> = self.input_text.char_indices().collect();
            if self.cursor_pos <= char_indices.len() {
                // 删除前一个字符
                let start = char_indices.get(self.cursor_pos - 1).map(|(i, _)| *i).unwrap_or(0);
                let end = char_indices.get(self.cursor_pos).map(|(i, _)| *i).unwrap_or(self.input_text.len());

                self.input_text = format!(
                    "{}{}",
                    &self.input_text[..start],
                    &self.input_text[end.min(self.input_text.len())..]
                );
                self.cursor_pos -= 1;
                cx.notify();
            }
        }
    }

    /// 删除光标后一个字符
    fn delete_forward(&mut self, cx: &mut Context<Self>) {
        // 如果有选择，删除选中内容
        if self.selection.is_some() {
            self.delete_selection(cx);
            return;
        }

        let char_count = self.input_text.chars().count();
        if self.cursor_pos < char_count {
            let char_indices: Vec<_> = self.input_text.char_indices().collect();
            let start = char_indices.get(self.cursor_pos).map(|(i, _)| *i).unwrap_or(self.input_text.len());
            let end = char_indices.get(self.cursor_pos + 1).map(|(i, _)| *i).unwrap_or(self.input_text.len());

            self.input_text = format!(
                "{}{}",
                &self.input_text[..start],
                &self.input_text[end..]
            );
            cx.notify();
        }
    }

    /// 光标左移
    fn move_left(&mut self, _: &MoveLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection = None;
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            cx.notify();
        }
    }

    /// 光标右移
    fn move_right(&mut self, _: &MoveRight, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection = None;
        let char_count = self.input_text.chars().count();
        if self.cursor_pos < char_count {
            self.cursor_pos += 1;
            cx.notify();
        }
    }

    /// 光标移动到开头
    fn move_to_start(&mut self, _: &MoveToStart, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection = None;
        self.cursor_pos = 0;
        cx.notify();
    }

    /// 光标移动到结尾
    fn move_to_end(&mut self, _: &MoveToEnd, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection = None;
        self.cursor_pos = self.input_text.chars().count();
        cx.notify();
    }

    /// 清除选择
    fn clear_selection(&mut self, _: &ClearSelection, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_some() {
            self.selection = None;
            cx.notify();
        }
    }

    /// Shift+左箭头选择
    fn select_left(&mut self, _: &SelectLeft, _window: &mut Window, cx: &mut Context<Self>) {
        if self.cursor_pos > 0 {
            let lines = self.build_text_lines();
            let input_line = self.input_line_index(&lines);
            let input_start_col = self.get_prompt().chars().count() + 1;

            let anchor = if let Some(sel) = &self.selection {
                sel.anchor
            } else {
                let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
                TextPosition::new(input_line, input_start_col + cursor_col)
            };

            self.cursor_pos -= 1;
            let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
            let active = TextPosition::new(input_line, input_start_col + cursor_col);
            self.selection = Some(TextSelection::new(anchor, active, SelectionType::Simple));
            cx.notify();
        }
    }

    /// Shift+右箭头选择
    fn select_right(&mut self, _: &SelectRight, _window: &mut Window, cx: &mut Context<Self>) {
        let char_count = self.input_text.chars().count();
        if self.cursor_pos < char_count {
            let lines = self.build_text_lines();
            let input_line = self.input_line_index(&lines);
            let input_start_col = self.get_prompt().chars().count() + 1;

            let anchor = if let Some(sel) = &self.selection {
                sel.anchor
            } else {
                let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
                TextPosition::new(input_line, input_start_col + cursor_col)
            };

            self.cursor_pos += 1;
            let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
            let active = TextPosition::new(input_line, input_start_col + cursor_col);
            self.selection = Some(TextSelection::new(anchor, active, SelectionType::Simple));
            cx.notify();
        }
    }

    /// Shift+Cmd+左箭头选择到开头
    fn select_to_start(&mut self, _: &SelectToStart, _window: &mut Window, cx: &mut Context<Self>) {
        let lines = self.build_text_lines();
        let input_line = self.input_line_index(&lines);
        let input_start_col = self.get_prompt().chars().count() + 1;

        let anchor = if let Some(sel) = &self.selection {
            sel.anchor
        } else {
            let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
            TextPosition::new(input_line, input_start_col + cursor_col)
        };

        self.cursor_pos = 0;
        let active = TextPosition::new(input_line, input_start_col);
        self.selection = Some(TextSelection::new(anchor, active, SelectionType::Simple));
        cx.notify();
    }

    /// Shift+Cmd+右箭头选择到结尾
    fn select_to_end(&mut self, _: &SelectToEnd, _window: &mut Window, cx: &mut Context<Self>) {
        let char_count = self.input_text.chars().count();
        let lines = self.build_text_lines();
        let input_line = self.input_line_index(&lines);
        let input_start_col = self.get_prompt().chars().count() + 1;

        let anchor = if let Some(sel) = &self.selection {
            sel.anchor
        } else {
            let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
            TextPosition::new(input_line, input_start_col + cursor_col)
        };

        self.cursor_pos = char_count;
        let input_cell_len = cell_len(&self.input_text);
        let active = TextPosition::new(input_line, input_start_col + input_cell_len);
        self.selection = Some(TextSelection::new(anchor, active, SelectionType::Simple));
        cx.notify();
    }

    fn input_line_index(&self, lines: &[String]) -> usize {
        lines.len().saturating_sub(1)
    }

    /// 构建文本行列表（用于选择计算和渲染）
    fn build_text_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // 欢迎信息
        lines.push(t!("RedisCli.welcome").to_string());
        lines.push(String::new());

        // 历史命令和结果
        for entry in &self.output_entries {
            // 命令行
            lines.push(format!("{} {}", self.get_prompt(), entry.command));

            // 结果行
            let text = match &entry.result {
                CliResult::Success(value) => self.format_redis_value(value, 0),
                CliResult::Error(e) => format!("(error) {}", e),
            };

            for line_text in text.lines() {
                lines.push(line_text.to_string());
            }
        }

        // 当前输入行
        lines.push(format!("{} {}", self.get_prompt(), self.input_text));

        lines
    }

    /// 将像素位置转换为文本位置
    fn pixel_to_text_position(&self, position: Point<Pixels>) -> TextPosition {
        let bounds = self.terminal_bounds;
        let line_height = self.theme.font_size * self.theme.line_height_scale;
        let char_width = self.cell_width;
        let padding_x = px(12.0);

        // 计算相对于文本内容区域的位置
        let relative_x = position.x - bounds.origin.x - padding_x;
        let relative_y = position.y - bounds.origin.y;

        // 考虑滚动偏移：scroll_offset 是向上滚动的行数
        // 向下滚动时 scroll_offset 为正，文本向上移动
        let scrolled_y = relative_y + (line_height * self.scroll_offset);

        // 计算行号
        let line = (scrolled_y / line_height).max(0.0) as usize;

        // 计算列号
        let column = (relative_x / char_width).max(0.0) as usize;

        TextPosition::new(line, column)
    }

    /// 处理鼠标按下事件
    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);

        if event.button != MouseButton::Left {
            return;
        }

        let raw_position = self.pixel_to_text_position(event.position);
        let now = Instant::now();

        let lines = self.build_text_lines();
        let position = clamp_position_to_lines(raw_position, &lines);

        // 检测双击/三击
        let is_multi_click = self.mouse_state.last_click_position == Some(position)
            && self.mouse_state.last_click_time
                .map_or(false, |t| now.duration_since(t).as_millis() < DOUBLE_CLICK_THRESHOLD_MS);

        if is_multi_click {
            self.mouse_state.click_count += 1;
        } else {
            self.mouse_state.click_count = 1;
        }

        self.mouse_state.last_click_position = Some(position);
        self.mouse_state.last_click_time = Some(now);

        match self.mouse_state.click_count {
            1 => {
                // 单击：开始选择
                self.selection = Some(TextSelection::point(position));
                self.mouse_state.selecting = true;
            }
            2 => {
                // 双击：选择单词
                self.select_word_at(position, &lines);
            }
            _ => {
                // 三击及以上：选择整行
                self.select_line_at(position, &lines);
            }
        }

        // 如果点击在输入行，更新光标位置
        let input_line = self.input_line_index(&lines);
        if position.line == input_line {
            let prompt_len = self.get_prompt().chars().count() + 1;
            let col = position.column.saturating_sub(prompt_len);
            self.cursor_pos = char_index_for_cell_column(&self.input_text, col);
        }

        cx.notify();
    }

    /// 处理鼠标移动事件
    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.mouse_state.selecting {
            return;
        }

        let raw_position = self.pixel_to_text_position(event.position);
        let lines = self.build_text_lines();
        let position = clamp_position_to_lines(raw_position, &lines);

        if let Some(ref mut selection) = self.selection {
            selection.active = position;
        }

        // 更新光标位置（如果在输入行）
        let input_line = self.input_line_index(&lines);
        if position.line == input_line {
            let prompt_len = self.get_prompt().chars().count() + 1;
            let col = position.column.saturating_sub(prompt_len);
            self.cursor_pos = char_index_for_cell_column(&self.input_text, col);
        }

        cx.notify();
    }

    /// 处理鼠标释放事件
    fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }

        self.mouse_state.selecting = false;

        // 如果选择为空，清除选择状态
        if let Some(ref selection) = self.selection {
            if selection.is_empty() {
                self.selection = None;
            }
        }

        cx.notify();
    }

    /// 选择指定位置的单词
    fn select_word_at(&mut self, position: TextPosition, lines: &[String]) {
        if position.line >= lines.len() {
            return;
        }

        let line = &lines[position.line];
        let char_index = char_index_for_cell_column(line, position.column);
        let chars: Vec<char> = line.chars().collect();

        if char_index >= chars.len() {
            return;
        }

        // 找到单词边界
        let mut start = char_index;
        let mut end = char_index;

        // 向左查找单词开始
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }

        // 向右查找单词结束
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }

        if start < end {
            let start_col = cell_column_for_char_index(line, start);
            let end_col = cell_column_for_char_index(line, end);
            self.selection = Some(TextSelection::new(
                TextPosition::new(position.line, start_col),
                TextPosition::new(position.line, end_col),
                SelectionType::Word,
            ));
        }
    }

    /// 选择指定行
    fn select_line_at(&mut self, position: TextPosition, lines: &[String]) {
        if position.line >= lines.len() {
            return;
        }

        let line_len = cell_len(&lines[position.line]);
        self.selection = Some(TextSelection::new(
            TextPosition::new(position.line, 0),
            TextPosition::new(position.line, line_len),
            SelectionType::Line,
        ));
    }

    /// 获取选中的文本
    fn get_selected_text(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        if selection.is_empty() {
            return None;
        }

        let lines = self.build_text_lines();
        let (start, end) = selection.normalized();

        let mut result = String::new();

        for (line_idx, line) in lines.iter().enumerate() {
            if line_idx < start.line || line_idx > end.line {
                continue;
            }

            let chars: Vec<char> = line.chars().collect();
            let start_col = if line_idx == start.line { start.column } else { 0 };
            let end_col = if line_idx == end.line { end.column } else { cell_len(line) };

            let start_char = char_index_for_cell_column(line, start_col).min(chars.len());
            let end_char = char_index_for_cell_column(line, end_col).min(chars.len());

            if start_char < end_char {
                result.push_str(&chars[start_char..end_char].iter().collect::<String>());
            }

            // 如果不是最后一行，添加换行符
            if line_idx < end.line {
                result.push('\n');
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// 执行当前输入的命令
    fn execute_current_command(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let command = self.input_text.trim().to_string();
        if self.is_executing {
            return;
        }

        if command.is_empty() {
            self.add_output_entry(
                CliEntry {
                    command: String::new(),
                    result: CliResult::Success(RedisValue::Status(String::new())),
                },
                cx,
            );
            self.input_text.clear();
            self.cursor_pos = 0;
            self.selection = None;
            self.history_index = None;
            self.temp_input.clear();
            return;
        }

        // 检查是否是本地命令
        if self.handle_local_command(&command, cx) {
            self.input_text.clear();
            self.cursor_pos = 0;
            return;
        }

        // 添加到历史记录
        self.add_to_history(command.clone());

        // 清空输入
        self.input_text.clear();
        self.cursor_pos = 0;

        // 执行远程命令
        self.execute_remote_command(command, cx);
    }

    /// 处理本地命令（如 CLEAR, HELP 等）
    fn handle_local_command(&mut self, command: &str, cx: &mut Context<Self>) -> bool {
        let upper = command.to_uppercase();

        match upper.as_str() {
            "CLEAR" | "CLS" => {
                self.clear_output(cx);
                true
            }
            "HELP" => {
                self.show_help(cx);
                true
            }
            _ => false,
        }
    }

    /// 显示帮助信息
    fn show_help(&mut self, cx: &mut Context<Self>) {
        // 先添加命令行
        self.output_entries.push(CliEntry {
            command: "HELP".to_string(),
            result: CliResult::Success(RedisValue::String(r#"Redis CLI 帮助
================

本地命令:
  CLEAR, CLS    清空输出
  HELP          显示此帮助信息

键盘快捷键:
  Enter         执行命令
  ↑ / ↓         浏览命令历史
  Ctrl+L        清空输出
  Cmd+C         复制
  Cmd+V         粘贴

常用 Redis 命令:
  PING          测试连接
  INFO          获取服务器信息
  KEYS *        列出所有键
  GET <key>     获取字符串值
  SET <key> <value>  设置字符串值
  DEL <key>     删除键"#.to_string())),
        });
        cx.notify();
    }

    /// 执行远程 Redis 命令
    fn execute_remote_command(&mut self, command: String, cx: &mut Context<Self>) {
        self.is_executing = true;
        cx.notify();

        let connection_id = self.connection_id.clone();
        let global_state = cx.global::<GlobalRedisState>().clone();
        let _start_time = Instant::now();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let spawn_result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let command = command.clone();
                async move {
                    let conn = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!(t!("RedisCli.connection_missing")))?;
                    let guard = conn.read().await;
                    guard.execute_command(&command).await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            }).await;

            let result = match spawn_result {
                Ok(value) =>  {
                    CliResult::Success(value)
                },
                Err(e) => CliResult::Error(e.to_string()),
            };

            _ = this.update(cx, |view, cx| {
                let entry = CliEntry {
                    command: command.clone(),
                    result,
                };
                view.add_output_entry(entry, cx);
                view.is_executing = false;
                cx.emit(RedisCliViewEvent::CommandExecuted { command });
                cx.notify();
            });
        })
        .detach();
    }

    /// 添加输出条目
    fn add_output_entry(&mut self, entry: CliEntry, cx: &mut Context<Self>) {
        self.output_entries.push(entry);

        // 限制输出条目数量
        while self.output_entries.len() > self.max_output_entries {
            self.output_entries.remove(0);
        }

        cx.notify();
    }

    /// 添加到历史记录
    fn add_to_history(&mut self, command: String) {
        // 避免连续重复的命令
        if self.command_history.last() != Some(&command) {
            self.command_history.push(command);
        }

        // 限制历史记录数量
        while self.command_history.len() > self.max_history {
            self.command_history.remove(0);
        }

        // 重置历史索引
        self.history_index = None;
        self.temp_input.clear();
    }

    /// 上一条命令
    fn previous_command(&mut self, cx: &mut Context<Self>) {
        if self.command_history.is_empty() {
            return;
        }

        // 保存当前输入
        if self.history_index.is_none() {
            self.temp_input = self.input_text.clone();
        }

        let new_index = match self.history_index {
            None => self.command_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };

        self.history_index = Some(new_index);
        self.input_text = self.command_history[new_index].clone();
        self.cursor_pos = self.input_text.chars().count();
        cx.notify();
    }

    /// 下一条命令
    fn next_command(&mut self, cx: &mut Context<Self>) {
        let Some(current_index) = self.history_index else {
            return;
        };

        if current_index >= self.command_history.len() - 1 {
            // 恢复临时输入
            self.history_index = None;
            self.input_text = self.temp_input.clone();
            self.cursor_pos = self.input_text.chars().count();
        } else {
            let new_index = current_index + 1;
            self.history_index = Some(new_index);
            self.input_text = self.command_history[new_index].clone();
            self.cursor_pos = self.input_text.chars().count();
        }
        cx.notify();
    }

    /// 清空输出
    fn clear_output(&mut self, cx: &mut Context<Self>) {
        self.output_entries.clear();
        self.scroll_offset = 0.0;
        cx.notify();
    }

    /// 处理滚轮事件
    fn handle_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 计算总行数
        let total_lines = self.build_render_lines().len() as f32;
        let line_height = self.theme.font_size * self.theme.line_height_scale;

        // 计算可见行数（使用终端边界高度）
        let visible_lines = if self.terminal_bounds.size.height > px(0.0) {
            (self.terminal_bounds.size.height / line_height) as f32
        } else {
            20.0 // 默认值
        };

        // 计算最大滚动偏移
        let max_scroll = (total_lines - visible_lines).max(0.0);

        // 根据滚轮方向调整偏移
        let delta_pixels = event.delta.pixel_delta(line_height);
        let delta = (delta_pixels.y / line_height) as f32;
        self.scroll_offset = (self.scroll_offset - delta).clamp(0.0, max_scroll);

        cx.notify();
    }

    /// 清空输出（action handler）
    fn clear_output_action(&mut self, _: &ClearOutput, _window: &mut Window, cx: &mut Context<Self>) {
        self.clear_output(cx);
    }

    /// 复制
    fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        // 优先复制选中的文本
        if let Some(text) = self.get_selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            return;
        }

        // 复制当前输入文本
        if !self.input_text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.input_text.clone()));
        } else if let Some(entry) = self.output_entries.last() {
            // 复制最后一条命令的结果
            let text = match &entry.result {
                CliResult::Success(value) => self.format_redis_value(value, 0),
                CliResult::Error(e) => format!("(error) {}", e),
            };
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    /// 粘贴
    fn paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        // 如果有选择，先删除选中的文本
        if self.selection.is_some() {
            self.delete_selection(cx);
        }

        if let Some(clipboard) = cx.read_from_clipboard() {
            if let Some(text) = clipboard.text() {
                // 只取第一行
                let first_line = text.lines().next().unwrap_or("");
                self.insert_text(first_line, cx);
            }
        }
    }

    /// 删除选中的文本（仅输入行）
    fn delete_selection(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = self.selection.take() else {
            return;
        };

        let lines = self.build_text_lines();
        let input_line = self.input_line_index(&lines);
        let prompt_len = self.get_prompt().chars().count() + 1;

        let (start, end) = selection.normalized();

        // 只删除输入行的选中部分
        if start.line == input_line && end.line == input_line {
        let start_col = start.column.saturating_sub(prompt_len);
        let end_col = end.column.saturating_sub(prompt_len);
        let char_count = self.input_text.chars().count();

        let start_char = char_index_for_cell_column(&self.input_text, start_col).min(char_count);
        let end_char = char_index_for_cell_column(&self.input_text, end_col).min(char_count);

        if start_char < end_char {
            let chars: Vec<char> = self.input_text.chars().collect();
            self.input_text = chars[..start_char]
                .iter()
                .chain(chars[end_char..].iter())
                .collect();
            self.cursor_pos = start_char;
            self.ime_marked_range = None;
            cx.notify();
        }
        }
    }

    fn input_selection_range(&self) -> Option<std::ops::Range<usize>> {
        let selection = self.selection.as_ref()?;
        if selection.is_empty() {
            return None;
        }

        let lines = self.build_text_lines();
        let input_line = self.input_line_index(&lines);
        let prompt_len = self.get_prompt().chars().count() + 1;
        let (start, end) = selection.normalized();

        if start.line != input_line || end.line != input_line {
            return None;
        }

        let start_col = start.column.saturating_sub(prompt_len);
        let end_col = end.column.saturating_sub(prompt_len);
        let char_count = self.input_text.chars().count();

        let start_char = char_index_for_cell_column(&self.input_text, start_col).min(char_count);
        let end_char = char_index_for_cell_column(&self.input_text, end_col).min(char_count);

        if start_char >= end_char {
            None
        } else {
            Some(start_char..end_char)
        }
    }

    fn replace_input_range(
        &mut self,
        range: std::ops::Range<usize>,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let char_count = self.input_text.chars().count();
        let start = range.start.min(char_count);
        let end = range.end.min(char_count);

        if start > end {
            return;
        }

        let start_byte = byte_index_for_char_index(&self.input_text, start);
        let end_byte = byte_index_for_char_index(&self.input_text, end);

        self.input_text.replace_range(start_byte..end_byte, text);
        self.cursor_pos = start + text.chars().count();
        self.history_index = None;
        self.selection = None;
        cx.notify();
    }

    /// 命令补全（Tab）
    fn complete_command(&mut self, cx: &mut Context<Self>) {
        if self.cursor_pos != self.input_text.chars().count() {
            return;
        }

        let mut start_char = None;
        for (index, ch) in self.input_text.chars().enumerate() {
            if !ch.is_whitespace() {
                start_char = Some(index);
                break;
            }
        }
        let Some(start_char) = start_char else {
            return;
        };

        let mut end_char = self.input_text.chars().count();
        for (index, ch) in self.input_text.chars().enumerate().skip(start_char) {
            if ch.is_whitespace() {
                end_char = index;
                break;
            }
        }

        let start_byte = byte_index_for_char_index(&self.input_text, start_char);
        let end_byte = byte_index_for_char_index(&self.input_text, end_char);
        let command_part = &self.input_text[start_byte..end_byte];
        if command_part.is_empty() {
            return;
        }

        let rest = &self.input_text[end_byte..];
        if rest.chars().any(|ch| !ch.is_whitespace()) {
            return;
        }

        let command_upper = command_part.to_ascii_uppercase();
        let mut matches: Vec<&CommandHint> = COMMAND_HINTS
            .iter()
            .filter(|hint| hint.name.starts_with(&command_upper))
            .collect();

        if matches.len() != 1 {
            return;
        }

        matches.sort_by_key(|hint| hint.name);
        let match_name = matches[0].name;
        let mut new_text = String::new();
        new_text.push_str(&self.input_text[..start_byte]);
        new_text.push_str(match_name);
        if rest.is_empty() {
            new_text.push(' ');
        } else {
            new_text.push_str(rest);
        }

        self.input_text = new_text;
        self.cursor_pos = self.input_text.chars().count();
        self.history_index = None;
        self.selection = None;
        self.ime_marked_range = None;
        cx.notify();
    }

    /// 命令补全（Action）
    fn complete_command_action(
        &mut self,
        _: &CompleteCommand,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.complete_command(cx);
    }

    fn resolve_input_replace_range(
        &self,
        replacement_range_utf16: Option<std::ops::Range<usize>>,
    ) -> std::ops::Range<usize> {
        if let Some(range_utf16) = replacement_range_utf16 {
            let start = char_index_for_utf16_offset(&self.input_text, range_utf16.start);
            let end = char_index_for_utf16_offset(&self.input_text, range_utf16.end);
            return start..end.max(start);
        }

        if let Some(range_utf16) = self.ime_marked_range.as_ref() {
            let start = char_index_for_utf16_offset(&self.input_text, range_utf16.start);
            let end = char_index_for_utf16_offset(&self.input_text, range_utf16.end);
            return start..end.max(start);
        }

        if let Some(range) = self.input_selection_range() {
            return range;
        }

        self.cursor_pos..self.cursor_pos
    }

    /// 全选
    fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        let lines = self.build_text_lines();
        if lines.is_empty() {
            return;
        }

        let last_line = lines.len() - 1;
        let last_line_len = cell_len(&lines[last_line]);

        self.selection = Some(TextSelection::new(
            TextPosition::new(0, 0),
            TextPosition::new(last_line, last_line_len),
            SelectionType::Simple,
        ));
        cx.notify();
    }

    /// 格式化 Redis 值
    fn format_redis_value(&self, value: &RedisValue, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        match value {
            RedisValue::Nil => format!("{}(nil)", prefix),
            RedisValue::String(s) => format!("{}\"{}\"", prefix, s),
            RedisValue::Integer(i) => format!("{}(integer) {}", prefix, i),
            RedisValue::Float(f) => format!("{}(float) {}", prefix, f),
            RedisValue::Status(s) => format!("{}{}", prefix, s),
            RedisValue::Error(e) => format!("{}(error) {}", prefix, e),
            RedisValue::Binary(b) => {
                if let Ok(s) = String::from_utf8(b.clone()) {
                    format!("{}\"{}\"", prefix, s)
                } else {
                    format!("{}<binary: {} bytes>", prefix, b.len())
                }
            }
            RedisValue::Bulk(arr) => {
                if arr.is_empty() {
                    return format!("{}(empty array)", prefix);
                }
                let mut lines = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    lines.push(format!("{}{}) {}", prefix, i + 1, self.format_redis_value(item, 0).trim_start()));
                }
                lines.join("\n")
            }
        }
    }

    /// 构建渲染行列表
    fn build_render_lines(&self) -> Vec<CliLine> {
        let mut lines = Vec::new();

        // 欢迎信息
        lines.push(CliLine {
            line_type: CliLineType::Welcome {
                text: t!("RedisCli.welcome").to_string(),
            },
        });
        lines.push(CliLine {
            line_type: CliLineType::Welcome {
                text: String::new(),
            },
        });

        // 历史命令和结果
        for entry in &self.output_entries {
            // 命令行
            lines.push(CliLine {
                line_type: CliLineType::Prompt {
                    command: entry.command.clone(),
                },
            });

            // 结果行
            let (text, is_error) = match &entry.result {
                CliResult::Success(value) => (self.format_redis_value(value, 0), false),
                CliResult::Error(e) => (format!("(error) {}", e), true),
            };

            // 每行结果单独一个 CliLine
            for line_text in text.lines() {
                lines.push(CliLine {
                    line_type: CliLineType::Result {
                        text: line_text.to_string(),
                        is_error,
                    },
                });
            }
        }

        let hint = self.command_hint_inline();

        // 当前输入行
        lines.push(CliLine {
            line_type: CliLineType::Input {
                prompt: self.get_prompt(),
                text: self.input_text.clone(),
                cursor_pos: self.cursor_pos,
                hint,
            },
        });

        lines
    }

    /// 构建命令提示文本（用于内联显示）
    fn command_hint_inline(&self) -> Option<String> {
        let input = self.input_text.trim_start();
        if input.is_empty() {
            return None;
        }

        let mut parts = input.split_whitespace();
        let command_part = parts.next().unwrap_or("");
        if command_part.is_empty() {
            return None;
        }

        let command_upper = command_part.to_ascii_uppercase();
        let has_trailing_space = self.input_text.ends_with(' ');
        let has_args = input.len() > command_part.len();

        if let Some(hint) = COMMAND_HINTS.iter().find(|hint| hint.name == command_upper) {
            let usage_tail = hint.usage.strip_prefix(hint.name).unwrap_or("");
            let usage_tail = usage_tail.trim_start();
            if usage_tail.is_empty() {
                return None;
            }
            if has_args && !has_trailing_space {
                return None;
            }
            if has_trailing_space {
                return Some(usage_tail.to_string());
            }
            return Some(format!(" {}", usage_tail));
        }

        if has_args {
            return None;
        }

        let mut matches: Vec<&CommandHint> = COMMAND_HINTS
            .iter()
            .filter(|hint| hint.name.starts_with(&command_upper))
            .collect();

        if matches.len() != 1 {
            return None;
        }

        matches.sort_by_key(|hint| hint.name);
        let full = matches[0].name;
        if command_part.len() >= full.len() {
            None
        } else {
            Some(full[command_part.len()..].to_string())
        }
    }

    /// 渲染终端（使用 RedisCliElement 进行精确渲染）
    fn render_terminal(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let lines = self.build_render_lines();
        let text_lines = self.build_text_lines();
        let cursor_visible = self.blink_manager.visible();
        let selection = self.selection;
        let theme = self.theme.clone();
        let cell_width = self.cell_width;
        let scroll_offset = self.scroll_offset;

        RedisCliElement::new(
            lines,
            text_lines,
            theme,
            cursor_visible,
            scroll_offset,
            selection,
            cell_width,
        )
    }

    /// 构建右键上下文菜单
    fn build_context_menu(
        menu: PopupMenu,
        view: &Entity<Self>,
        _window: &mut Window,
        _cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let view_copy = view.clone();
        let view_paste = view.clone();
        let view_clear = view.clone();

        menu
            .item(
                PopupMenuItem::new(t!("Common.copy").to_string())
                    .icon(IconName::Copy)
                    .action(Box::new(Copy))
                    .on_click(move |_, window, cx| {
                        view_copy.update(cx, |this, cx| {
                            this.copy(&Copy, window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(t!("Common.paste").to_string())
                    .action(Box::new(Paste))
                    .on_click(move |_, window, cx| {
                        view_paste.update(cx, |this, cx| {
                            this.paste(&Paste, window, cx);
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(t!("RedisCli.clear_output").to_string())
                    .icon(IconName::Delete)
                    .action(Box::new(ClearOutput))
                    .on_click(move |_, window, cx| {
                        view_clear.update(cx, |this, cx| {
                            this.clear_output_action(&ClearOutput, window, cx);
                        });
                    }),
            )
    }
}

impl EventEmitter<RedisCliViewEvent> for RedisCliView {}
impl EventEmitter<TabContentEvent> for RedisCliView {}

impl Focusable for RedisCliView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TabContent for RedisCliView {
    fn content_key(&self) -> &'static str {
        "RedisCli"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "CLI".into()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Terminal).with_size(Size::Medium))
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
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

impl Render for RedisCliView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let focus_handle = self.focus_handle.clone();
        let entity = cx.entity().downgrade();

        // 焦点变化时更新闪烁状态
        if focus_handle.is_focused(window) {
            self.blink_manager.enable();
        } else {
            self.blink_manager.disable();
        }

        // 精确计算字符宽度（复用 terminal_view 的方法）
        self.update_cell_width(window);

        div()
            .id("redis-cli-view")
            .size_full()
            .bg(rgb(0x1E1E1E))  // 终端深色背景
            .track_focus(&focus_handle)
            .key_context(REDIS_CLI_CONTEXT)
            .on_action(cx.listener(Self::clear_output_action))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::clear_selection))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_to_start))
            .on_action(cx.listener(Self::move_to_end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_to_start))
            .on_action(cx.listener(Self::select_to_end))
            .on_action(cx.listener(Self::complete_command_action))
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .flex_1()
            .relative()
            .on_scroll_wheel(cx.listener(Self::handle_scroll))
            .child(
                // Canvas 用于 IME 输入定位
                canvas(
                    move |bounds, _window, cx| {
                        if let Some(entity) = entity.upgrade() {
                            entity.update(cx, |this, _cx| {
                                this.terminal_bounds = bounds;
                            });
                        }
                    },
                    {
                        let entity = cx.entity().downgrade();
                        let focus_handle = focus_handle.clone();
                        move |bounds, _state, window, cx| {
                            if let Some(entity) = entity.upgrade() {
                                let input_handler = ElementInputHandler::new(bounds, entity);
                                window.handle_input(&focus_handle, input_handler, cx);
                            }
                        }
                    },
                )
                .absolute()
                .left(px(12.))
                .right(px(12.))
                .top(px(12.))
                .bottom(px(12.)),
            )
            .child({
                let view = view.clone();
                div()
                    .absolute()
                    .left(px(12.))
                    .right(px(12.))
                    .top(px(12.))
                    .bottom(px(12.))
                    .child(self.render_terminal(cx))
                    .context_menu(move |menu, window, cx| {
                        Self::build_context_menu(menu, &view, window, cx)
                    })
            })
    }
}

impl EntityInputHandler for RedisCliView {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let total_utf16 = self.input_text.encode_utf16().count();
        let start = range_utf16.start.min(total_utf16);
        let end = range_utf16.end.min(total_utf16);
        actual_range.replace(start..end);

        if start >= end {
            return Some(String::new());
        }

        let start_char = char_index_for_utf16_offset(&self.input_text, start);
        let end_char = char_index_for_utf16_offset(&self.input_text, end);
        let start_byte = byte_index_for_char_index(&self.input_text, start_char);
        let end_byte = byte_index_for_char_index(&self.input_text, end_char);

        Some(self.input_text[start_byte..end_byte].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if let Some(range) = self.input_selection_range() {
            let start_utf16 = utf16_offset_for_char_index(&self.input_text, range.start);
            let end_utf16 = utf16_offset_for_char_index(&self.input_text, range.end);
            let reversed = self
                .selection
                .as_ref()
                .map(|selection| selection.anchor > selection.active)
                .unwrap_or(false);

            return Some(UTF16Selection {
                range: start_utf16..end_utf16,
                reversed,
            });
        }

        let cursor_utf16 = utf16_offset_for_char_index(&self.input_text, self.cursor_pos);
        Some(UTF16Selection {
            range: cursor_utf16..cursor_utf16,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.ime_marked_range.clone()
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.ime_marked_range.is_some() {
            self.ime_marked_range = None;
            cx.notify();
        }
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<std::ops::Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = self.resolve_input_replace_range(replacement_range);
        self.ime_marked_range = None;
        self.replace_input_range(range, text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_marked_range: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = self.resolve_input_replace_range(range);
        let start_char = range.start;
        self.replace_input_range(range, new_text, cx);

        let marked_len = new_text.encode_utf16().count();
        if marked_len == 0 {
            self.ime_marked_range = None;
        } else {
            let start_utf16 = utf16_offset_for_char_index(&self.input_text, start_char);
            self.ime_marked_range = Some(start_utf16..start_utf16 + marked_len);
        }
    }

    fn bounds_for_range(
        &mut self,
        _range: std::ops::Range<usize>,
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // 返回光标位置用于 IME 定位
        let theme = &self.theme;
        let line_height = theme.font_size * theme.line_height_scale;
        let char_width = self.cell_width;

        let prompt_len = self.get_prompt().chars().count() + 1;
        let cursor_col = cell_column_for_char_index(&self.input_text, self.cursor_pos);
        let x = self.terminal_bounds.origin.x + px(12.0)
            + char_width * (prompt_len + cursor_col) as f32;

        let lines = self.build_text_lines();
        let input_line = self.input_line_index(&lines);
        let scroll_offset_px = line_height * self.scroll_offset;
        let y = self.terminal_bounds.origin.y + line_height * input_line as f32 - scroll_offset_px;

        Some(Bounds::new(
            Point::new(x, y),
            size(char_width, line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

/// 判断字符是否为单词字符
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn clamp_position_to_lines(position: TextPosition, lines: &[String]) -> TextPosition {
    if lines.is_empty() {
        return position;
    }

    let line = position.line.min(lines.len().saturating_sub(1));
    let line_len = lines
        .get(line)
        .map(|text| cell_len(text))
        .unwrap_or(0);
    let column = position.column.min(line_len);

    TextPosition::new(line, column)
}

fn byte_index_for_char_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn utf16_offset_for_char_index(text: &str, char_index: usize) -> usize {
    text.chars()
        .take(char_index)
        .map(|ch| ch.len_utf16())
        .sum()
}

fn char_index_for_utf16_offset(text: &str, utf16_index: usize) -> usize {
    let mut current_offset = 0;
    let mut char_index = 0;

    for ch in text.chars() {
        let next_offset = current_offset + ch.len_utf16();
        if next_offset > utf16_index {
            break;
        }
        current_offset = next_offset;
        char_index += 1;
    }

    char_index
}

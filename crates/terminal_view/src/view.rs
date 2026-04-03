use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point as AlacPoint, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_component::notification::Notification;
use gpui_component::scroll::{Scrollbar, ScrollbarHandle, ScrollbarShow};
use gpui_component::{kbd::Kbd, BlinkCursor, Icon, IconName, Sizable, WindowExt};
use std::borrow::Cow;
use std::cell::{Cell as StdCell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::addon::{
    register_default_addons, AddonManager, SearchAddon, TerminalAddonFrameContext,
    TerminalAddonMouseContext,
};
use crate::sidebar::{SidebarPanel, TerminalSidebar, TerminalSidebarEvent};
use crate::terminal_element::{RenderCache, TerminalElement};
use crate::theme::{TerminalTheme, DEFAULT_FONT_SIZE, MAX_FONT_SIZE, MIN_FONT_SIZE};
use one_core::layout::{SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use one_core::storage::models::{ActiveConnections, StoredConnection};
use one_core::tab_container::{TabContent, TabContentEvent};
use one_ui::resize_handle::{resize_handle, HandlePlacement, ResizePanel};
use rust_i18n::t;
use std::ops::Deref;
use terminal::terminal::{
    ConnectionState, Terminal, TerminalConnectionKind, TerminalModelEvent, TerminalScrollProxy,
};
use terminal::LocalConfig;

actions!(
    terminal_view,
    [
        SendTab,
        SendShiftTab,
        Copy,
        Paste,
        SelectAll,
        ClearSelection,
        SearchForward,
        SearchBackward,
        ToggleViMode,
        ViModeStartSelection,
        IncreaseFont,
        DecreaseFont,
        ResetFont,
    ]
);

#[derive(Clone, Debug)]
pub enum TerminalViewEvent {
    FontSizeChanged { size: f32 },
    AutoCopyChanged { enabled: bool },
    MiddleClickPasteChanged { enabled: bool },
    SyncPathChanged { enabled: bool },
    ThemeChanged { theme: TerminalTheme },
    CursorBlinkChanged { enabled: bool },
    ConfirmMultilinePasteChanged { enabled: bool },
    ConfirmHighRiskCommandChanged { enabled: bool },
}

const TERMINAL_CONTEXT: &str = "TerminalView";

#[cfg(target_os = "macos")]
const TERMINAL_COPY_SHORTCUT: &str = "cmd-c";
#[cfg(not(target_os = "macos"))]
const TERMINAL_COPY_SHORTCUT: &str = "ctrl-shift-c";
#[cfg(target_os = "macos")]
const TERMINAL_PASTE_SHORTCUT: &str = "cmd-v";
#[cfg(not(target_os = "macos"))]
const TERMINAL_PASTE_SHORTCUT: &str = "ctrl-shift-v";
#[cfg(target_os = "macos")]
const TERMINAL_SELECT_ALL_SHORTCUT: &str = "cmd-a";
#[cfg(not(target_os = "macos"))]
const TERMINAL_SELECT_ALL_SHORTCUT: &str = "ctrl-shift-a";
#[cfg(target_os = "macos")]
const TERMINAL_SEARCH_FORWARD_SHORTCUT: &str = "cmd-f";
#[cfg(not(target_os = "macos"))]
const TERMINAL_SEARCH_FORWARD_SHORTCUT: &str = "ctrl-shift-f";
#[cfg(target_os = "macos")]
const TERMINAL_SEARCH_BACKWARD_SHORTCUT: &str = "cmd-g";
#[cfg(not(target_os = "macos"))]
const TERMINAL_SEARCH_BACKWARD_SHORTCUT: &str = "ctrl-shift-g";
const TERMINAL_TOGGLE_VI_MODE_SHORTCUT: &str = "f7";

const DEFAULT_CELL_WIDTH: Pixels = px(8.0);
const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 24;

fn take_whole_scroll_lines(scroll_lines_accumulated: &mut f32) -> i32 {
    let lines = scroll_lines_accumulated.trunc() as i32;
    *scroll_lines_accumulated -= lines as f32;
    lines
}

fn alt_screen_scroll_arrow(lines: i32, app_cursor: bool) -> Option<&'static str> {
    if lines == 0 {
        return None;
    }

    Some(match (lines > 0, app_cursor) {
        (true, true) => "\x1bOA",   // Up, application mode
        (true, false) => "\x1b[A",  // Up, normal mode
        (false, true) => "\x1bOB",  // Down, application mode
        (false, false) => "\x1b[B", // Down, normal mode
    })
}

fn should_scroll_to_bottom_on_user_input(
    display_offset: usize,
    pending_display_offset: &StdCell<Option<usize>>,
) -> bool {
    pending_display_offset.take();
    display_offset > 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnbracketedPasteHazard {
    HereDoc,
    UnterminatedQuote,
    LineContinuation,
}

fn multiline_non_empty_line_count(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

fn contains_heredoc_operator(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        !line.is_empty() && !line.starts_with('#') && line.contains("<<")
    })
}

fn has_trailing_line_continuation(text: &str) -> bool {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if lines.peek().is_none() {
            break;
        }

        let trimmed = line.trim_end();
        if !trimmed.is_empty() && trimmed.ends_with('\\') {
            return true;
        }
    }

    false
}

fn has_unterminated_shell_quote(text: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in text.chars() {
        if in_single_quote {
            if ch == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '\'' => in_single_quote = true,
            '"' => in_double_quote = !in_double_quote,
            _ => {}
        }
    }

    in_single_quote || in_double_quote
}

fn detect_unbracketed_paste_hazard(text: &str) -> Option<UnbracketedPasteHazard> {
    if contains_heredoc_operator(text) {
        return Some(UnbracketedPasteHazard::HereDoc);
    }

    if has_trailing_line_continuation(text) {
        return Some(UnbracketedPasteHazard::LineContinuation);
    }

    if has_unterminated_shell_quote(text) {
        return Some(UnbracketedPasteHazard::UnterminatedQuote);
    }

    None
}

fn terminal_shortcut_label(shortcut: &str) -> SharedString {
    Kbd::format(&Keystroke::parse(shortcut).expect("终端快捷键定义非法")).into()
}

/// 对路径进行简单 shell 转义（用单引号包裹，处理内部单引号）
fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == '-' || c == '_')
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// 正在调整大小的面板
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizingPanel {
    Sidebar,
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", SendTab, Some(TERMINAL_CONTEXT)),
        KeyBinding::new("shift-tab", SendShiftTab, Some(TERMINAL_CONTEXT)),
        KeyBinding::new(TERMINAL_COPY_SHORTCUT, Copy, Some(TERMINAL_CONTEXT)),
        KeyBinding::new(TERMINAL_PASTE_SHORTCUT, Paste, Some(TERMINAL_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-insert", Paste, Some(TERMINAL_CONTEXT)),
        KeyBinding::new(
            TERMINAL_SELECT_ALL_SHORTCUT,
            SelectAll,
            Some(TERMINAL_CONTEXT),
        ),
        KeyBinding::new("escape", ClearSelection, Some(TERMINAL_CONTEXT)),
        KeyBinding::new(
            TERMINAL_SEARCH_FORWARD_SHORTCUT,
            SearchForward,
            Some(TERMINAL_CONTEXT),
        ),
        KeyBinding::new(
            TERMINAL_SEARCH_BACKWARD_SHORTCUT,
            SearchBackward,
            Some(TERMINAL_CONTEXT),
        ),
        KeyBinding::new(
            TERMINAL_TOGGLE_VI_MODE_SHORTCUT,
            ToggleViMode,
            Some(TERMINAL_CONTEXT),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-+", IncreaseFont, Some(TERMINAL_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-=", IncreaseFont, Some(TERMINAL_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd--", DecreaseFont, Some(TERMINAL_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-0", ResetFont, Some(TERMINAL_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-+", IncreaseFont, Some(TERMINAL_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-=", IncreaseFont, Some(TERMINAL_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl--", DecreaseFont, Some(TERMINAL_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-0", ResetFont, Some(TERMINAL_CONTEXT)),
    ]);
}

/// IME composition state
struct ImeState {
    marked_range: Option<std::ops::Range<usize>>,
}

/// Terminal view component - supports both Local and SSH backends
pub struct TerminalView {
    /// Terminal model entity
    terminal: Entity<Terminal>,
    /// 本地终端工作目录
    local_working_dir: Option<PathBuf>,
    /// 光标闪烁管理器
    blink_manager: Entity<BlinkCursor>,
    /// 侧边栏
    sidebar: Entity<TerminalSidebar>,

    font_size: Pixels,
    line_height: Pixels,
    cell_width: Pixels,

    last_size: Option<(usize, usize)>,
    scroll_lines_accumulated: f32,

    mouse_state: MouseState,
    addon_manager: AddonManager,

    _subscriptions: Vec<Subscription>,

    mouse_position: Option<Point<Pixels>>,

    render_cache: RenderCache,
    focus_handle: FocusHandle,

    terminal_bounds: Bounds<Pixels>,

    ime_state: Option<ImeState>,

    current_theme: TerminalTheme,

    /// 标签页序号（用于多实例显示）
    tab_index: Option<usize>,

    /// 是否启用光标闪烁
    cursor_blink_enabled: bool,
    /// 非 bracketed 模式下，多行粘贴是否弹确认
    confirm_multiline_paste: bool,
    /// 高危命令是否弹确认
    confirm_high_risk_command: bool,
    /// 选中自动复制
    auto_copy_on_select: bool,
    /// 中键粘贴
    middle_click_paste: bool,

    /// 侧边栏面板大小
    sidebar_panel_size: Pixels,
    /// 正在调整大小的面板
    resizing: Option<ResizingPanel>,
    /// 视图边界
    view_bounds: Bounds<Pixels>,

    scrollbar_metrics: Rc<RefCell<TerminalScrollbarMetrics>>,
    scrollbar_handle: TerminalScrollbarHandle,
}

/// Mouse interaction state
#[derive(Default)]
struct MouseState {
    selecting: bool,
    last_click_point: Option<AlacPoint>,
    click_count: u32,
    last_click_time: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
struct TerminalScrollbarMetrics {
    viewport_size: gpui::Size<Pixels>,
    line_height: Pixels,
    cell_width: Pixels,
}

impl Default for TerminalScrollbarMetrics {
    fn default() -> Self {
        Self {
            viewport_size: size(px(0.0), px(0.0)),
            line_height: px(1.0),
            cell_width: px(1.0),
        }
    }
}

#[derive(Clone)]
struct TerminalScrollbarHandle {
    proxy: TerminalScrollProxy,
    metrics: Rc<RefCell<TerminalScrollbarMetrics>>,
    future_display_offset: Rc<StdCell<Option<usize>>>,
}

impl TerminalScrollbarHandle {
    fn new(proxy: TerminalScrollProxy, metrics: Rc<RefCell<TerminalScrollbarMetrics>>) -> Self {
        Self {
            proxy,
            metrics,
            future_display_offset: Rc::new(StdCell::new(None)),
        }
    }

    fn take_future_display_offset(&self) -> Option<usize> {
        self.future_display_offset.take()
    }
}

impl ScrollbarHandle for TerminalScrollbarHandle {
    fn offset(&self) -> Point<Pixels> {
        let metrics = self.metrics.borrow();
        let line_height = metrics.line_height.max(px(1.0));
        // Snapshot terminal state in a single lock to avoid inconsistency
        let snapshot = self.proxy.snapshot();
        let max_offset = snapshot.history_size;
        let scroll_offset = max_offset.saturating_sub(snapshot.display_offset);
        Point::new(px(0.0), -(scroll_offset as f32 * line_height))
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        let metrics = self.metrics.borrow();
        let line_height = metrics.line_height.max(px(1.0));
        let snapshot = self.proxy.snapshot();
        let max_offset = snapshot.history_size as i32;
        if max_offset == 0 {
            return;
        }
        let offset_delta = (offset.y / line_height).round() as i32;
        let display_offset = (max_offset + offset_delta).clamp(0, max_offset) as usize;
        self.future_display_offset.set(Some(display_offset));
    }

    fn content_size(&self) -> gpui::Size<Pixels> {
        let metrics = self.metrics.borrow();
        let line_height = metrics.line_height.max(px(1.0));
        let snapshot = self.proxy.snapshot();
        let total_lines = snapshot.history_size + snapshot.screen_lines;
        let height = line_height * total_lines as f32;
        let width = metrics
            .viewport_size
            .width
            .max(metrics.cell_width * snapshot.columns as f32);
        size(width, height)
    }
}

impl TerminalView {
    fn release_active_connection(&self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.terminal.read(cx).connection_id() else {
            return;
        };

        cx.global_mut::<ActiveConnections>().remove(connection_id);
    }

    pub fn new(config: LocalConfig, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_index(config, None, window, cx)
    }

    pub fn new_with_index(
        config: LocalConfig,
        tab_index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // 创建 Terminal Entity
        let local_working_dir = config.working_dir.clone().map(PathBuf::from);
        let init_error = Rc::new(RefCell::new(None));
        let init_error_clone = init_error.clone();
        let terminal = cx.new(move |cx| {
            let (terminal, error) = Terminal::new_local_or_disconnected(config, cx);
            *init_error_clone.borrow_mut() = error;
            terminal
        });
        let view = Self::new_with_terminal(
            terminal,
            None,
            None,
            true,
            local_working_dir,
            tab_index,
            window,
            cx,
        );

        if let Some(error) = init_error.borrow_mut().take() {
            window.push_notification(
                Notification::error(format!("创建本地终端失败: {}", error)).autohide(true),
                cx,
            );
        }

        view
    }

    pub fn new_ssh(conn: StoredConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_ssh_with_index(conn, None, window, cx, None, true)
    }

    pub fn new_ssh_with_index(
        conn: StoredConnection,
        tab_index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
        working_dir: Option<&str>,
        sync_path_with_terminal: bool,
    ) -> Self {
        // 创建 SSH Terminal Entity
        let connection_id = conn.id;
        let stored_conn = conn.clone();
        let terminal =
            cx.new(|cx| Terminal::new_ssh(conn, cx, working_dir, sync_path_with_terminal));
        Self::new_with_terminal(
            terminal,
            connection_id,
            Some(stored_conn),
            sync_path_with_terminal,
            None,
            tab_index,
            window,
            cx,
        )
    }

    pub fn new_serial(conn: StoredConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_serial_with_index(conn, None, window, cx)
    }

    pub fn new_serial_with_index(
        conn: StoredConnection,
        tab_index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let connection_id = conn.id;
        let terminal = cx.new(|cx| Terminal::new_serial(conn, cx));
        // 串口不传 stored_connection，避免创建文件管理器面板
        Self::new_with_terminal(
            terminal,
            connection_id,
            None,
            true,
            None,
            tab_index,
            window,
            cx,
        )
    }

    fn new_with_terminal(
        terminal: Entity<Terminal>,
        connection_id: Option<i64>,
        stored_connection: Option<StoredConnection>,
        sync_path_enabled: bool,
        local_working_dir: Option<PathBuf>,
        tab_index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let blink_manager = cx.new(|_| BlinkCursor::new());

        // 获取初始颜色
        let colors = terminal.read(cx).term().lock().colors().clone();
        let is_local_terminal =
            terminal.read(cx).connection_kind() == TerminalConnectionKind::Local;

        // 创建默认主题（需要在创建侧边栏之前）
        let default_theme = TerminalTheme::ocean();
        let ssh_config = terminal.read(cx).ssh_config().cloned();

        // 创建侧边栏（传递 StoredConnection 用于文件管理器）
        let sidebar = cx.new(|cx| {
            TerminalSidebar::new(
                connection_id,
                stored_connection,
                ssh_config,
                &default_theme,
                sync_path_enabled,
                window,
                cx,
            )
        });

        // 订阅侧边栏事件（需要 window 以便弹确认对话框）
        let sidebar_subscription = cx.subscribe_in(&sidebar, window, Self::handle_sidebar_event);

        // 订阅 Terminal 事件
        let terminal_subscription = cx.subscribe(&terminal, Self::handle_terminal_event);

        // 订阅 BlinkCursor 变化
        let blink_subscription = cx.observe(&blink_manager, |this, _, cx| {
            cx.notify();
            let _ = this;
        });

        let focus_handle = cx.focus_handle();

        // 焦点获得/失去订阅
        let focus_subscription = cx.on_focus(&focus_handle, window, |this, _window, cx| {
            if this.cursor_blink_enabled {
                this.blink_manager.update(cx, BlinkCursor::start);
            }
        });
        let blur_subscription = cx.on_blur(&focus_handle, window, |this, _window, cx| {
            if this.cursor_blink_enabled {
                this.blink_manager.update(cx, BlinkCursor::stop);
            }
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(sidebar_subscription);
        subscriptions.push(terminal_subscription);
        subscriptions.push(blink_subscription);
        subscriptions.push(focus_subscription);
        subscriptions.push(blur_subscription);

        let scrollbar_metrics = Rc::new(RefCell::new(TerminalScrollbarMetrics::default()));
        let scrollbar_handle = TerminalScrollbarHandle::new(
            terminal.read(cx).scroll_proxy(),
            scrollbar_metrics.clone(),
        );

        Self {
            terminal,
            local_working_dir: if is_local_terminal {
                local_working_dir
            } else {
                None
            },
            blink_manager,
            sidebar,
            font_size: default_theme.font_size,
            line_height: default_theme.line_height(),
            cell_width: DEFAULT_CELL_WIDTH,
            // 初始化为 None，确保首次渲染时会触发 resize，
            // 将正确的终端尺寸发送给 PTY
            last_size: None,
            scroll_lines_accumulated: 0.0,
            mouse_state: MouseState::default(),
            addon_manager: Self::create_addon_manager(),
            _subscriptions: subscriptions,
            mouse_position: None,
            render_cache: RenderCache::new(DEFAULT_ROWS, DEFAULT_COLS, colors),
            focus_handle,
            terminal_bounds: Bounds::default(),
            ime_state: None,
            current_theme: default_theme,
            tab_index,
            cursor_blink_enabled: false,
            confirm_multiline_paste: true,
            confirm_high_risk_command: true,
            auto_copy_on_select: true,
            middle_click_paste: true,
            sidebar_panel_size: SIDEBAR_DEFAULT_WIDTH,
            resizing: None,
            view_bounds: Bounds::default(),
            scrollbar_metrics,
            scrollbar_handle,
        }
    }

    /// 处理侧边栏事件
    fn handle_sidebar_event(
        &mut self,
        _sidebar: &Entity<TerminalSidebar>,
        event: &TerminalSidebarEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            TerminalSidebarEvent::PanelChanged(_panel) => {
                cx.notify();
            }
            TerminalSidebarEvent::SearchPatternChanged(pattern) => {
                let _ = self.set_search_pattern(pattern);
                cx.notify();
            }
            TerminalSidebarEvent::SearchPrevious => {
                self.search_backward_internal(cx);
            }
            TerminalSidebarEvent::SearchNext => {
                self.search_forward_internal(cx);
            }
            TerminalSidebarEvent::FontSizeChanged(size) => {
                self.set_font_size(*size, cx);
            }
            TerminalSidebarEvent::FontFamilyChanged(family) => {
                self.set_font_family(family.clone(), cx);
            }
            TerminalSidebarEvent::ThemeChanged(theme) => {
                self.set_theme(theme.clone(), cx);
                cx.emit(TerminalViewEvent::ThemeChanged {
                    theme: theme.clone(),
                });
            }
            TerminalSidebarEvent::ExecuteCommand(command) => {
                // 仅粘贴命令，不自动回车执行，降低误操作风险
                self.paste_text(command, window, cx);
            }
            TerminalSidebarEvent::PasteCodeToTerminal(code) => {
                // 粘贴代码块到终端（使用 bracketed paste 模式，不自动执行）
                self.paste_code_block(&code, window, cx);
            }
            TerminalSidebarEvent::AskAi => {
                // AI 请求已由 sidebar 内部处理，这里只需要通知刷新
                cx.notify();
            }
            TerminalSidebarEvent::CursorBlinkChanged(enabled) => {
                self.cursor_blink_enabled = *enabled;
                if *enabled {
                    self.blink_manager.update(cx, BlinkCursor::start);
                } else {
                    self.blink_manager.update(cx, BlinkCursor::stop);
                }
                cx.emit(TerminalViewEvent::CursorBlinkChanged { enabled: *enabled });
            }
            TerminalSidebarEvent::ConfirmMultilinePasteChanged(enabled) => {
                self.confirm_multiline_paste = *enabled;
                cx.emit(TerminalViewEvent::ConfirmMultilinePasteChanged { enabled: *enabled });
            }
            TerminalSidebarEvent::ConfirmHighRiskCommandChanged(enabled) => {
                self.confirm_high_risk_command = *enabled;
                cx.emit(TerminalViewEvent::ConfirmHighRiskCommandChanged { enabled: *enabled });
            }
            TerminalSidebarEvent::AutoCopyChanged(enabled) => {
                self.set_auto_copy(*enabled, cx);
            }
            TerminalSidebarEvent::MiddleClickPasteChanged(enabled) => {
                self.set_middle_click_paste(*enabled, cx);
            }
            TerminalSidebarEvent::SyncPathChanged(enabled) => {
                cx.emit(TerminalViewEvent::SyncPathChanged { enabled: *enabled });
            }
            TerminalSidebarEvent::CdToTerminal(path) => {
                // 向终端发送 cd 命令并回车
                let cmd = format!("cd {}\n", shell_escape(path));
                self.write_to_pty(cmd.into_bytes(), cx);
            }
            TerminalSidebarEvent::SyncWorkingDir => {
                if let Some(path) = self
                    .terminal
                    .read(cx)
                    .current_working_dir()
                    .map(str::to_string)
                {
                    self.sidebar.update(cx, |sidebar, cx| {
                        sidebar.sync_file_manager_path(path, cx);
                    });
                }
            }
        }
    }

    /// 内部搜索：向前搜索
    fn search_forward_internal(&mut self, cx: &mut Context<Self>) {
        if let Some(search) = self.addon_manager.get_as_mut::<SearchAddon>("search") {
            let term = self.terminal.read(cx).term().clone();
            let mut term = term.lock();
            search.find_next(&mut term);
        }
        cx.notify();
    }

    /// 内部搜索：向后搜索
    fn search_backward_internal(&mut self, cx: &mut Context<Self>) {
        if let Some(search) = self.addon_manager.get_as_mut::<SearchAddon>("search") {
            let term = self.terminal.read(cx).term().clone();
            let mut term = term.lock();
            search.find_previous(&mut term);
        }
        cx.notify();
    }

    fn handle_terminal_event(
        &mut self,
        _terminal: Entity<Terminal>,
        event: &TerminalModelEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TerminalModelEvent::Wakeup => {
                cx.notify();
            }
            TerminalModelEvent::TitleChanged(_) => {
                cx.emit(TabContentEvent::StateChanged);
            }
            TerminalModelEvent::Bell => {
                // 可选：播放声音或闪烁标签
            }
            TerminalModelEvent::ChildExit(_) => {
                cx.notify();
            }
            TerminalModelEvent::ClipboardStore(data) => {
                cx.write_to_clipboard(ClipboardItem::new_string(data.clone()));
            }
            TerminalModelEvent::WorkingDirChanged(path) => {
                let path = path.clone();
                self.sidebar.update(cx, |sidebar, cx| {
                    sidebar.set_file_manager_initial_dir(path.clone(), cx);
                    sidebar.sync_file_manager_path(path, cx);
                });
            }
        }
    }

    fn create_addon_manager() -> AddonManager {
        let mut manager = AddonManager::new();
        register_default_addons(&mut manager);
        manager
    }

    /// Apply a terminal theme
    pub fn set_theme(&mut self, theme: TerminalTheme, cx: &mut Context<Self>) {
        self.current_theme = theme;
        self.font_size = self.current_theme.font_size;
        self.line_height = self.current_theme.line_height();
        cx.notify();
    }

    /// Get current theme
    pub fn current_theme(&self) -> &TerminalTheme {
        &self.current_theme
    }

    /// 获取连接类型（本地 / SSH）
    pub fn connection_kind(&self, cx: &App) -> TerminalConnectionKind {
        self.terminal.read(cx).connection_kind()
    }

    /// 获取 SSH 连接 ID（本地终端返回 None）
    pub fn connection_id(&self, cx: &App) -> Option<i64> {
        self.terminal.read(cx).connection_id()
    }

    /// 获取本地终端的工作目录
    pub fn local_working_dir(&self) -> Option<&std::path::Path> {
        self.local_working_dir.as_deref()
    }

    /// Get all available themes
    pub fn available_themes() -> Vec<TerminalTheme> {
        TerminalTheme::all()
    }

    /// 设置字体大小
    pub fn set_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        let clamped = size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        let current = f32::from(self.current_theme.font_size);
        if (current - clamped).abs() < f32::EPSILON {
            return;
        }
        self.current_theme.font_size = px(clamped);
        self.font_size = self.current_theme.font_size;
        self.line_height = self.current_theme.line_height();
        cx.emit(TerminalViewEvent::FontSizeChanged { size: clamped });
        cx.notify();
    }

    pub fn apply_terminal_settings(
        &mut self,
        font_size: f32,
        auto_copy: bool,
        middle_click_paste: bool,
        sync_path: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 字体大小
        let clamped = font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        let current = f32::from(self.current_theme.font_size);
        if (current - clamped).abs() >= f32::EPSILON {
            self.current_theme.font_size = px(clamped);
            self.font_size = self.current_theme.font_size;
            self.line_height = self.current_theme.line_height();
        }

        self.auto_copy_on_select = auto_copy;
        self.middle_click_paste = middle_click_paste;

        self.terminal.update(cx, |terminal, _cx| {
            terminal.set_sync_path_with_terminal(sync_path);
        });

        let theme = self.current_theme.clone();
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.update_current_theme(&theme, window, cx);
            sidebar.set_auto_copy(auto_copy, cx);
            sidebar.set_middle_click_paste(middle_click_paste, cx);
            sidebar.set_sync_path_enabled(sync_path, cx);
        });

        cx.notify();
    }

    /// 应用主题（不 emit 事件，用于跨 tab 同步）
    pub fn apply_theme(
        &mut self,
        theme: &TerminalTheme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.current_theme.name == theme.name {
            return;
        }
        self.current_theme = theme.clone();
        self.font_size = self.current_theme.font_size;
        self.line_height = self.current_theme.line_height();
        self.sync_sidebar_theme(window, cx);
        cx.notify();
    }

    /// 应用光标闪烁（不 emit 事件，用于跨 tab 同步）
    pub fn apply_cursor_blink(
        &mut self,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cursor_blink_enabled = enabled;
        if enabled {
            if self.focus_handle.is_focused(window) {
                self.blink_manager.update(cx, BlinkCursor::start);
            }
        } else {
            self.blink_manager.update(cx, BlinkCursor::stop);
        }
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_cursor_blink(enabled, cx);
        });
        cx.notify();
    }

    /// 应用多行粘贴确认（不 emit 事件，用于跨 tab 同步）
    pub fn apply_confirm_multiline_paste(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.confirm_multiline_paste = enabled;
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_confirm_multiline_paste(enabled, cx);
        });
        cx.notify();
    }

    /// 应用高危命令确认（不 emit 事件，用于跨 tab 同步）
    pub fn apply_confirm_high_risk_command(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.confirm_high_risk_command = enabled;
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_confirm_high_risk_command(enabled, cx);
        });
        cx.notify();
    }

    pub fn sync_sidebar_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let theme = self.current_theme.clone();
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.update_current_theme(&theme, window, cx);
        });
    }

    pub fn set_auto_copy(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.auto_copy_on_select == enabled {
            return;
        }
        self.auto_copy_on_select = enabled;
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_auto_copy(enabled, cx);
        });
        cx.emit(TerminalViewEvent::AutoCopyChanged { enabled });
        cx.notify();
    }

    pub fn set_middle_click_paste(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.middle_click_paste == enabled {
            return;
        }
        self.middle_click_paste = enabled;
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_middle_click_paste(enabled, cx);
        });
        cx.emit(TerminalViewEvent::MiddleClickPasteChanged { enabled });
        cx.notify();
    }

    /// 增大字体
    pub fn increase_font_size(&mut self, cx: &mut Context<Self>) {
        let current = f32::from(self.current_theme.font_size);
        self.set_font_size(current + 1.0, cx);
    }

    /// 减小字体
    pub fn decrease_font_size(&mut self, cx: &mut Context<Self>) {
        let current = f32::from(self.current_theme.font_size);
        self.set_font_size(current - 1.0, cx);
    }

    /// 重置字体大小为默认值
    pub fn reset_font_size(&mut self, cx: &mut Context<Self>) {
        self.set_font_size(DEFAULT_FONT_SIZE, cx);
    }

    /// 获取当前字体大小
    pub fn font_size(&self) -> f32 {
        f32::from(self.current_theme.font_size)
    }

    /// 设置主字体
    pub fn set_font_family(&mut self, family: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.current_theme.font_family = family.into();
        cx.notify();
    }

    /// 获取当前主字体
    pub fn font_family(&self) -> &SharedString {
        &self.current_theme.font_family
    }

    /// 设置行高比例
    pub fn set_line_height_scale(&mut self, scale: f32, cx: &mut Context<Self>) {
        self.current_theme.line_height_scale = scale.clamp(1.0, 2.5);
        self.line_height = self.current_theme.line_height();
        cx.notify();
    }

    /// 获取当前行高比例
    pub fn line_height_scale(&self) -> f32 {
        self.current_theme.line_height_scale
    }

    pub fn reconnect(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let working_dir = self
            .terminal
            .read(cx)
            .current_working_dir()
            .map(str::to_string);
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.reconnect_file_manager(working_dir, cx);
            sidebar.reconnect_server_monitor(cx);
        });
        self.terminal.update(cx, |terminal, cx| {
            terminal.reconnect(cx);
        });
    }

    fn write_to_pty(&mut self, data: Vec<u8>, cx: &mut Context<Self>) {
        // 用户输入时自动滚动到底部
        let display_offset = self.terminal.read(cx).term().lock().grid().display_offset();
        if should_scroll_to_bottom_on_user_input(
            display_offset,
            &self.scrollbar_handle.future_display_offset,
        ) {
            self.terminal.update(cx, |terminal, _| {
                terminal
                    .term()
                    .lock()
                    .scroll_display(alacritty_terminal::grid::Scroll::Bottom);
            });
        }
        self.terminal.read(cx).write(&data);
    }

    fn commit_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if !text.is_empty() {
            self.write_to_pty(text.as_bytes().to_vec(), cx);
        }
    }

    fn set_marked_text(
        &mut self,
        _text: String,
        range: Option<std::ops::Range<usize>>,
        cx: &mut Context<Self>,
    ) {
        self.ime_state = Some(ImeState {
            marked_range: range,
        });
        cx.notify();
    }

    fn clear_marked_text(&mut self, cx: &mut Context<Self>) {
        if self.ime_state.is_some() {
            self.ime_state = None;
            cx.notify();
        }
    }

    fn marked_text_range(&self) -> Option<std::ops::Range<usize>> {
        self.ime_state
            .as_ref()
            .and_then(|state| state.marked_range.clone())
    }

    fn handle_key_event(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 输入时暂停闪烁
        if self.cursor_blink_enabled {
            self.blink_manager.update(cx, BlinkCursor::pause);
        }

        #[cfg(target_os = "macos")]
        if event.keystroke.modifiers.platform && event.keystroke.key == "v" {
            self.paste(&Paste, _window, cx);
            return;
        }

        #[cfg(not(target_os = "macos"))]
        if event.keystroke.modifiers.control
            && event.keystroke.modifiers.shift
            && event.keystroke.key == "v"
        {
            self.paste(&Paste, _window, cx);
            return;
        }

        #[cfg(target_os = "macos")]
        if event.keystroke.modifiers.platform && event.keystroke.key == "c" {
            self.copy(&Copy, _window, cx);
            return;
        }

        #[cfg(not(target_os = "macos"))]
        if event.keystroke.modifiers.control
            && event.keystroke.modifiers.shift
            && event.keystroke.key == "c"
        {
            self.copy(&Copy, _window, cx);
            return;
        }

        let mode = self.terminal.read(cx).mode();

        if mode.contains(TermMode::VI) {
            self.handle_vi_key_event(event, cx);
            return;
        }

        if let Some(esc_str) = crate::keys::to_esc_str(&event.keystroke, &mode, false) {
            let bytes = match esc_str {
                Cow::Borrowed(s) => s.as_bytes().to_vec(),
                Cow::Owned(s) => s.into_bytes(),
            };
            self.write_to_pty(bytes, cx);
        }
    }

    fn handle_vi_key_event(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        use alacritty_terminal::vi_mode::ViMotion;

        let key = &event.keystroke.key;
        let shift = event.keystroke.modifiers.shift;
        let ctrl = event.keystroke.modifiers.control;

        let motion = match (key.as_str(), shift, ctrl) {
            ("h", true, false) => Some(ViMotion::High),
            ("m", true, false) => Some(ViMotion::Middle),
            ("l", true, false) => Some(ViMotion::Low),
            ("b", true, false) => Some(ViMotion::WordLeft),
            ("w", true, false) => Some(ViMotion::WordRight),
            ("e", true, false) => Some(ViMotion::WordRightEnd),
            ("h" | "left", false, false) => Some(ViMotion::Left),
            ("j" | "down", false, false) => Some(ViMotion::Down),
            ("k" | "up", false, false) => Some(ViMotion::Up),
            ("l" | "right", false, false) => Some(ViMotion::Right),
            ("0", _, false) => Some(ViMotion::First),
            ("$", _, false) => Some(ViMotion::Last),
            ("^", _, false) => Some(ViMotion::FirstOccupied),
            ("b", false, false) => Some(ViMotion::SemanticLeft),
            ("w", false, false) => Some(ViMotion::SemanticRight),
            ("e", false, false) => Some(ViMotion::SemanticRightEnd),
            ("%", _, false) => Some(ViMotion::Bracket),
            ("{", _, false) => Some(ViMotion::ParagraphUp),
            ("}", _, false) => Some(ViMotion::ParagraphDown),
            _ => None,
        };

        if let Some(ref motion) = motion {
            let term = self.terminal.read(cx).term().clone();
            let mut term = term.lock();
            term.vi_motion(motion.clone());
            drop(term);
            cx.notify();
            return;
        }

        let term = self.terminal.read(cx).term().clone();

        match key.as_str() {
            "v" if !ctrl && !shift => {
                self.vi_start_selection(SelectionType::Simple, cx);
            }
            "v" if shift => {
                self.vi_start_selection(SelectionType::Lines, cx);
            }
            "y" => {
                let term = term.lock();
                if let Some(text) = term.selection_to_string() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                drop(term);
                self.terminal.read(cx).term().lock().selection = None;
                cx.notify();
            }
            "u" if ctrl => {
                let mut term = term.lock();
                let lines = term.screen_lines() as i32 / 2;
                let vi_cursor = term.vi_mode_cursor.scroll(&term, lines);
                term.vi_goto_point(vi_cursor.point);
                drop(term);
                cx.notify();
            }
            "d" if ctrl => {
                let mut term = term.lock();
                let lines = term.screen_lines() as i32 / 2;
                let vi_cursor = term.vi_mode_cursor.scroll(&term, -lines);
                term.vi_goto_point(vi_cursor.point);
                drop(term);
                cx.notify();
            }
            "g" if !shift => {
                let mut term = term.lock();
                let point = AlacPoint::new(Line(term.topmost_line().0), Column(0));
                term.vi_goto_point(point);
                drop(term);
                cx.notify();
            }
            "g" if shift => {
                let mut term = term.lock();
                let point = AlacPoint::new(term.bottommost_line(), Column(0));
                term.vi_goto_point(point);
                drop(term);
                cx.notify();
            }
            _ => {}
        }
    }

    fn vi_start_selection(&mut self, selection_type: SelectionType, cx: &mut Context<Self>) {
        use alacritty_terminal::selection::Selection;

        let term = self.terminal.read(cx).term().clone();
        let mut term = term.lock();
        let point = term.vi_mode_cursor.point;
        if term.selection.is_some() {
            term.selection = None;
        } else {
            term.selection = Some(Selection::new(selection_type, point, Side::Left));
        }
        drop(term);
        cx.notify();
    }

    fn toggle_vi_mode(&mut self, _: &ToggleViMode, window: &mut Window, cx: &mut Context<Self>) {
        let in_vi_mode = self.terminal.update(cx, |terminal, _| {
            terminal.toggle_vi_mode();
            terminal.mode().contains(TermMode::VI)
        });
        let shortcut = terminal_shortcut_label(TERMINAL_TOGGLE_VI_MODE_SHORTCUT);
        let message = if in_vi_mode {
            t!("TerminalView.vi_mode_enabled", shortcut = shortcut).to_string()
        } else {
            t!("TerminalView.vi_mode_disabled", shortcut = shortcut).to_string()
        };
        window.push_notification(message, cx);
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.terminal.read(cx).selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        self.focus_terminal(window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(clipboard) = cx.read_from_clipboard() {
            if let Some(text) = clipboard.text() {
                self.paste_text(&text, window, cx);
            }
        }
    }

    fn increase_font(&mut self, _: &IncreaseFont, window: &mut Window, cx: &mut Context<Self>) {
        self.increase_font_size(cx);
        self.sync_sidebar_theme(window, cx);
    }

    fn decrease_font(&mut self, _: &DecreaseFont, window: &mut Window, cx: &mut Context<Self>) {
        self.decrease_font_size(cx);
        self.sync_sidebar_theme(window, cx);
    }

    fn reset_font(&mut self, _: &ResetFont, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_font_size(cx);
        self.sync_sidebar_theme(window, cx);
    }

    /// 粘贴文本到终端
    ///
    /// 统一使用 bracketed paste 模式处理所有粘贴操作，确保：
    /// 1. 多行文本不会被立即执行（每一行都需要用户确认）
    /// 2. 保持文本的完整性，让用户可以检查后再执行
    /// 3. 避免意外执行危险命令
    fn paste_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mode = self.terminal.read(cx).mode();

        // ALT_SCREEN（如 Vim、less）属于全屏交互程序，粘贴内容不会像 shell 那样直接执行。
        // 这里跳过高危/多行确认，避免编辑器场景误弹确认框。
        if mode.contains(TermMode::ALT_SCREEN) {
            self.paste_text_unchecked(text, window, cx);
            return;
        }

        if self.confirm_high_risk_command && Self::contains_high_risk_command(text) {
            self.show_paste_confirm_dialog(
                text.to_string(),
                t!("TerminalView.high_risk_paste_title").to_string(),
                t!("TerminalView.high_risk_paste_message").to_string(),
                window,
                cx,
            );
            return;
        }

        let is_bracketed_paste = mode.contains(TermMode::BRACKETED_PASTE);

        if !is_bracketed_paste {
            if let Some(hazard) = detect_unbracketed_paste_hazard(text) {
                self.show_unbracketed_paste_block_dialog(text, hazard, window, cx);
                return;
            }
        }

        let is_multiline = multiline_non_empty_line_count(text) > 1;
        if self.confirm_multiline_paste && is_multiline && !is_bracketed_paste {
            self.show_paste_confirm_dialog(
                text.to_string(),
                t!("TerminalView.multiline_paste_title").to_string(),
                t!("TerminalView.multiline_paste_message").to_string(),
                window,
                cx,
            );
            return;
        }

        self.paste_text_unchecked(text, window, cx);
    }

    fn paste_text_unchecked(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        // 仅在应用请求 bracketed paste 模式时才包装，避免把控制序列
        // 原样送进不支持的程序（例如 Vim 未开启时可能导致光标/位置异常）。
        let mode = self.terminal.read(cx).mode();
        if mode.contains(TermMode::BRACKETED_PASTE) {
            let paste_text = format!("\x1b[200~{}\x1b[201~", text.replace('\x1b', ""));
            self.write_to_pty(paste_text.into_bytes(), cx);
        } else {
            self.write_to_pty(text.as_bytes().to_vec(), cx);
        }
        self.focus_terminal(window, cx);
    }

    /// 粘贴代码块到终端（用于AI生成的代码）
    ///
    /// 内部调用 paste_text，保持统一的粘贴行为
    fn paste_code_block(&mut self, code: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.paste_text(code, window, cx);
    }

    fn paste_preview_text(text: &str) -> String {
        let preview = text.lines().take(6).collect::<Vec<_>>().join("\n");
        if text.lines().count() > 6 {
            format!("{preview}\n...")
        } else {
            preview
        }
    }

    fn show_paste_confirm_dialog(
        &mut self,
        text: String,
        title: String,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let preview_text = Self::paste_preview_text(&text);
        let view = cx.entity().clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let view_ok = view.clone();
            let text_ok = text.clone();

            dialog
                .title(title.clone())
                .confirm()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().text_sm().child(message.clone()))
                        .child(div().text_xs().child(t!("TerminalView.paste_preview")))
                        .child(
                            div()
                                .max_h(px(180.0))
                                .overflow_hidden()
                                .text_xs()
                                .child(preview_text.clone()),
                        )
                        .into_any_element(),
                )
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.ok"))
                        .cancel_text(t!("Common.cancel")),
                )
                .on_ok(move |_event, window, cx| {
                    view_ok.update(cx, |this, cx| {
                        this.paste_text_unchecked(&text_ok, window, cx);
                    });
                    true
                })
        });
    }

    fn focus_terminal(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
    }

    fn show_unbracketed_paste_block_dialog(
        &mut self,
        text: &str,
        hazard: UnbracketedPasteHazard,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = t!("TerminalView.unbracketed_paste_block_title").to_string();
        let message = match hazard {
            UnbracketedPasteHazard::HereDoc => {
                t!("TerminalView.unbracketed_paste_heredoc_message").to_string()
            }
            UnbracketedPasteHazard::UnterminatedQuote => {
                t!("TerminalView.unbracketed_paste_quote_message").to_string()
            }
            UnbracketedPasteHazard::LineContinuation => {
                t!("TerminalView.unbracketed_paste_continuation_message").to_string()
            }
        };
        let preview_text = Self::paste_preview_text(text);

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title(title.clone())
                .alert()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().text_sm().child(message.clone()))
                        .child(div().text_xs().child(t!("TerminalView.paste_preview")))
                        .child(
                            div()
                                .max_h(px(180.0))
                                .overflow_hidden()
                                .text_xs()
                                .child(preview_text.clone()),
                        )
                        .into_any_element(),
                )
                .button_props(DialogButtonProps::default().ok_text(t!("Common.close")))
        });
    }

    fn contains_high_risk_command(text: &str) -> bool {
        text.lines().any(|line| {
            let cmd = line.trim().to_lowercase();
            if cmd.is_empty() {
                return false;
            }

            cmd.starts_with("rm -rf")
                || cmd.contains(" rm -rf ")
                || cmd.starts_with("mkfs")
                || cmd.starts_with("dd if=")
                || cmd.starts_with("shutdown ")
                || cmd.starts_with("reboot")
                || cmd.starts_with("poweroff")
                || cmd.starts_with("systemctl stop ")
                || cmd.starts_with("systemctl disable ")
                || cmd.starts_with("chmod -r 777 /")
                || cmd.starts_with("chown -r ")
                || cmd.contains(":(){")
                || cmd.contains("curl ") && (cmd.contains("| sh") || cmd.contains("| bash"))
                || cmd.contains("wget ") && (cmd.contains("| sh") || cmd.contains("| bash"))
        })
    }

    fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.terminal.update(cx, |terminal, _| {
            terminal.select_all();
        });
        cx.notify();
    }

    fn clear_selection(&mut self, _: &ClearSelection, window: &mut Window, cx: &mut Context<Self>) {
        // 如果侧边栏有激活的面板，按 Escape 关闭它
        if self.sidebar.read(cx).active_panel().is_some() {
            self.sidebar.update(cx, |sidebar, cx| {
                sidebar.set_active_panel(None, cx);
            });
            // 清除搜索
            self.sidebar.update(cx, |sidebar, cx| {
                sidebar.set_search_value("", window, cx);
            });
            if let Some(search) = self.addon_manager.get_as_mut::<SearchAddon>("search") {
                search.clear();
            }
            cx.notify();
            return;
        }

        let term = self.terminal.read(cx).term().clone();
        let mut term_lock = term.lock();
        let in_vi_mode = term_lock.mode().contains(TermMode::VI);
        let has_selection = term_lock.selection.is_some();

        if in_vi_mode {
            if has_selection {
                term_lock.selection = None;
            } else {
                term_lock.toggle_vi_mode();
            }
            drop(term_lock);
            cx.notify();
        } else if has_selection {
            term_lock.selection = None;
            drop(term_lock);
            cx.notify();
        } else {
            drop(term_lock);
            self.write_to_pty(b"\x1b".to_vec(), cx);
        }
    }

    fn search_forward(&mut self, _: &SearchForward, _window: &mut Window, cx: &mut Context<Self>) {
        // 如果侧边栏设置面板未激活，则激活它
        if self.sidebar.read(cx).active_panel() != Some(SidebarPanel::Settings) {
            self.sidebar.update(cx, |sidebar, cx| {
                sidebar.set_active_panel(Some(SidebarPanel::Settings), cx);
            });
            cx.notify();
            return;
        }
        // 执行向前搜索
        self.search_forward_internal(cx);
    }

    fn search_backward(
        &mut self,
        _: &SearchBackward,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 如果侧边栏设置面板未激活，则激活它
        if self.sidebar.read(cx).active_panel() != Some(SidebarPanel::Settings) {
            self.sidebar.update(cx, |sidebar, cx| {
                sidebar.set_active_panel(Some(SidebarPanel::Settings), cx);
            });
            cx.notify();
            return;
        }
        // 执行向后搜索
        self.search_backward_internal(cx);
    }

    pub fn set_search_pattern(&mut self, pattern: &str) -> Result<()> {
        if let Some(search) = self.addon_manager.get_as_mut::<SearchAddon>("search") {
            search
                .set_pattern(pattern)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Ok(())
    }

    fn resize_if_needed(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let cols = (bounds.size.width / self.cell_width).floor() as usize;
        let rows = (bounds.size.height / self.line_height).floor() as usize;

        let cols = cols.max(1);
        let rows = rows.max(1);

        let new_size = (cols, rows);
        if self.last_size != Some(new_size) {
            self.last_size = Some(new_size);
            self.terminal.update(cx, |terminal, _| {
                terminal.resize(
                    cols,
                    rows,
                    f32::from(bounds.size.width).round() as u16,
                    f32::from(bounds.size.height).round() as u16,
                );
            });
        }
    }

    fn get_line_text(&self, screen_line: usize, cx: &Context<Self>) -> String {
        let term = self.terminal.read(cx).term().lock();
        let grid = term.grid();
        let display_offset = grid.display_offset();
        let grid_line = screen_line as i32 - display_offset as i32;

        if grid_line < -(term.history_size() as i32) || grid_line >= term.screen_lines() as i32 {
            return String::new();
        }

        let line = &grid[Line(grid_line)];
        let text: String = line[..].iter().map(|cell| cell.c).collect();
        text.trim_end_matches(|c: char| c == ' ' || c == '\0')
            .to_string()
    }

    fn render_terminal(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Prepare addons before rendering
        {
            let is_local =
                self.terminal.read(cx).connection_kind() == TerminalConnectionKind::Local;
            let term = self.terminal.read(cx).term().lock();
            let display_offset = term.grid().display_offset();
            let visible_lines = 0..term.screen_lines();
            let context = TerminalAddonFrameContext {
                term: &term,
                visible_lines,
                display_offset,
                is_local,
                base_dir: self.local_working_dir.as_deref(),
            };
            self.addon_manager.dispatch_frame(&context);
        }

        // Update render cache with decorations from all addons
        {
            let term = self.terminal.read(cx).term().clone();
            let mut term = term.lock();

            self.render_cache
                .update(&mut term, &self.addon_manager, &self.current_theme);
            term.reset_damage();
        }

        // 获取光标可见性
        let cursor_visible = if self.cursor_blink_enabled {
            self.blink_manager.read(cx).visible()
        } else {
            true
        };

        TerminalElement::new(
            &self.render_cache,
            self.current_theme.font_family.clone(),
            self.current_theme.font_size,
            self.current_theme
                .font_fallbacks
                .iter()
                .map(|s| s.to_string())
                .collect(),
            self.current_theme.line_height_scale,
            cursor_visible,
            self.cell_width, // 传入预计算的 cell_width，确保与 resize 一致
        )
        .into_element()
    }

    /// 构建终端右键菜单
    fn build_context_menu(
        menu: PopupMenu,
        has_selection: bool,
        selection_text: Option<String>,
        view: &Entity<Self>,
        sidebar: &Entity<TerminalSidebar>,
        _window: &mut Window,
        _cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let view_copy = view.clone();
        let view_paste = view.clone();
        let view_select_all = view.clone();
        let view_clear = view.clone();
        let copy_shortcut = terminal_shortcut_label(TERMINAL_COPY_SHORTCUT);
        let paste_shortcut = terminal_shortcut_label(TERMINAL_PASTE_SHORTCUT);
        let select_all_shortcut = terminal_shortcut_label(TERMINAL_SELECT_ALL_SHORTCUT);

        let mut menu = menu
            // 复制
            .item(
                PopupMenuItem::new(t!(
                    "ContextMenu.copy_with_shortcut",
                    shortcut = copy_shortcut
                ))
                .icon(IconName::Copy)
                .action(Box::new(Copy))
                .disabled(!has_selection)
                .on_click(move |_, window, cx| {
                    let _ = view_copy.update(cx, |this, cx| {
                        this.copy(&Copy, window, cx);
                    });
                }),
            )
            // 粘贴
            .item(
                PopupMenuItem::new(t!(
                    "ContextMenu.paste_with_shortcut",
                    shortcut = paste_shortcut
                ))
                .action(Box::new(Paste))
                .on_click(move |_, window, cx| {
                    let _ = view_paste.update(cx, |this, cx| {
                        this.paste(&Paste, window, cx);
                    });
                }),
            )
            .separator()
            // 全选
            .item(
                PopupMenuItem::new(t!(
                    "ContextMenu.select_all_with_shortcut",
                    shortcut = select_all_shortcut
                ))
                .action(Box::new(SelectAll))
                .on_click(move |_, window, cx| {
                    let _ = view_select_all.update(cx, |this, cx| {
                        this.select_all(&SelectAll, window, cx);
                    });
                }),
            )
            // 清除选择
            .item(
                PopupMenuItem::new(t!("ContextMenu.clear_selection"))
                    .action(Box::new(ClearSelection))
                    .disabled(!has_selection)
                    .on_click(move |_, window, cx| {
                        let _ = view_clear.update(cx, |this, cx| {
                            this.clear_selection(&ClearSelection, window, cx);
                        });
                    }),
            );

        // 询问AI（仅在有选中文本时可用）
        if let Some(text) = selection_text {
            let message = format!(
                "{}",
                t!(
                    "TerminalView.ask_ai_selection_template",
                    content = text.trim()
                )
            );
            let sidebar_clone = sidebar.clone();
            menu = menu.separator().item(
                PopupMenuItem::new(t!("ContextMenu.ask_ai"))
                    .icon(IconName::AI.color())
                    .on_click(move |_, _window, cx| {
                        sidebar_clone.update(cx, |sidebar, cx| {
                            sidebar.ask_ai(message.clone(), cx);
                        });
                    }),
            );

            let save_text = text.trim().to_string();
            let sidebar_quick = sidebar.clone();
            if !save_text.is_empty() {
                menu = menu.item(
                    PopupMenuItem::new(t!("ContextMenu.save_quick_command"))
                        .icon(IconName::SquareTerminal)
                        .on_click(move |_, _window, cx| {
                            sidebar_quick.update(cx, |sidebar, cx| {
                                sidebar.add_quick_command(save_text.clone(), cx);
                            });
                        }),
                );
            }
        }

        menu
    }

    fn render_connection_overlay(
        &self,
        can_reconnect: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let connection_state = self.terminal.read(cx).connection_state().clone();
        let is_connecting = matches!(connection_state, ConnectionState::Connecting);
        let error_msg = match &connection_state {
            ConnectionState::Disconnected { error } => error.clone(),
            _ => None,
        };

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(Hsla {
                h: 0.,
                s: 0.,
                l: 0.,
                a: 0.7,
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .p_6()
                    .bg(rgb(0x2d2d2d))
                    .rounded_lg()
                    .shadow_lg()
                    .max_w(px(400.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(if is_connecting {
                                    IconName::Loader
                                } else {
                                    IconName::CircleX
                                })
                                .color()
                                .with_size(px(24.0))
                                .text_color(if is_connecting {
                                    rgb(0xfbbf24)
                                } else {
                                    rgb(0xef4444)
                                }),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(if is_connecting {
                                        t!("SshSession.connecting")
                                    } else {
                                        t!("SshSession.connection_lost")
                                    }),
                            ),
                    )
                    .when_some(error_msg, |this, msg| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xef4444))
                                .max_w(px(350.0))
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(msg),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x9ca3af))
                            .child(if is_connecting {
                                t!("SshSession.establishing")
                            } else {
                                t!("SshSession.disconnected")
                            }),
                    )
                    .when(can_reconnect && !is_connecting, |this| {
                        this.child(
                            Button::new("reconnect-btn")
                                .label(t!("SshSession.reconnect"))
                                .primary()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.reconnect(window, cx);
                                })),
                        )
                    }),
            )
    }

    fn handle_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta_pixels = event.delta.pixel_delta(self.line_height);
        let delta_lines = delta_pixels.y / self.line_height;
        self.scroll_lines_accumulated += delta_lines;

        let mode = self.terminal.read(cx).mode();
        let lines = take_whole_scroll_lines(&mut self.scroll_lines_accumulated);

        if mode.contains(TermMode::ALT_SCREEN) {
            // ALT_SCREEN（vim、less 等）：累计到整行后再转为上下箭头，避免放大小幅滚轮输入
            if let Some(arrow) = alt_screen_scroll_arrow(lines, mode.contains(TermMode::APP_CURSOR))
            {
                for _ in 0..lines.abs() {
                    self.write_to_pty(arrow.as_bytes().to_vec(), cx);
                }
            }
            return;
        }

        if lines != 0 {
            let term = self.terminal.read(cx).term().clone();

            if mode.contains(TermMode::VI) {
                let mut term = term.lock();
                // 沿用 Alacritty `ViModeCursor::scroll` 的符号语义，直接传入离散后的行数
                let vi_cursor = term.vi_mode_cursor.scroll(&term, lines);
                term.vi_goto_point(vi_cursor.point);

                let display_offset = term.grid().display_offset();
                let cursor_line = vi_cursor.point.line.0;
                let screen_lines = term.screen_lines() as i32;

                if cursor_line < -(display_offset as i32) {
                    let delta = cursor_line + display_offset as i32;
                    term.scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
                } else if cursor_line >= screen_lines - (display_offset as i32) {
                    let delta = cursor_line - screen_lines + 1 + display_offset as i32;
                    term.scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
                }
            } else {
                // 沿用终端 display scroll 的符号语义，直接传入离散后的行数
                term.lock()
                    .scroll_display(alacritty_terminal::grid::Scroll::Delta(lines));
            }
            cx.notify();
        }
    }

    fn pixel_to_point(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        cx: &Context<Self>,
    ) -> AlacPoint {
        let relative_x = position.x - bounds.origin.x;
        let relative_y = position.y - bounds.origin.y;

        let col = (relative_x / self.cell_width).floor().max(0.0) as usize;
        let line = (relative_y / self.line_height).floor().max(0.0) as i32;

        let term = self.terminal.read(cx).term().lock();
        let col = col.min(term.columns().saturating_sub(1));
        let line = line.clamp(0, term.screen_lines() as i32 - 1);
        drop(term);

        AlacPoint::new(Line(line), Column(col))
    }

    /// 根据鼠标在单元格内的位置计算 Side
    fn pixel_to_side(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> Side {
        let relative_x = position.x - bounds.origin.x;
        let col_f = (relative_x / self.cell_width).max(0.0);
        let cell_offset = col_f.fract();
        if cell_offset < 0.5 {
            Side::Left
        } else {
            Side::Right
        }
    }

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

        let bounds = self.terminal_bounds;

        let point = self.pixel_to_point(event.position, bounds, cx);
        let screen_line = point.line.0 as usize;
        let column = point.column.0;
        let line_text = self.get_line_text(screen_line, cx);
        let is_local = self.terminal.read(cx).connection_kind() == TerminalConnectionKind::Local;
        let consumed = {
            let mut open_url = |url: &str| cx.open_url(url);
            let mut context = TerminalAddonMouseContext::new(
                screen_line,
                column,
                &line_text,
                event.modifiers,
                event.position,
                is_local,
                self.local_working_dir.as_deref(),
                &mut open_url,
            );
            self.addon_manager.dispatch_mouse_down(&mut context)
        };

        if consumed {
            return;
        }

        let now = std::time::Instant::now();
        let is_double_click = self.mouse_state.last_click_point == Some(point)
            && self
                .mouse_state
                .last_click_time
                .map_or(false, |t| now.duration_since(t).as_millis() < 500);

        if is_double_click {
            self.mouse_state.click_count += 1;
        } else {
            self.mouse_state.click_count = 1;
        }

        self.mouse_state.last_click_point = Some(point);
        self.mouse_state.last_click_time = Some(now);

        let selection_type = match self.mouse_state.click_count {
            1 => SelectionType::Simple,
            2 => SelectionType::Semantic,
            _ => SelectionType::Lines,
        };

        self.terminal.update(cx, |terminal, _| {
            terminal.start_selection(
                selection_type,
                point,
                self.pixel_to_side(event.position, bounds),
            );
        });

        self.mouse_state.selecting = true;
        cx.notify();
    }

    fn handle_middle_mouse_down(
        &mut self,
        _event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.middle_click_paste {
            return;
        }
        if let Some(clipboard) = cx.read_from_clipboard() {
            if let Some(text) = clipboard.text() {
                self.paste_text(&text, window, cx);
            }
        }
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = self.terminal_bounds;
        self.mouse_position = Some(event.position);
        let point = self.pixel_to_point(event.position, bounds, cx);
        let screen_line = point.line.0 as usize;
        let column = point.column.0;
        let line_text = self.get_line_text(screen_line, cx);
        let is_local = self.terminal.read(cx).connection_kind() == TerminalConnectionKind::Local;
        let hover_changed = {
            let mut open_url = |url: &str| cx.open_url(url);
            let mut context = TerminalAddonMouseContext::new(
                screen_line,
                column,
                &line_text,
                event.modifiers,
                event.position,
                is_local,
                self.local_working_dir.as_deref(),
                &mut open_url,
            );
            self.addon_manager.dispatch_mouse_move(&mut context)
        };
        if hover_changed {
            cx.notify();
        }

        if !self.mouse_state.selecting {
            return;
        }

        let point = self.pixel_to_point(event.position, bounds, cx);
        let side = self.pixel_to_side(event.position, bounds);

        self.terminal.update(cx, |terminal, _| {
            terminal.update_selection(point, side);
        });
        cx.notify();
    }

    fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        let bounds = self.terminal_bounds;
        let point = self.pixel_to_point(event.position, bounds, cx);
        let screen_line = point.line.0 as usize;
        let column = point.column.0;
        let line_text = self.get_line_text(screen_line, cx);
        let is_local = self.terminal.read(cx).connection_kind() == TerminalConnectionKind::Local;
        {
            let mut open_url = |url: &str| cx.open_url(url);
            let mut context = TerminalAddonMouseContext::new(
                screen_line,
                column,
                &line_text,
                event.modifiers,
                event.position,
                is_local,
                self.local_working_dir.as_deref(),
                &mut open_url,
            );
            let _ = self.addon_manager.dispatch_mouse_up(&mut context);
        }
        self.mouse_state.selecting = false;
        if self.auto_copy_on_select {
            if let Some(text) = self.terminal.read(cx).selection_text() {
                if !text.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
        }
        cx.notify();
    }

    fn send_tab(&mut self, _: &SendTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.write_to_pty(b"\x09".to_vec(), cx);
    }

    fn send_shift_tab(&mut self, _: &SendShiftTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.write_to_pty(b"\x1b[Z".to_vec(), cx);
    }

    fn render_sidebar_resize_handle(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        resize_handle::<ResizePanel, ResizePanel>(
            "terminal-sidebar-resize-handle",
            Axis::Horizontal,
        )
        .placement(HandlePlacement::Right)
        .on_drag(ResizePanel, move |info, _, _, cx| {
            cx.stop_propagation();
            view.update(cx, |view, cx| {
                view.resizing = Some(ResizingPanel::Sidebar);
                cx.notify();
            });
            cx.new(|_| info.deref().clone())
        })
    }

    fn resize_sidebar(
        &mut self,
        mouse_position: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(resizing) = self.resizing else {
            return;
        };

        match resizing {
            ResizingPanel::Sidebar => {
                let new_size = self.view_bounds.right() - mouse_position.x;
                self.sidebar_panel_size = new_size.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
            }
        }

        cx.notify();
    }

    fn done_resizing(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.resizing = None;
        cx.notify();
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabContentEvent> for TerminalView {}
impl EventEmitter<TerminalViewEvent> for TerminalView {}

impl TabContent for TerminalView {
    fn content_key(&self) -> &'static str {
        "Terminal"
    }

    fn title(&self, cx: &App) -> SharedString {
        let terminal = self.terminal.read(cx);
        let base_title = if let Some(name) = terminal.connection_name() {
            name.to_string()
        } else if !terminal.title().is_empty() {
            terminal.title().to_string()
        } else {
            "Terminal".to_string()
        };

        // 如果有序号，添加到标题后
        if let Some(index) = self.tab_index {
            SharedString::from(format!("{}({})", base_title, index))
        } else {
            SharedString::from(base_title)
        }
    }

    fn icon(&self, cx: &App) -> Option<Icon> {
        if self.connection_kind(cx) == TerminalConnectionKind::Serial {
            Some(IconName::SerialPort.color())
        } else {
            Some(IconName::TerminalColor.color())
        }
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }

    fn try_close(
        &mut self,
        _tab_id: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        // tab 会立即从容器中移除，先同步回收活跃状态，避免主页残留“连接使用中”标记。
        self.release_active_connection(cx);
        // 关闭终端连接
        self.terminal.read(cx).shutdown();
        Task::ready(true)
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 创建与 terminal_element 一致的字体配置（包含 fallbacks）
        let fallbacks = if self.current_theme.font_fallbacks.is_empty() {
            None
        } else {
            Some(FontFallbacks::from_fonts(
                self.current_theme
                    .font_fallbacks
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            ))
        };
        let features = FontFeatures(std::sync::Arc::new(vec![("calt".to_string(), 0)]));

        let font = Font {
            family: self.current_theme.font_family.clone(),
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            features,
            fallbacks,
        };
        let font_id = window.text_system().resolve_font(&font);
        // 使用 advance('m').width 计算 cell_width
        // advance 返回字符的前进宽度，比 em_width 更准确反映等宽字体的单元格宽度
        let new_cell_width = window
            .text_system()
            .advance(font_id, self.current_theme.font_size, 'm')
            .map(|size| size.width)
            .unwrap_or(self.current_theme.font_size * 0.6);

        if self.cell_width != new_cell_width {
            self.cell_width = new_cell_width;
        }

        self.line_height = self.current_theme.font_size * self.current_theme.line_height_scale;
        self.font_size = self.current_theme.font_size;

        if let Some(new_display_offset) = self.scrollbar_handle.take_future_display_offset() {
            self.terminal.update(cx, |terminal, _| {
                let current = terminal.term().lock().grid().display_offset() as i32;
                let target = new_display_offset as i32;
                let delta = target - current;
                if delta != 0 {
                    terminal.scroll(delta);
                }
            });
        }

        let connection_state = self.terminal.read(cx).connection_state().clone();
        let can_reconnect = self.terminal.read(cx).can_reconnect();
        let bg_color = self.current_theme.background;
        let has_selection = self.terminal.read(cx).term().lock().selection.is_some();
        let selection_text = self.terminal.read(cx).selection_text();
        let sidebar_visible = self.sidebar.read(cx).is_visible();
        let sidebar_panel_size = self.sidebar_panel_size;
        let view = cx.entity().clone();
        let terminal_mode = self.terminal.read(cx).mode();
        let history_size = self.terminal.read(cx).term().lock().history_size();
        let show_scrollbar = !terminal_mode.contains(TermMode::ALT_SCREEN) && history_size > 0;

        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(bg_color)
            .child({
                let tooltip = self.addon_manager.tooltip();
                let mouse_pos = self.mouse_position;
                let terminal_bounds = self.terminal_bounds;
                let entity = cx.entity().downgrade();
                let focus_handle = self.focus_handle.clone();
                let terminal_core = div()
                    .track_focus(&focus_handle)
                    .key_context(TERMINAL_CONTEXT)
                    .on_action(cx.listener(Self::send_tab))
                    .on_action(cx.listener(Self::send_shift_tab))
                    .on_action(cx.listener(Self::copy))
                    .on_action(cx.listener(Self::paste))
                    .on_action(cx.listener(Self::select_all))
                    .on_action(cx.listener(Self::clear_selection))
                    .on_action(cx.listener(Self::search_forward))
                    .on_action(cx.listener(Self::search_backward))
                    .on_action(cx.listener(Self::toggle_vi_mode))
                    .on_action(cx.listener(Self::increase_font))
                    .on_action(cx.listener(Self::decrease_font))
                    .on_action(cx.listener(Self::reset_font))
                    .on_key_down(cx.listener(Self::handle_key_event))
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(Self::handle_scroll))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(Self::handle_middle_mouse_down),
                    )
                    .on_mouse_move(cx.listener(Self::handle_mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
                    .child(
                        canvas(
                            move |bounds, _window, cx| {
                                if let Some(entity) = entity.upgrade() {
                                    entity.update(cx, |this, cx| {
                                        this.terminal_bounds = bounds;
                                        {
                                            let mut metrics = this.scrollbar_metrics.borrow_mut();
                                            metrics.viewport_size = bounds.size;
                                            metrics.line_height = this.line_height;
                                            metrics.cell_width = this.cell_width;
                                        }
                                        this.resize_if_needed(bounds, cx);
                                    });
                                }
                            },
                            {
                                let entity = cx.entity().downgrade();
                                let focus_handle = focus_handle.clone();
                                move |bounds, _state, window, cx| {
                                    if let Some(entity) = entity.upgrade() {
                                        let input_handler =
                                            ElementInputHandler::new(bounds, entity);
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
                        let view = cx.entity().clone();
                        let sidebar = self.sidebar.clone();
                        div()
                            .absolute()
                            .left(px(12.))
                            .right(px(12.))
                            .top(px(12.))
                            .bottom(px(12.))
                            .bg(self.current_theme.background)
                            .overflow_hidden()
                            .child(self.render_terminal(cx))
                            .context_menu(move |menu, window, cx| {
                                Self::build_context_menu(
                                    menu,
                                    has_selection,
                                    selection_text.clone(),
                                    &view,
                                    &sidebar,
                                    window,
                                    cx,
                                )
                            })
                    })
                    .when_some(tooltip.zip(mouse_pos), |this, (tooltip, pos)| {
                        let relative_x = pos.x - terminal_bounds.origin.x;
                        let relative_y = pos.y - terminal_bounds.origin.y;
                        this.child(
                            div()
                                .absolute()
                                .left(relative_x + px(10.0))
                                .top(relative_y + px(20.0))
                                .px_2()
                                .py_1()
                                .bg(rgb(0x3d3d3d))
                                .rounded_md()
                                .shadow_md()
                                .text_size(px(11.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .px_1()
                                                .bg(rgb(0x4d4d4d))
                                                .rounded_sm()
                                                .text_color(rgb(0xcccccc))
                                                .child(tooltip.action_hint),
                                        )
                                        .child(
                                            div()
                                                .text_color(rgb(0x888888))
                                                .child(tooltip.action_text),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_color(tooltip.display_color)
                                        .overflow_hidden()
                                        .max_w(px(400.0))
                                        .text_ellipsis()
                                        .child(tooltip.display_text),
                                ),
                        )
                    })
                    .when(
                        matches!(connection_state, ConnectionState::Disconnected { .. })
                            || matches!(connection_state, ConnectionState::Connecting),
                        |this| this.child(self.render_connection_overlay(can_reconnect, cx)),
                    );

                div()
                    .relative()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(terminal_core)
                    .when(show_scrollbar, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top(px(12.0))
                                .right(px(4.0))
                                .bottom(px(12.0))
                                .w(px(12.0))
                                .child(
                                    Scrollbar::vertical(&self.scrollbar_handle)
                                        .scrollbar_show(ScrollbarShow::Always),
                                ),
                        )
                    })
            })
            // 渲染侧边栏
            .when(sidebar_visible, |this| {
                this.child(
                    div()
                        .relative()
                        .h_full()
                        .w(sidebar_panel_size)
                        .flex_shrink_0()
                        .child(self.render_sidebar_resize_handle(window, cx))
                        .child(self.sidebar.clone()),
                )
            })
            .when(!sidebar_visible, |this| this.child(self.sidebar.clone()))
            .child(ResizeEventHandler { view })
    }
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        _range: std::ops::Range<usize>,
        _actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.marked_text_range()
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.clear_marked_text(cx);
    }

    fn replace_text_in_range(
        &mut self,
        _replacement_range: Option<std::ops::Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_marked_text(cx);
        self.commit_text(text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_marked_range: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_marked_text(new_text.to_string(), new_marked_range, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range: std::ops::Range<usize>,
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // 获取光标位置用于 IME 定位
        let term = self.terminal.read(cx).term().lock();
        let cursor = term.grid().cursor.point;
        let display_offset = term.grid().display_offset();
        drop(term);

        let screen_line = cursor.line.0 + display_offset as i32;
        let col = cursor.column.0;

        // 计算像素位置
        let origin = Point::new(
            self.terminal_bounds.origin.x + self.cell_width * col as f32,
            self.terminal_bounds.origin.y + self.line_height * screen_line as f32,
        );

        Some(Bounds::new(origin, size(self.cell_width, self.line_height)))
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

struct ResizeEventHandler {
    view: Entity<TerminalView>,
}

impl IntoElement for ResizeEventHandler {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ResizeEventHandler {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let bounds = window.bounds();
        self.view.update(cx, |view, _| {
            view.view_bounds = Bounds {
                origin: Point::default(),
                size: bounds.size,
            };
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.on_mouse_event({
            let view = self.view.clone();
            let resizing = view.read(cx).resizing;
            move |e: &MouseMoveEvent, phase, window, cx| {
                if resizing.is_none() {
                    return;
                }
                if !phase.bubble() {
                    return;
                }
                view.update(cx, |view, cx| view.resize_sidebar(e.position, window, cx));
            }
        });

        window.on_mouse_event({
            let view = self.view.clone();
            move |_: &MouseUpEvent, phase, window, cx| {
                if phase.bubble() {
                    view.update(cx, |view, cx| view.done_resizing(window, cx));
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        alt_screen_scroll_arrow, detect_unbracketed_paste_hazard, has_trailing_line_continuation,
        has_unterminated_shell_quote, multiline_non_empty_line_count,
        should_scroll_to_bottom_on_user_input, take_whole_scroll_lines, UnbracketedPasteHazard,
    };
    use std::cell::Cell as StdCell;

    #[test]
    fn take_whole_scroll_lines_preserves_fractional_remainder() {
        let mut accumulated = 0.4;
        assert_eq!(take_whole_scroll_lines(&mut accumulated), 0);
        assert!((accumulated - 0.4).abs() < f32::EPSILON);

        accumulated += 0.8;
        assert_eq!(take_whole_scroll_lines(&mut accumulated), 1);
        assert!((accumulated - 0.2).abs() < 0.0001);
    }

    #[test]
    fn take_whole_scroll_lines_handles_negative_accumulation() {
        let mut accumulated = -0.45;
        assert_eq!(take_whole_scroll_lines(&mut accumulated), 0);
        assert!((accumulated + 0.45).abs() < f32::EPSILON);

        accumulated -= 0.8;
        assert_eq!(take_whole_scroll_lines(&mut accumulated), -1);
        assert!((accumulated + 0.25).abs() < 0.0001);
    }

    #[test]
    fn alt_screen_scroll_arrow_maps_positive_lines_to_up() {
        assert_eq!(alt_screen_scroll_arrow(1, false), Some("\x1b[A"));
        assert_eq!(alt_screen_scroll_arrow(1, true), Some("\x1bOA"));
    }

    #[test]
    fn alt_screen_scroll_arrow_maps_negative_lines_to_down() {
        assert_eq!(alt_screen_scroll_arrow(-1, false), Some("\x1b[B"));
        assert_eq!(alt_screen_scroll_arrow(-1, true), Some("\x1bOB"));
        assert_eq!(alt_screen_scroll_arrow(0, false), None);
    }

    #[test]
    fn multiline_non_empty_line_count_ignores_blank_lines() {
        assert_eq!(multiline_non_empty_line_count("echo 1\n\n echo 2\n"), 2);
        assert_eq!(multiline_non_empty_line_count("echo 1"), 1);
    }

    #[test]
    fn detect_unbracketed_paste_hazard_matches_heredoc() {
        let text = "cat <<EOF\nhello\nEOF";
        assert_eq!(
            detect_unbracketed_paste_hazard(text),
            Some(UnbracketedPasteHazard::HereDoc)
        );
    }

    #[test]
    fn detect_unbracketed_paste_hazard_matches_line_continuation() {
        assert!(has_trailing_line_continuation("echo hello \\\nworld"));
        assert_eq!(
            detect_unbracketed_paste_hazard("echo hello \\\nworld"),
            Some(UnbracketedPasteHazard::LineContinuation)
        );
    }

    #[test]
    fn detect_unbracketed_paste_hazard_matches_unterminated_quote() {
        assert!(has_unterminated_shell_quote("printf 'hello\nworld"));
        assert_eq!(
            detect_unbracketed_paste_hazard("printf 'hello\nworld"),
            Some(UnbracketedPasteHazard::UnterminatedQuote)
        );
    }

    #[test]
    fn detect_unbracketed_paste_hazard_ignores_plain_text() {
        assert_eq!(
            detect_unbracketed_paste_hazard("printf '%s\\n' hello"),
            None
        );
        assert!(!has_unterminated_shell_quote("printf '%s\\n' hello"));
        assert!(!has_trailing_line_continuation("echo hello\necho world"));
    }

    #[test]
    fn user_input_scroll_clears_pending_offset_even_when_already_at_bottom() {
        let pending_display_offset = StdCell::new(Some(12));

        assert!(!should_scroll_to_bottom_on_user_input(
            0,
            &pending_display_offset
        ));
        assert_eq!(pending_display_offset.take(), None);
    }

    #[test]
    fn user_input_scroll_requests_bottom_when_terminal_is_scrolled_up() {
        let pending_display_offset = StdCell::new(Some(12));

        assert!(should_scroll_to_bottom_on_user_input(
            5,
            &pending_display_offset
        ));
        assert_eq!(pending_display_offset.take(), None);
    }
}

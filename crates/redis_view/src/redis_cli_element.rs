//! Redis CLI 自定义渲染元素
//!
//! 使用 GPUI Element trait 实现高性能的 CLI 输出渲染，
//! 参考 terminal_view/terminal_element.rs 的实现模式。

use gpui::*;
use std::sync::Arc;

/// 文本位置（行号和字符索引）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPosition {
    /// 行号（从 0 开始）
    pub line: usize,
    /// 字符索引（从 0 开始，以字符为单位而非字节）
    pub column: usize,
}

impl TextPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// 比较两个位置，返回较早的位置
    pub fn min(self, other: Self) -> Self {
        if self.line < other.line || (self.line == other.line && self.column <= other.column) {
            self
        } else {
            other
        }
    }

    /// 比较两个位置，返回较晚的位置
    pub fn max(self, other: Self) -> Self {
        if self.line > other.line || (self.line == other.line && self.column >= other.column) {
            self
        } else {
            other
        }
    }
}

impl PartialOrd for TextPosition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TextPosition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => self.column.cmp(&other.column),
            other => other,
        }
    }
}

/// 选择类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionType {
    /// 普通字符选择
    Simple,
    /// 单词选择（双击）
    Word,
    /// 整行选择（三击）
    Line,
}

/// 文本选择范围
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    /// 选择锚点（开始位置）
    pub anchor: TextPosition,
    /// 选择活动端（当前位置）
    pub active: TextPosition,
    /// 选择类型
    pub selection_type: SelectionType,
}

impl TextSelection {
    pub fn new(anchor: TextPosition, active: TextPosition, selection_type: SelectionType) -> Self {
        Self {
            anchor,
            active,
            selection_type,
        }
    }

    /// 创建单点选择（光标位置）
    pub fn point(pos: TextPosition) -> Self {
        Self {
            anchor: pos,
            active: pos,
            selection_type: SelectionType::Simple,
        }
    }

    /// 获取规范化的选择范围（start <= end）
    pub fn normalized(&self) -> (TextPosition, TextPosition) {
        if self.anchor <= self.active {
            (self.anchor, self.active)
        } else {
            (self.active, self.anchor)
        }
    }

    /// 选择是否为空（单点）
    pub fn is_empty(&self) -> bool {
        self.anchor == self.active
    }

    /// 检查指定位置是否在选择范围内
    pub fn contains(&self, pos: TextPosition) -> bool {
        let (start, end) = self.normalized();
        pos >= start && pos < end
    }

    /// 检查指定行是否在选择范围内
    pub fn intersects_line(&self, line: usize) -> bool {
        let (start, end) = self.normalized();
        line >= start.line && line <= end.line
    }

    /// 获取指定行的选中列范围
    /// 返回 (start_col, end_col)，如果该行不在选择范围内返回 None
    pub fn get_line_selection(&self, line: usize, line_len: usize) -> Option<(usize, usize)> {
        let (start, end) = self.normalized();

        if line < start.line || line > end.line {
            return None;
        }

        let start_col = if line == start.line { start.column } else { 0 };
        let end_col = if line == end.line { end.column } else { line_len };

        let start_col = start_col.min(line_len);
        let end_col = end_col.min(line_len);

        if start_col >= end_col {
            return None;
        }

        Some((start_col, end_col))
    }
}

/// CLI 行类型
#[derive(Clone, Debug)]
pub enum CliLineType {
    /// 命令提示符行（带输入）
    Prompt { command: String },
    /// 命令结果行
    Result { text: String, is_error: bool },
    /// 命令提示行
    Hint { text: String },
    /// 欢迎信息行
    Welcome { text: String },
    /// 当前输入行（带光标）
    Input { prompt: String, text: String, cursor_pos: usize, hint: Option<String> },
}

/// CLI 渲染行
#[derive(Clone, Debug)]
pub struct CliLine {
    /// 行类型
    pub line_type: CliLineType,
}

/// CLI 渲染主题
#[derive(Clone)]
pub struct CliTheme {
    /// 背景色
    pub background: Hsla,
    /// 前景色（普通文本）
    pub foreground: Hsla,
    /// 提示文本颜色
    pub hint: Hsla,
    /// 命令提示符颜色
    pub prompt: Hsla,
    /// 命令文本颜色
    pub command: Hsla,
    /// 成功结果颜色
    pub success: Hsla,
    /// 错误结果颜色
    pub error: Hsla,
    /// 光标颜色
    pub cursor: Hsla,
    /// 选择背景色
    pub selection_background: Hsla,
    /// 选择前景色
    pub selection_foreground: Hsla,
    /// 字体族
    pub font_family: SharedString,
    /// 字体大小
    pub font_size: Pixels,
    /// 备用字体列表
    pub font_fallbacks: Vec<SharedString>,
    /// 行高比例
    pub line_height_scale: f32,
}

impl Default for CliTheme {
    fn default() -> Self {
        Self {
            background: rgb(0x1E1E1E).into(),
            foreground: rgb(0xE4E4E4).into(),
            hint: rgb(0x9CA3AF).into(),
            prompt: rgb(0xDCDCAA).into(),
            command: rgb(0xE4E4E4).into(),
            success: rgb(0x98C379).into(),
            error: rgb(0xF44747).into(),
            cursor: rgb(0xFFFFFF).into(),
            selection_background: hsla(0.58, 0.5, 0.4, 1.0),
            selection_foreground: hsla(0.0, 0.0, 1.0, 1.0),
            font_family: default_monospace_font().into(),
            font_size: px(13.0),
            font_fallbacks: default_font_fallbacks(),
            line_height_scale: 1.4,
        }
    }
}

fn default_monospace_font() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "DejaVu Sans Mono"
    }
}

fn default_font_fallbacks() -> Vec<SharedString> {
    if cfg!(target_os = "macos") {
        vec![
            "Monaco".into(),
            "SF Mono".into(),
            "Courier New".into(),
            "Apple Color Emoji".into(),
            "Apple Symbols".into(),
            "PingFang SC".into(),
            "PingFang TC".into(),
            "Hiragino Sans GB".into(),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            "Cascadia Mono".into(),
            "Courier New".into(),
            "Lucida Console".into(),
            "Segoe UI Emoji".into(),
            "Microsoft YaHei".into(),
            "SimSun".into(),
        ]
    } else {
        vec![
            "Ubuntu Mono".into(),
            "Liberation Mono".into(),
            "Courier New".into(),
            "Noto Color Emoji".into(),
            "Noto Sans CJK SC".into(),
            "WenQuanYi Micro Hei".into(),
        ]
    }
}

/// 预计算的字体变体
#[derive(Clone)]
pub struct CliFont {
    pub normal: Font,
    pub bold: Font,
}

impl CliFont {
    pub fn new(family: SharedString, fallbacks: Vec<SharedString>) -> Self {
        let features = FontFeatures(Arc::new(vec![("calt".to_string(), 0)]));
        let fallbacks = if fallbacks.is_empty() {
            None
        } else {
            Some(FontFallbacks::from_fonts(
                fallbacks
                    .iter()
                    .map(|fallback| fallback.to_string())
                    .collect::<Vec<_>>(),
            ))
        };

        Self {
            normal: Font {
                family: family.clone(),
                weight: FontWeight::NORMAL,
                style: FontStyle::Normal,
                features: features.clone(),
                fallbacks: fallbacks.clone(),
            },
            bold: Font {
                family,
                weight: FontWeight::BOLD,
                style: FontStyle::Normal,
                features,
                fallbacks,
            },
        }
    }
}

/// 布局状态
pub struct CliLayout {
    line_height: Pixels,
    char_width: Pixels,
    fonts: CliFont,
}

/// Redis CLI 渲染元素
pub struct RedisCliElement {
    lines: Vec<CliLine>,
    text_lines: Vec<String>,
    theme: CliTheme,
    cursor_visible: bool,
    scroll_offset: f32,
    selection: Option<TextSelection>,
    cell_width: Pixels,
}

impl RedisCliElement {
    pub fn new(
        lines: Vec<CliLine>,
        text_lines: Vec<String>,
        theme: CliTheme,
        cursor_visible: bool,
        scroll_offset: f32,
        selection: Option<TextSelection>,
        cell_width: Pixels,
    ) -> Self {
        Self {
            lines,
            text_lines,
            theme,
            cursor_visible,
            scroll_offset,
            selection,
            cell_width,
        }
    }
}

impl IntoElement for RedisCliElement {
    type Element = RedisCliElementImpl;

    fn into_element(self) -> Self::Element {
        RedisCliElementImpl {
            lines: self.lines,
            text_lines: self.text_lines,
            theme: self.theme,
            cursor_visible: self.cursor_visible,
            scroll_offset: self.scroll_offset,
            selection: self.selection,
            cell_width: self.cell_width,
        }
    }
}

/// CLI 元素实现
pub struct RedisCliElementImpl {
    lines: Vec<CliLine>,
    text_lines: Vec<String>,
    theme: CliTheme,
    cursor_visible: bool,
    scroll_offset: f32,
    selection: Option<TextSelection>,
    cell_width: Pixels,
}

impl IntoElement for RedisCliElementImpl {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RedisCliElementImpl {
    type RequestLayoutState = ();
    type PrepaintState = CliLayout;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            // 使用绝对定位填满父容器，确保获得完整的绘制区域
            position: Position::Absolute,
            inset: Edges {
                top: px(0.0).into(),
                right: px(0.0).into(),
                bottom: px(0.0).into(),
                left: px(0.0).into(),
            },
            ..Default::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let fonts = CliFont::new(
            self.theme.font_family.clone(),
            self.theme.font_fallbacks.clone(),
        );
        let line_height = self.theme.font_size * self.theme.line_height_scale;

        // 使用由 view 传入的 cell_width，确保与坐标计算一致
        CliLayout {
            line_height,
            char_width: self.cell_width,
            fonts,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let line_height = prepaint.line_height;
        let char_width = prepaint.char_width;
        let fonts = prepaint.fonts.clone();

        // 绘制背景
        window.paint_quad(fill(bounds, self.theme.background));

        // 应用滚动偏移
        let scroll_offset_px = line_height * self.scroll_offset;
        let mut y = bounds.origin.y - scroll_offset_px;
        let x_base = bounds.origin.x + px(12.0);

        for (line_idx, line) in self.lines.iter().enumerate() {
            // 检查是否在可见区域
            let in_viewport = y + line_height >= bounds.origin.y
                && y <= bounds.origin.y + bounds.size.height;

            if in_viewport {
                // 获取该行的选择范围
                let line_text = self.text_lines.get(line_idx).cloned().unwrap_or_default();
                let line_cell_len = cell_len(&line_text);
                let line_selection = self.selection.as_ref().and_then(|s| {
                    s.get_line_selection(line_idx, line_cell_len)
                });

                // 绘制选择背景
                if let Some((sel_start, sel_end)) = line_selection {
                    let sel_x = x_base + char_width * sel_start as f32;
                    let sel_w = char_width * sel_end.saturating_sub(sel_start) as f32;
                    let sel_rect = Bounds::new(
                        Point::new(sel_x, y),
                        size(sel_w, line_height),
                    );
                    window.paint_quad(fill(sel_rect, self.theme.selection_background));
                }

                match &line.line_type {
                    CliLineType::Welcome { text } => {
                        self.paint_line_text(
                            text,
                            x_base,
                            y,
                            self.theme.foreground,
                            &fonts.bold,
                            char_width,
                            line_height,
                            line_selection,
                            window,
                            cx,
                        );
                    }
                    CliLineType::Prompt { command } => {
                        // 绘制完整提示符 + 命令
                        let prompt_text = format!("{}", self.get_prompt_from_line(&line_text));
                        let prompt_cell_len = cell_len(&prompt_text);
                        let command_text = format!(" {}", command);
                        let command_cell_len = cell_len(&command_text);

                        // 绘制提示符部分
                        self.paint_line_text(
                            &prompt_text,
                            x_base,
                            y,
                            self.theme.prompt,
                            &fonts.bold,
                            char_width,
                            line_height,
                            line_selection.map(|sel| self.clip_selection(sel, 0, prompt_cell_len)),
                            window,
                            cx,
                        );

                        // 绘制命令部分
                        let cmd_x = x_base + char_width * (prompt_cell_len as f32);
                        let cmd_offset = prompt_cell_len;
                        self.paint_line_text(
                            &command_text,
                            cmd_x,
                            y,
                            self.theme.command,
                            &fonts.normal,
                            char_width,
                            line_height,
                            line_selection.map(|sel| self.clip_selection(sel, cmd_offset, cmd_offset + command_cell_len)),
                            window,
                            cx,
                        );
                    }
                    CliLineType::Result { text, is_error } => {
                        let color = if *is_error {
                            self.theme.error
                        } else {
                            self.theme.success
                        };

                        self.paint_line_text(
                            text,
                            x_base,
                            y,
                            color,
                            &fonts.normal,
                            char_width,
                            line_height,
                            line_selection,
                            window,
                            cx,
                        );
                    }
                    CliLineType::Hint { text } => {
                        self.paint_line_text(
                            text,
                            x_base,
                            y,
                            self.theme.hint,
                            &fonts.normal,
                            char_width,
                            line_height,
                            line_selection,
                            window,
                            cx,
                        );
                    }
                    CliLineType::Input { prompt, text, cursor_pos, hint } => {
                        let prompt_cell_len = cell_len(prompt);
                        let text_cell_len = cell_len(text);

                        // 绘制提示符
                        self.paint_line_text(
                            prompt,
                            x_base,
                            y,
                            self.theme.prompt,
                            &fonts.bold,
                            char_width,
                            line_height,
                            line_selection.map(|sel| self.clip_selection(sel, 0, prompt_cell_len)),
                            window,
                            cx,
                        );

                        // 空格 + 输入文本
                        let text_x = x_base + char_width * (prompt_cell_len as f32 + 1.0);
                        let text_offset = prompt_cell_len + 1; // 提示符 + 空格

                        if !text.is_empty() {
                            self.paint_line_text(
                                text,
                                text_x,
                                y,
                                self.theme.command,
                                &fonts.normal,
                                char_width,
                                line_height,
                                line_selection.map(|sel| self.clip_selection(sel, text_offset, text_offset + text_cell_len)),
                                window,
                                cx,
                            );
                        }

                        if let Some(hint_text) = hint {
                            if !hint_text.is_empty() {
                                let hint_x = text_x + char_width * text_cell_len as f32;
                                self.paint_line_text(
                                    hint_text,
                                    hint_x,
                                    y,
                                    self.theme.hint,
                                    &fonts.normal,
                                    char_width,
                                    line_height,
                                    None,
                                    window,
                                    cx,
                                );
                            }
                        }

                        // 绘制光标
                        if self.cursor_visible {
                            let cursor_cell = cell_column_for_char_index(text, *cursor_pos);
                            let cursor_x = text_x + char_width * cursor_cell as f32;
                            let cursor_rect = Bounds::new(
                                Point::new(cursor_x, y),
                                size(px(2.0), line_height),
                            );
                            window.paint_quad(fill(cursor_rect, self.theme.cursor));
                        }
                    }
                }
            }
            y += line_height;
        }
    }
}

impl RedisCliElementImpl {
    /// 从行文本中提取提示符部分
    fn get_prompt_from_line(&self, line_text: &str) -> String {
        // 提示符格式: "redis:dbN>"
        if let Some(idx) = line_text.find('>') {
            line_text[..=idx].to_string()
        } else {
            String::new()
        }
    }

    /// 裁剪选择范围到指定字符偏移区间
    fn clip_selection(&self, sel: (usize, usize), offset: usize, end: usize) -> (usize, usize) {
        let start = sel.0.saturating_sub(offset).min(end.saturating_sub(offset));
        let end_col = sel.1.saturating_sub(offset).min(end.saturating_sub(offset));
        (start, end_col)
    }

    /// 绘制行文本（支持选择高亮）
    fn paint_line_text(
        &self,
        text: &str,
        x: Pixels,
        y: Pixels,
        default_color: Hsla,
        font: &Font,
        char_width: Pixels,
        line_height: Pixels,
        selection: Option<(usize, usize)>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if text.is_empty() {
            return;
        }

        let display_text = expand_text_for_cells(text);

        // 如果有选择，拆分成多段渲染
        if let Some((sel_start, sel_end)) = selection {
            if sel_start < sel_end && sel_start < display_text.chars().count() {
                let chars: Vec<char> = display_text.chars().collect();
                let sel_end = sel_end.min(chars.len());

                // 选择前的部分
                if sel_start > 0 {
                    let before: String = chars[..sel_start].iter().collect();
                    paint_text_with_cell_width(
                        &before,
                        Point::new(x, y),
                        default_color,
                        font,
                        self.theme.font_size,
                        line_height,
                        char_width,
                        window,
                        cx,
                    );
                }

                // 选中部分（使用选择前景色）
                let selected: String = chars[sel_start..sel_end].iter().collect();
                let sel_x = x + char_width * sel_start as f32;
                paint_text_with_cell_width(
                    &selected,
                    Point::new(sel_x, y),
                    self.theme.selection_foreground,
                    font,
                    self.theme.font_size,
                    line_height,
                    char_width,
                    window,
                    cx,
                );

                // 选择后的部分
                if sel_end < chars.len() {
                    let after: String = chars[sel_end..].iter().collect();
                    let after_x = x + char_width * sel_end as f32;
                    paint_text_with_cell_width(
                        &after,
                        Point::new(after_x, y),
                        default_color,
                        font,
                        self.theme.font_size,
                        line_height,
                        char_width,
                        window,
                        cx,
                    );
                }
                return;
            }
        }

        // 无选择，正常渲染
        paint_text_with_cell_width(
            &display_text,
            Point::new(x, y),
            default_color,
            font,
            self.theme.font_size,
            line_height,
            char_width,
            window,
            cx,
        );
    }
}

/// 绘制文本（带 cell_width 等宽渲染，与 terminal_view 保持一致）
fn paint_text_with_cell_width(
    text: &str,
    origin: Point<Pixels>,
    color: Hsla,
    font: &Font,
    font_size: Pixels,
    line_height: Pixels,
    cell_width: Pixels,
    window: &mut Window,
    cx: &mut App,
) {
    if text.is_empty() {
        return;
    }

    let text_owned: SharedString = text.to_string().into();

    let shaped = window.text_system().shape_line(
        text_owned,
        font_size,
        &[TextRun {
            len: text.len(),
            font: font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        Some(cell_width),  // 使用 cell_width 确保等宽渲染
    );

    let _ = shaped.paint(origin, line_height, TextAlign::Left, None, window, cx);
}


pub fn cell_len(text: &str) -> usize {
    text.chars().map(cell_width_for_char).sum()
}

pub fn cell_column_for_char_index(text: &str, char_index: usize) -> usize {
    text.chars()
        .take(char_index)
        .map(cell_width_for_char)
        .sum()
}

pub fn char_index_for_cell_column(text: &str, column: usize) -> usize {
    let mut col = 0;
    let mut index = 0;

    for ch in text.chars() {
        let width = cell_width_for_char(ch);
        if col + width > column {
            break;
        }
        col += width;
        index += 1;
    }

    index
}

pub fn expand_text_for_cells(text: &str) -> String {
    let mut expanded = String::with_capacity(text.len());
    for ch in text.chars() {
        expanded.push(ch);
        if cell_width_for_char(ch) == 2 {
            expanded.push(' ');
        }
    }
    expanded
}

fn cell_width_for_char(ch: char) -> usize {
    if is_wide_char(ch) {
        2
    } else {
        1
    }
}

fn is_wide_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x3FFFD
    )
}

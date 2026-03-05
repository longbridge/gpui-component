//! High-performance terminal element with incremental rendering
//!
//! Features:
//! - Damage-based incremental updates
//! - Cell batching for backgrounds and text
//! - Selection and search highlighting
//! - Theme colors support

use crate::addon::{AddonManager, CellDecoration, DecorationSpan};
use crate::TerminalTheme;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::selection::SelectionRange;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::{RenderableContent, Term, TermDamage};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};
use gpui::*;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use terminal::pty_backend::GpuiEventProxy;

/// 预缓存的字体变体，避免每帧重复创建 Font 对象
#[derive(Clone)]
pub struct FontVariants {
    pub normal: Font,
    pub bold: Font,
    pub italic: Font,
    pub bold_italic: Font,
}

impl FontVariants {
    pub fn new(family: SharedString, fallbacks: Vec<String>) -> Self {
        // 与 view.rs 保持一致：当 fallbacks 为空时使用 None
        let fallbacks = if fallbacks.is_empty() {
            None
        } else {
            Some(FontFallbacks::from_fonts(fallbacks))
        };

        // 只禁用 calt（上下文替代），避免等宽字符出现连字影响栅格对齐
        let features = FontFeatures(Arc::new(vec![("calt".to_string(), 0)]));

        Self {
            normal: Font {
                family: family.clone(),
                weight: FontWeight::NORMAL,
                style: FontStyle::Normal,
                features: features.clone(),
                fallbacks: fallbacks.clone(),
            },
            bold: Font {
                family: family.clone(),
                weight: FontWeight::BOLD,
                style: FontStyle::Normal,
                features: features.clone(),
                fallbacks: fallbacks.clone(),
            },
            italic: Font {
                family: family.clone(),
                weight: FontWeight::NORMAL,
                style: FontStyle::Italic,
                features: features.clone(),
                fallbacks: fallbacks.clone(),
            },
            bold_italic: Font {
                family,
                weight: FontWeight::BOLD,
                style: FontStyle::Italic,
                features,
                fallbacks,
            },
        }
    }

    #[inline]
    pub fn get(&self, bold: bool, italic: bool) -> &Font {
        match (bold, italic) {
            (false, false) => &self.normal,
            (true, false) => &self.bold,
            (false, true) => &self.italic,
            (true, true) => &self.bold_italic,
        }
    }
}

/// 检查是否为装饰字符（边框、块元素、Powerline 等）
/// 装饰字符保持原始颜色，不应用自定义前景色
#[inline]
fn is_decorative_character(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x2500..=0x257F     // Box Drawing: ─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼
        | 0x2580..=0x259F   // Block Elements: ▀ ▄ █ ░ ▒ ▓
        | 0x25A0..=0x25FF   // Geometric Shapes: ■ □ ▪ ▫ ● ○
        | 0xE0B0..=0xE0D7   // Powerline symbols
        | 0x2800..=0x28FF   // Braille Patterns
    )
}

/// Manages decorations from all addons
pub struct DecorationManager {
    // Decorations indexed by line number
    decorations_by_line: HashMap<usize, Vec<DecorationSpan>>,
}

impl DecorationManager {
    pub fn new() -> Self {
        Self {
            decorations_by_line: HashMap::new(),
        }
    }

    /// Collect decorations from all addons
    pub fn collect_from_addons(
        &mut self,
        addon_manager: &AddonManager,
        visible_lines: Range<usize>,
        display_offset: usize,
    ) {
        self.decorations_by_line.clear();

        // Collect decorations from each addon
        for addon in addon_manager.iter_addons() {
            let decorations = addon.provide_decorations(visible_lines.clone(), display_offset);

            for deco in decorations {
                self.decorations_by_line
                    .entry(deco.line)
                    .or_insert_with(Vec::new)
                    .push(deco);
            }
        }

        // Sort decorations by priority (ascending, so lower priority first)
        for decorations in self.decorations_by_line.values_mut() {
            decorations.sort_by_key(|d| d.decoration.priority());
        }
    }

    /// Get all decorations for a specific cell
    pub fn get_decorations_for_cell(
        &self,
        line: usize,
        col: usize,
    ) -> impl Iterator<Item = &CellDecoration> + '_ {
        self.decorations_by_line
            .get(&line)
            .into_iter()
            .flat_map(move |decorations| {
                decorations
                    .iter()
                    .filter(move |span| span.col_range.contains(&col))
                    .map(|span| &span.decoration)
            })
    }

    /// Apply decorations to get final cell colors and underline
    pub fn apply_decorations(
        &self,
        line: usize,
        col: usize,
        default_fg: Hsla,
        default_bg: Hsla,
    ) -> (Hsla, Hsla, bool) {
        let mut fg = default_fg;
        let mut bg = default_bg;
        let mut underline = false;

        // Apply decorations in priority order (low to high)
        for decoration in self.get_decorations_for_cell(line, col) {
            match decoration {
                CellDecoration::Foreground { color, .. } => fg = *color,
                CellDecoration::Background { color, .. } => bg = *color,
                CellDecoration::Underline { .. } => underline = true,
                CellDecoration::Highlight {
                    foreground,
                    background,
                    ..
                } => {
                    fg = *foreground;
                    bg = *background;
                }
            }
        }

        (fg, bg, underline)
    }
}

/// Cached rendering data for a single line
#[derive(Clone)]
pub struct CachedLine {
    pub background_rects: Vec<(usize, usize, Hsla)>,
    pub text_runs: Vec<CachedTextRun>,
}

#[derive(Clone)]
pub struct CachedTextRun {
    pub start_col: usize,
    pub text: String,
    pub color: Hsla,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub char_count: usize,
}

/// Terminal rendering cache maintained by TerminalView
pub struct RenderCache {
    lines: Vec<CachedLine>,
    cursor: Option<CachedCursor>,
    num_cols: usize,
    num_lines: usize,
    colors: Colors,
    default_bg: Hsla,

    // Decoration system replaces all addon-specific fields
    decoration_manager: DecorationManager,

    custom_foreground: Hsla,
    custom_background: Hsla,
    /// 主题定义的光标颜色（确保与背景色不同）
    custom_cursor: Hsla,

    /// 上一帧的选择范围，用于增量更新
    last_selection: Option<SelectionRange>,

    /// 左边缘列指纹（用于检测脏区漏报导致的首列残字）
    left_edge_fingerprint: Vec<u64>,
}

#[derive(Clone)]
struct CachedCursor {
    column: usize,
    line: usize,
    shape: CursorShape,
}

impl RenderCache {
    pub fn new(num_lines: usize, num_cols: usize, colors: Colors) -> Self {
        let default_bg = convert_color(Color::Named(NamedColor::Background), &colors);
        Self {
            lines: vec![
                CachedLine {
                    background_rects: Vec::new(),
                    text_runs: Vec::new()
                };
                num_lines
            ],
            cursor: None,
            num_cols,
            num_lines,
            colors,
            default_bg,
            decoration_manager: DecorationManager::new(),
            custom_foreground: rgb(0xE4E4E4).into(),
            custom_background: rgb(0x1E1E1E).into(),
            custom_cursor: rgb(0xFFFFFF).into(),
            last_selection: None,
            left_edge_fingerprint: vec![0; num_lines],
        }
    }

    /// Update cache based on terminal damage, with incremental selection support
    pub fn update(
        &mut self,
        term: &mut Term<GpuiEventProxy>,
        addon_manager: &AddonManager,
        theme: &TerminalTheme,
    ) {
        let num_cols = term.columns();
        let num_lines = term.screen_lines();

        // Handle resize
        if num_lines != self.num_lines || num_cols != self.num_cols {
            self.resize(num_lines, num_cols);
        }

        // Collect decorations from all addons
        let display_offset = term.grid().display_offset();
        self.decoration_manager
            .collect_from_addons(addon_manager, 0..num_lines, display_offset);

        // Check if custom foreground changed
        let fg_changed = theme.foreground != self.custom_foreground;
        self.custom_foreground = theme.foreground;

        // Check if custom background changed
        let bg_changed = theme.background != self.custom_background;
        self.custom_background = theme.background;
        if bg_changed {
            self.default_bg = convert_color(Color::Named(NamedColor::Background), &self.colors)
        }

        // 同步主题光标颜色
        self.custom_cursor = theme.cursor;

        // Force full rebuild when theme colors or decorations changed
        let has_decorations = !self.decoration_manager.decorations_by_line.is_empty();
        if fg_changed || bg_changed || has_decorations {
            self.rebuild_all(term);
            self.update_last_selection(term);
            return;
        }

        // Check terminal color palette changes
        let colors = term.colors();
        if !colors_equal(&self.colors, colors) {
            self.colors = colors.clone();
            self.default_bg = convert_color(Color::Named(NamedColor::Background), &self.colors);
            self.rebuild_all(term);
            self.update_last_selection(term);
            return;
        }

        // Collect dirty lines from terminal damage
        let mut dirty_lines: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let damage = term.damage();
        match damage {
            TermDamage::Full => {
                self.rebuild_all(term);
                self.update_last_selection(term);
                return;
            }
            TermDamage::Partial(iter) => {
                for line_damage in iter {
                    dirty_lines.insert(line_damage.line);
                }
            }
        }

        // Incremental selection update: only rebuild affected lines
        let has_selection = term.selection.is_some();
        let had_selection = self.last_selection.is_some();

        if has_selection || had_selection {
            let current_selection = {
                let content = term.renderable_content();
                content.selection.clone()
            };
            if self.last_selection != current_selection {
                let sel_offset = term.grid().display_offset();
                let selection_lines = self.compute_selection_changed_lines(
                    self.last_selection.as_ref(),
                    current_selection.as_ref(),
                    sel_offset,
                );
                for line in selection_lines {
                    dirty_lines.insert(line);
                }
                self.last_selection = current_selection;
            }
        }

        // 首列兜底：检测左边缘变化但未被 damage 标记的行。
        let edge_changed_lines = self.detect_left_edge_changed_lines(term, 4);
        for line in &edge_changed_lines {
            dirty_lines.insert(*line);
        }

        // Rebuild dirty lines or just update cursor
        if dirty_lines.is_empty() {
            self.update_cursor(term);
        } else {
            let lines: Vec<usize> = dirty_lines.into_iter().collect();
            self.rebuild_lines(term, &lines);
        }
    }

    fn resize(&mut self, num_lines: usize, num_cols: usize) {
        self.num_lines = num_lines;
        self.num_cols = num_cols;
        self.lines.resize(
            num_lines,
            CachedLine {
                background_rects: Vec::new(),
                text_runs: Vec::new(),
            },
        );
        self.left_edge_fingerprint.resize(num_lines, 0);
    }

    fn rebuild_all(&mut self, term: &Term<GpuiEventProxy>) {
        let content = term.renderable_content();
        let display_offset = content.display_offset;
        let selection = &content.selection;

        // Clear all lines
        for line in &mut self.lines {
            line.background_rects.clear();
            line.text_runs.clear();
        }

        // Group cells by screen line
        let mut line_cells: Vec<Vec<CellData>> = (0..self.num_lines).map(|_| Vec::new()).collect();

        for cell in content.display_iter {
            let screen_line = cell.point.line.0 + display_offset as i32;
            if screen_line < 0 || screen_line as usize >= self.num_lines {
                continue;
            }

            let is_selected = selection
                .as_ref()
                .map(|s: &SelectionRange| s.contains(cell.point))
                .unwrap_or(false);

            line_cells[screen_line as usize].push(CellData {
                column: cell.point.column.0,
                c: cell.c,
                fg: cell.fg,
                bg: cell.bg,
                flags: cell.flags,
                is_selected,
            });
        }

        // Build cache for each line
        for (line_idx, cells) in line_cells.into_iter().enumerate() {
            self.build_line_cache(line_idx, cells);
        }

        // Update cursor from a fresh content
        let content = term.renderable_content();
        self.update_cursor_from_content(&content);
    }

    /// Rebuild specified lines
    fn rebuild_lines(&mut self, term: &Term<GpuiEventProxy>, lines: &[usize]) {
        let content = term.renderable_content();
        let display_offset = content.display_offset;
        let selection = &content.selection;

        let lines_set: std::collections::HashSet<usize> = lines.iter().copied().collect();

        // Collect cells for specified lines
        let mut line_cells: Vec<Vec<CellData>> = (0..self.num_lines).map(|_| Vec::new()).collect();

        for cell in content.display_iter {
            let screen_line = cell.point.line.0 + display_offset as i32;
            if screen_line < 0 || screen_line as usize >= self.num_lines {
                continue;
            }

            let line_idx = screen_line as usize;
            if !lines_set.contains(&line_idx) {
                continue;
            }

            let is_selected = selection
                .as_ref()
                .map(|s: &SelectionRange| s.contains(cell.point))
                .unwrap_or(false);

            line_cells[line_idx].push(CellData {
                column: cell.point.column.0,
                c: cell.c,
                fg: cell.fg,
                bg: cell.bg,
                flags: cell.flags,
                is_selected,
            });
        }

        // Rebuild specified lines
        for &line_idx in &lines_set {
            if line_idx < self.num_lines {
                self.lines[line_idx].background_rects.clear();
                self.lines[line_idx].text_runs.clear();
                let cells = std::mem::take(&mut line_cells[line_idx]);
                self.build_line_cache(line_idx, cells);
            }
        }

        // Update cursor from fresh content
        let content = term.renderable_content();
        self.update_cursor_from_content(&content);
    }

    /// Compute which screen lines are affected by a selection change
    fn compute_selection_changed_lines(
        &self,
        old_selection: Option<&SelectionRange>,
        new_selection: Option<&SelectionRange>,
        display_offset: usize,
    ) -> Vec<usize> {
        let mut changed_lines = Vec::new();

        // Collect lines from old selection
        if let Some(sel) = old_selection {
            let start_line = (sel.start.line.0 + display_offset as i32).max(0) as usize;
            let end_line = (sel.end.line.0 + display_offset as i32).max(0) as usize;
            for line in start_line..=end_line.min(self.num_lines.saturating_sub(1)) {
                changed_lines.push(line);
            }
        }

        // Collect lines from new selection
        if let Some(sel) = new_selection {
            let start_line = (sel.start.line.0 + display_offset as i32).max(0) as usize;
            let end_line = (sel.end.line.0 + display_offset as i32).max(0) as usize;
            for line in start_line..=end_line.min(self.num_lines.saturating_sub(1)) {
                if !changed_lines.contains(&line) {
                    changed_lines.push(line);
                }
            }
        }

        changed_lines
    }

    /// Update the last_selection tracking field
    fn update_last_selection(&mut self, term: &Term<GpuiEventProxy>) {
        let content = term.renderable_content();
        self.last_selection = content.selection.clone();
    }

    /// 计算并同步左边缘列指纹，返回发生变化的行。
    ///
    /// 目的：在某些复杂 ANSI 序列下，`TermDamage::Partial` 可能未覆盖到首列擦除场景，
    /// 该指纹用于兜底发现“首几列变化但未标脏”的行，避免残字。
    fn detect_left_edge_changed_lines(
        &mut self,
        term: &Term<GpuiEventProxy>,
        probe_cols: usize,
    ) -> Vec<usize> {
        if self.num_lines == 0 || probe_cols == 0 {
            return Vec::new();
        }

        let mut current = vec![0_u64; self.num_lines];
        let content = term.renderable_content();
        let display_offset = content.display_offset;

        for cell in content.display_iter {
            if cell.point.column.0 >= probe_cols {
                continue;
            }

            let screen_line = cell.point.line.0 + display_offset as i32;
            if screen_line < 0 || screen_line as usize >= self.num_lines {
                continue;
            }

            let line_idx = screen_line as usize;
            let code = cell.c as u32 as u64;
            let col = cell.point.column.0 as u64;
            let flags = cell.flags.bits() as u64;
            // 仅用于变化检测，不追求密码学强度
            let piece = col.wrapping_shl(56) ^ code.wrapping_shl(24) ^ flags;
            current[line_idx] = current[line_idx]
                .wrapping_mul(1099511628211)
                .wrapping_add(piece.wrapping_add(1469598103934665603));
        }

        if self.left_edge_fingerprint.len() != self.num_lines {
            self.left_edge_fingerprint.resize(self.num_lines, 0);
        }

        let mut changed = Vec::new();
        for (line_idx, (old, new)) in self
            .left_edge_fingerprint
            .iter()
            .zip(current.iter())
            .enumerate()
        {
            if old != new {
                changed.push(line_idx);
            }
        }

        self.left_edge_fingerprint = current;
        changed
    }

    fn build_line_cache(&mut self, line_idx: usize, mut cells: Vec<CellData>) {
        if cells.is_empty() {
            return;
        }

        cells.sort_unstable_by_key(|c| c.column);

        let line = &mut self.lines[line_idx];
        let mut bg_span: Option<(usize, Hsla)> = None;
        let mut text_run: Option<CachedTextRun> = None;

        for cell in &cells {
            // Get base colors from terminal
            let base_fg = convert_color(cell.fg, &self.colors);
            let base_bg = convert_color(cell.bg, &self.colors);

            // Apply selection (higher priority than decorations)
            let (mut fg, mut bg) = if cell.is_selected {
                (hsla(0.0, 0.0, 1.0, 1.0), hsla(0.58, 0.5, 0.4, 1.0))
            } else {
                (base_fg, base_bg)
            };

            // Apply decorations from addons (unless selected)
            let mut underline = false;
            if !cell.is_selected {
                let (deco_fg, deco_bg, deco_underline) =
                    self.decoration_manager
                        .apply_decorations(line_idx, cell.column, fg, bg);
                fg = deco_fg;
                bg = deco_bg;
                underline = deco_underline;
            }

            // Apply custom foreground for default foreground color (lowest priority)
            // Decorative characters (box drawing, powerline, etc.) keep their original colors
            if !cell.is_selected
                && matches!(cell.fg, Color::Named(NamedColor::Foreground))
                && !is_decorative_character(cell.c)
            {
                // Only apply if decorations didn't change the foreground
                if hsla_eq(fg, base_fg) {
                    fg = self.custom_foreground;
                }
            }

            // DIM 标志：降低前景色透明度
            if cell.flags.contains(Flags::DIM) {
                fg.a *= 0.7;
            }

            // 对比度保证：确保非装饰字符的文字可读性
            // 关键：当单元格使用默认背景时，应该用 custom_background（来自 TerminalTheme）
            // 而不是 alacritty 的 NamedColor::Background，因为实际渲染的背景是 custom_background
            if !cell.is_selected && !is_decorative_character(cell.c) {
                let actual_bg = if matches!(cell.bg, Color::Named(NamedColor::Background)) {
                    self.custom_background
                } else {
                    bg
                };
                fg = ensure_minimum_contrast(fg, actual_bg);
            }

            // Background batching
            let is_default_bg = !cell.is_selected && hsla_eq(bg, self.default_bg);

            if is_default_bg {
                if let Some((start, color)) = bg_span.take() {
                    line.background_rects.push((start, cell.column, color));
                }
            } else {
                match &mut bg_span {
                    Some((_, ref span_color)) if hsla_eq(*span_color, bg) => {}
                    Some((start, color)) => {
                        line.background_rects.push((*start, cell.column, *color));
                        bg_span = Some((cell.column, bg));
                    }
                    None => {
                        bg_span = Some((cell.column, bg));
                    }
                }
            }

            // Skip wide character spacer
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }

            // Skip blank characters
            if cell.c == '\0' || cell.c == ' ' {
                if let Some(run) = text_run.take() {
                    line.text_runs.push(run);
                }
                continue;
            }

            let bold = cell.flags.contains(Flags::BOLD);
            let italic = cell.flags.contains(Flags::ITALIC);

            // Check if we can merge with existing run
            let can_merge = if let Some(ref run) = text_run {
                // Merge only if columns are consecutive
                run.start_col + run.char_count == cell.column
                    && hsla_eq(run.color, fg)
                    && run.bold == bold
                    && run.italic == italic
                    && run.underline == underline
            } else {
                false
            };

            if can_merge {
                let run = text_run.as_mut().unwrap();
                run.text.push(cell.c);
                run.char_count += 1;
            } else {
                if let Some(run) = text_run.take() {
                    line.text_runs.push(run);
                }
                text_run = Some(CachedTextRun {
                    start_col: cell.column,
                    text: cell.c.to_string(),
                    color: fg,
                    bold,
                    italic,
                    underline,
                    char_count: 1,
                });
            }
        }

        // Flush remaining
        if let Some((start, color)) = bg_span {
            line.background_rects.push((start, self.num_cols, color));
        }
        if let Some(run) = text_run {
            line.text_runs.push(run);
        }
    }

    fn update_cursor(&mut self, term: &Term<GpuiEventProxy>) {
        let content = term.renderable_content();
        self.update_cursor_from_content(&content);
    }

    fn update_cursor_from_content(&mut self, content: &RenderableContent<'_>) {
        if content.cursor.shape != CursorShape::Hidden {
            let cursor_line = content.cursor.point.line.0 + content.display_offset as i32;
            if cursor_line >= 0 && (cursor_line as usize) < self.num_lines {
                self.cursor = Some(CachedCursor {
                    column: content.cursor.point.column.0,
                    line: cursor_line as usize,
                    shape: content.cursor.shape,
                });
                return;
            }
        }
        self.cursor = None;
    }
}

struct CellData {
    column: usize,
    c: char,
    fg: Color,
    bg: Color,
    flags: Flags,
    is_selected: bool, // Keep selection as it's from terminal state, not addon
}

impl Clone for CellData {
    fn clone(&self) -> Self {
        Self {
            column: self.column,
            c: self.c,
            fg: self.fg,
            bg: self.bg,
            flags: self.flags,
            is_selected: self.is_selected,
        }
    }
}

/// Terminal element that renders from cached data
pub struct TerminalElement<'a> {
    cache: &'a RenderCache,
    font_family: SharedString,
    font_size: Pixels,
    font_fallbacks: Vec<String>,
    line_height_scale: f32,
    cursor_visible: bool,
    /// 预计算的 cell_width，由 view.rs 传入，确保与 resize 使用相同的值
    cell_width: Pixels,
}

impl<'a> TerminalElement<'a> {
    pub fn new(
        cache: &'a RenderCache,
        font_family: SharedString,
        font_size: Pixels,
        font_fallbacks: Vec<String>,
        line_height_scale: f32,
        cursor_visible: bool,
        cell_width: Pixels,
    ) -> Self {
        Self {
            cache,
            font_family,
            font_size,
            font_fallbacks,
            line_height_scale,
            cursor_visible,
            cell_width,
        }
    }
}

impl<'a> IntoElement for TerminalElement<'a> {
    type Element = TerminalElementImpl;

    fn into_element(self) -> Self::Element {
        TerminalElementImpl {
            lines: self.cache.lines.clone(),
            cursor: self.cache.cursor.clone(),
            num_cols: self.cache.num_cols,
            custom_background: self.cache.custom_background,
            custom_cursor: self.cache.custom_cursor,
            font_family: self.font_family,
            font_size: self.font_size,
            font_fallbacks: self.font_fallbacks,
            line_height_scale: self.line_height_scale,
            cursor_visible: self.cursor_visible,
            cell_width: self.cell_width,
        }
    }
}

pub struct TerminalElementImpl {
    lines: Vec<CachedLine>,
    cursor: Option<CachedCursor>,
    num_cols: usize,
    /// 主题定义的背景色
    custom_background: Hsla,
    /// 主题定义的光标颜色
    custom_cursor: Hsla,
    font_family: SharedString,
    font_size: Pixels,
    font_fallbacks: Vec<String>,
    line_height_scale: f32,
    cursor_visible: bool,
    /// 预计算的 cell_width，确保与 resize 使用相同的值
    cell_width: Pixels,
}

pub struct TerminalLayout {
    bounds: TerminalBounds,
    /// 预缓存的字体变体
    fonts: FontVariants,
}

#[derive(Clone, Copy)]
struct TerminalBounds {
    cell_width: Pixels,
    cell_height: Pixels,
    origin: Point<Pixels>,
}

impl TerminalBounds {
    #[inline]
    fn cell_origin(&self, line: usize, column: usize) -> Point<Pixels> {
        Point::new(
            self.origin.x + self.cell_width * column as f32,
            self.origin.y + self.cell_height * line as f32,
        )
    }

    #[inline]
    fn cell_rect(&self, line: usize, column: usize) -> Bounds<Pixels> {
        Bounds::new(
            self.cell_origin(line, column),
            size(self.cell_width, self.cell_height),
        )
    }
}

impl IntoElement for TerminalElementImpl {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TerminalElementImpl {
    type RequestLayoutState = ();
    type PrepaintState = TerminalLayout;

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
        // 关键：终端绘制元素必须填满父容器。
        // 使用 flex+auto 在绝对定位父容器中可能得到 0 高度，导致 element_bounds 异常。
        let style = Style {
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
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        // 预创建所有字体变体，避免在 paint 中逐次创建
        let fonts = FontVariants::new(self.font_family.clone(), self.font_fallbacks.clone());

        let line_height = self.font_size * self.line_height_scale;
        // 使用由 view.rs 传入的 cell_width，确保与 resize 使用完全相同的值
        let cell_width = self.cell_width;

        TerminalLayout {
            bounds: TerminalBounds {
                cell_width,
                cell_height: line_height,
                origin: bounds.origin,
            },
            fonts,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let tb = &prepaint.bounds;
        let fonts = &prepaint.fonts;

        // 视口裁剪：计算可见行范围，跳过不可见行的渲染
        let content_mask = window.content_mask().bounds;
        let terminal_height = tb.cell_height * self.lines.len() as f32;
        let terminal_bounds = Bounds::new(
            tb.origin,
            size(tb.cell_width * self.num_cols as f32, terminal_height),
        );

        let intersection = content_mask.intersect(&terminal_bounds);
        if intersection.size.height <= px(0.) || intersection.size.width <= px(0.) {
            return; // 完全不可见，跳过渲染
        }

        // 背景覆盖整个 content_mask 可见区域，而非仅 terminal_bounds
        // terminal_bounds 基于缓存尺寸 (num_cols * cell_width, num_lines * cell_height)，
        // 在 resize 过渡期间或交互式程序重绘时可能小于实际可见区域，
        // 导致边缘区域残留上一帧的文字。使用 content_mask 可确保全部区域被清除。
        window.paint_quad(fill(content_mask, self.custom_background));

        let first_visible = ((intersection.origin.y - tb.origin.y) / tb.cell_height)
            .floor()
            .max(0.0) as usize;
        let last_visible = ((intersection.origin.y + intersection.size.height - tb.origin.y)
            / tb.cell_height)
            .ceil() as usize;
        let visible_end = last_visible.min(self.lines.len());

        // Paint backgrounds (only visible lines)
        for line_idx in first_visible..visible_end {
            let line = &self.lines[line_idx];
            for &(start, end, color) in &line.background_rects {
                let rect = Bounds::new(
                    tb.cell_origin(line_idx, start),
                    size(tb.cell_width * (end - start) as f32, tb.cell_height),
                );
                window.paint_quad(fill(rect, color));
            }
        }

        // Paint text (only visible lines, using cached fonts)
        // 使用 cell_width 确保等宽渲染，避免字符布局漂移
        for line_idx in first_visible..visible_end {
            let line = &self.lines[line_idx];
            for run in &line.text_runs {
                let font = fonts.get(run.bold, run.italic);

                let underline = if run.underline {
                    Some(UnderlineStyle {
                        thickness: px(1.0),
                        color: Some(run.color),
                        wavy: false,
                    })
                } else {
                    None
                };

                let shaped = window.text_system().shape_line(
                    run.text.clone().into(),
                    self.font_size,
                    &[TextRun {
                        len: run.text.len(),
                        font: font.clone(),
                        color: run.color,
                        background_color: None,
                        underline,
                        strikethrough: None,
                    }],
                    Some(tb.cell_width),
                );
                let _ = shaped.paint(
                    tb.cell_origin(line_idx, run.start_col),
                    tb.cell_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }

        // Paint cursor (if visible and in visible range)
        if self.cursor_visible {
            if let Some(cursor) = &self.cursor {
                if cursor.line >= first_visible
                    && cursor.line < visible_end
                    && cursor.column < self.num_cols
                {
                    let cursor_color = self.custom_cursor;
                    let cursor_bounds = tb.cell_rect(cursor.line, cursor.column);

                    match cursor.shape {
                        CursorShape::Block => {
                            window.paint_quad(fill(cursor_bounds, cursor_color));
                        }
                        CursorShape::Underline => {
                            let h = px(2.0);
                            let underline = Bounds::new(
                                Point::new(
                                    cursor_bounds.origin.x,
                                    cursor_bounds.origin.y + tb.cell_height - h,
                                ),
                                size(tb.cell_width, h),
                            );
                            window.paint_quad(fill(underline, cursor_color));
                        }
                        CursorShape::Beam => {
                            let beam =
                                Bounds::new(cursor_bounds.origin, size(px(2.0), tb.cell_height));
                            window.paint_quad(fill(beam, cursor_color));
                        }
                        CursorShape::HollowBlock => {
                            window.paint_quad(quad(
                                cursor_bounds,
                                Corners::default(),
                                hsla(0.0, 0.0, 0.0, 0.0),
                                Edges::all(px(1.0)),
                                cursor_color,
                                BorderStyle::Solid,
                            ));
                        }
                        _ => {
                            window.paint_quad(fill(cursor_bounds, cursor_color));
                        }
                    }
                }
            }
        }
    }
}

// Helper functions

/// 快速颜色比较 - 使用位比较代替浮点比较
#[inline]
fn hsla_eq(a: Hsla, b: Hsla) -> bool {
    a.h.to_bits() == b.h.to_bits()
        && a.s.to_bits() == b.s.to_bits()
        && a.l.to_bits() == b.l.to_bits()
        && a.a.to_bits() == b.a.to_bits()
}

fn colors_equal(a: &Colors, b: &Colors) -> bool {
    for i in 0..269 {
        if a[i] != b[i] {
            return false;
        }
    }
    true
}

#[inline]
fn rgb_to_hsla(rgb: Rgb) -> Hsla {
    Rgba {
        r: rgb.r as f32 / 255.0,
        g: rgb.g as f32 / 255.0,
        b: rgb.b as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

fn convert_color(color: Color, colors: &Colors) -> Hsla {
    match color {
        Color::Named(named) => colors[named]
            .map(rgb_to_hsla)
            .unwrap_or_else(|| named_color_to_hsla(named)),
        Color::Spec(rgb) => rgb_to_hsla(rgb),
        Color::Indexed(idx) => colors[idx as usize]
            .map(rgb_to_hsla)
            .unwrap_or_else(|| indexed_color_to_hsla(idx)),
    }
}

// ============================================================================
// 对比度保证
// ============================================================================

/// WCAG AA 最小对比度 (4.5:1)
const MIN_CONTRAST_RATIO: f32 = 4.5;

/// 将 sRGB 分量线性化
#[inline]
fn linearize(value: f32) -> f32 {
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// 计算相对亮度 (WCAG 定义)
fn relative_luminance(color: Hsla) -> f32 {
    let rgba: Rgba = color.into();
    let r = linearize(rgba.r);
    let g = linearize(rgba.g);
    let b = linearize(rgba.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// 计算两个颜色之间的对比度 (WCAG 标准)
fn contrast_ratio(fg: Hsla, bg: Hsla) -> f32 {
    let fg_lum = relative_luminance(fg);
    let bg_lum = relative_luminance(bg);
    let (lighter, darker) = if fg_lum > bg_lum {
        (fg_lum, bg_lum)
    } else {
        (bg_lum, fg_lum)
    };
    (lighter + 0.05) / (darker + 0.05)
}

/// 确保前景色与背景色有最小对比度
/// 如果对比度不足，调整前景色的亮度
fn ensure_minimum_contrast(fg: Hsla, bg: Hsla) -> Hsla {
    let ratio = contrast_ratio(fg, bg);
    if ratio >= MIN_CONTRAST_RATIO {
        return fg;
    }

    // 根据背景亮度决定调整方向
    let bg_lum = relative_luminance(bg);
    let mut adjusted = fg;

    // 尝试调整亮度以达到最小对比度
    if bg_lum > 0.5 {
        // 亮背景 -> 降低前景亮度
        adjusted.l = (adjusted.l - 0.2).max(0.0);
    } else {
        // 暗背景 -> 提高前景亮度
        adjusted.l = (adjusted.l + 0.2).min(1.0);
    }

    // 如果仍然不够，进一步调整
    if contrast_ratio(adjusted, bg) < MIN_CONTRAST_RATIO {
        if bg_lum > 0.5 {
            adjusted.l = 0.0; // 纯黑
        } else {
            adjusted.l = 1.0; // 纯白
        }
    }

    adjusted
}

fn named_color_to_hsla(color: NamedColor) -> Hsla {
    let (r, g, b) = match color {
        NamedColor::Black => (0.0, 0.0, 0.0),
        NamedColor::Red => (0.80, 0.19, 0.19),
        NamedColor::Green => (0.05, 0.74, 0.47),
        NamedColor::Yellow => (0.90, 0.90, 0.06),
        NamedColor::Blue => (0.14, 0.45, 0.78),
        NamedColor::Magenta => (0.74, 0.25, 0.74),
        NamedColor::Cyan => (0.07, 0.66, 0.80),
        NamedColor::White => (0.90, 0.90, 0.90),
        NamedColor::BrightBlack => (0.40, 0.40, 0.40),
        NamedColor::BrightRed => (0.95, 0.30, 0.30),
        NamedColor::BrightGreen => (0.14, 0.82, 0.55),
        NamedColor::BrightYellow => (0.96, 0.96, 0.26),
        NamedColor::BrightBlue => (0.23, 0.56, 0.92),
        NamedColor::BrightMagenta => (0.84, 0.44, 0.84),
        NamedColor::BrightCyan => (0.16, 0.72, 0.86),
        NamedColor::BrightWhite => (1.0, 1.0, 1.0),
        NamedColor::Foreground => (0.83, 0.83, 0.83),
        NamedColor::Background => (0.12, 0.12, 0.12),
        NamedColor::Cursor => return hsla(0.0, 0.0, 1.0, 0.8),
        _ => (0.83, 0.83, 0.83),
    };
    Rgba { r, g, b, a: 1.0 }.into()
}

fn indexed_color_to_hsla(idx: u8) -> Hsla {
    match idx {
        0..=15 => {
            let named = match idx {
                0 => NamedColor::Black,
                1 => NamedColor::Red,
                2 => NamedColor::Green,
                3 => NamedColor::Yellow,
                4 => NamedColor::Blue,
                5 => NamedColor::Magenta,
                6 => NamedColor::Cyan,
                7 => NamedColor::White,
                8 => NamedColor::BrightBlack,
                9 => NamedColor::BrightRed,
                10 => NamedColor::BrightGreen,
                11 => NamedColor::BrightYellow,
                12 => NamedColor::BrightBlue,
                13 => NamedColor::BrightMagenta,
                14 => NamedColor::BrightCyan,
                15 => NamedColor::BrightWhite,
                _ => unreachable!(),
            };
            named_color_to_hsla(named)
        }
        16..=231 => {
            let idx = idx - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            let to_component = |v: u8| {
                if v == 0 {
                    0.0
                } else {
                    (55.0 + v as f32 * 40.0) / 255.0
                }
            };
            Rgba {
                r: to_component(r),
                g: to_component(g),
                b: to_component(b),
                a: 1.0,
            }
            .into()
        }
        232..=255 => {
            let shade = (8.0 + (idx - 232) as f32 * 10.0) / 255.0;
            Rgba {
                r: shade,
                g: shade,
                b: shade,
                a: 1.0,
            }
            .into()
        }
    }
}

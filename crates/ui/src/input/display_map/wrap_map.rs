/// WrapMap: Soft-wrapping layer (Buffer → Wrap rows).
///
/// This module wraps the existing TextWrapper and provides:
/// - BufferPoint ↔ WrapPoint mapping
/// - Efficient buffer_line → wrap_row queries via prefix sum cache
/// - Incremental updates when text or layout changes
use std::ops::Range;

use gpui::{App, Font, Pixels};
use ropey::Rope;

use super::fold_map::FoldMap;
use super::text_wrapper::{LineItem, TextWrapper, WrapDisplayPoint};
use super::{BufferPoint, WrapPoint};
use crate::input::rope_ext::RopeExt;

/// WrapMap manages soft-wrapping and provides buffer ↔ wrap coordinate mapping.
pub struct WrapMap {
    /// The underlying text wrapper (reuses existing implementation)
    wrapper: TextWrapper,

    /// Prefix sum cache: buffer_line_starts[line] = first wrap_row for buffer line `line`
    /// This allows O(1) lookup of buffer_line → wrap_row
    buffer_line_starts: Vec<usize>,

    /// Cached line count from last rebuild
    /// Used to detect if we need full or incremental cache update
    cached_line_count: usize,
}

impl WrapMap {
    pub fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            wrapper: TextWrapper::new(font, font_size, wrap_width),
            buffer_line_starts: Vec::new(),
            cached_line_count: 0,
        }
    }

    /// Get total number of wrap rows (visual rows after soft-wrapping)
    #[inline]
    pub fn wrap_row_count(&self) -> usize {
        self.wrapper.len()
    }

    /// Get total number of buffer lines (logical lines)
    #[inline]
    pub fn buffer_line_count(&self) -> usize {
        self.wrapper.lines.len()
    }

    /// Convert buffer position to wrap position
    pub(super) fn buffer_pos_to_wrap_pos(&self, pos: BufferPoint) -> WrapPoint {
        let BufferPoint { line, col } = pos;

        // Clamp to valid range
        let line = line.min(self.buffer_line_count().saturating_sub(1));
        let line_item = self.wrapper.lines.get(line);

        let col = if let Some(line_item) = line_item {
            col.min(line_item.len())
        } else {
            0
        };

        // Calculate offset in rope
        let line_start_offset = self.wrapper.text().line_start_offset(line);
        let offset = line_start_offset + col;

        // Use TextWrapper's existing conversion
        let display_point = self.wrapper.offset_to_display_point(offset);

        WrapPoint::new(display_point.row, display_point.column)
    }

    /// Convert wrap position to buffer position
    pub(super) fn wrap_pos_to_buffer_pos(&self, pos: WrapPoint) -> BufferPoint {
        let WrapPoint { row, col } = pos;

        // Clamp wrap_row to valid range
        let row = row.min(self.wrap_row_count().saturating_sub(1));

        // Use TextWrapper's existing conversion
        let display_point = WrapDisplayPoint::new(row, 0, col);
        let offset = self.wrapper.display_point_to_offset(display_point);

        // Convert offset to buffer position
        let point = self.wrapper.text().offset_to_point(offset);
        let line_start = self.wrapper.text().line_start_offset(point.row);
        let col = offset.saturating_sub(line_start);

        BufferPoint::new(point.row, col)
    }

    /// Get the buffer line for a given wrap row
    pub fn wrap_row_to_buffer_line(&self, wrap_row: usize) -> usize {
        if wrap_row >= self.wrap_row_count() {
            return self.buffer_line_count().saturating_sub(1);
        }

        // Binary search in prefix sum cache
        match self.buffer_line_starts.binary_search(&wrap_row) {
            Ok(line) => line,
            Err(insert_pos) => insert_pos.saturating_sub(1),
        }
    }

    /// Get the first wrap row for a given buffer line
    pub fn buffer_line_to_first_wrap_row(&self, line: usize) -> usize {
        if line >= self.buffer_line_starts.len() {
            return self.wrap_row_count();
        }
        self.buffer_line_starts[line]
    }

    /// Get the wrap row range for a buffer line: [start, end)
    pub fn buffer_line_to_wrap_row_range(&self, line: usize) -> Range<usize> {
        let start = self.buffer_line_to_first_wrap_row(line);
        let end = if line + 1 < self.buffer_line_starts.len() {
            self.buffer_line_starts[line + 1]
        } else {
            self.wrap_row_count()
        };
        start..end
    }

    /// Update text (incremental or full)
    pub fn on_text_changed(
        &mut self,
        changed_text: &Rope,
        range: &Range<usize>,
        new_text: &Rope,
        cx: &mut App,
    ) {
        self.wrapper.update(changed_text, range, new_text, cx);
        self.rebuild_cache();
    }

    /// Update layout parameters (wrap width or font)
    pub fn on_layout_changed(&mut self, wrap_width: Option<Pixels>, cx: &mut App) {
        self.wrapper.set_wrap_width(wrap_width, cx);
        self.rebuild_cache();
    }

    /// Set font parameters
    pub fn set_font(&mut self, font: Font, font_size: Pixels, cx: &mut App) {
        self.wrapper.set_font(font, font_size, cx);
        self.rebuild_cache();
    }

    /// Ensure text is prepared (initializes wrapper if needed)
    pub fn ensure_text_prepared(&mut self, text: &Rope, cx: &mut App) -> bool {
        let did_initialize = self.wrapper.prepare_if_need(text, cx);
        if did_initialize {
            self.rebuild_cache();
        }
        did_initialize
    }

    /// Initialize with text
    pub fn set_text(&mut self, text: &Rope, cx: &mut App) {
        self.wrapper.set_default_text(text);
        self.wrapper.prepare_if_need(text, cx);
        self.rebuild_cache();
    }

    /// Rebuild the prefix sum cache: buffer_line_starts
    fn rebuild_cache(&mut self) {
        let line_count = self.wrapper.lines.len();

        // Performance optimization: skip if nothing changed
        if line_count == self.cached_line_count && !self.buffer_line_starts.is_empty() {
            return;
        }

        // Check if we can do incremental update
        let can_incremental = line_count > self.cached_line_count
            && self.cached_line_count > 0
            && self.buffer_line_starts.len() == self.cached_line_count;

        if can_incremental {
            // Incremental update: only process new lines
            let mut wrap_row = if let Some(&last) = self.buffer_line_starts.last() {
                let last_line = &self.wrapper.lines[self.cached_line_count - 1];
                last + last_line.lines_len()
            } else {
                0
            };

            for line_item in &self.wrapper.lines[self.cached_line_count..] {
                self.buffer_line_starts.push(wrap_row);
                wrap_row += line_item.lines_len();
            }
        } else {
            // Full rebuild
            self.buffer_line_starts.clear();

            let mut wrap_row = 0;
            for line_item in &self.wrapper.lines {
                self.buffer_line_starts.push(wrap_row);
                wrap_row += line_item.lines_len();
            }
        }

        self.cached_line_count = line_count;
    }

    /// Get access to the underlying wrapper (for rendering/hit-testing)
    pub(crate) fn wrapper(&self) -> &TextWrapper {
        &self.wrapper
    }

    /// Get access to line items (for rendering)
    pub(crate) fn lines(&self) -> &[LineItem] {
        &self.wrapper.lines
    }

    /// Get the rope text
    pub fn text(&self) -> &Rope {
        self.wrapper.text()
    }

    /// Calculate how many wrap rows of a buffer line are visible (not folded)
    pub fn visible_wrap_row_count_for_line(&self, line: usize, fold_map: &FoldMap) -> usize {
        let wrap_range = self.buffer_line_to_wrap_row_range(line);
        wrap_range
            .filter(|&wr| fold_map.wrap_row_to_display_row(wr).is_some())
            .count()
    }
}

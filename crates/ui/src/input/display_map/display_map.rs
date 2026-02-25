/// DisplayMap: Public facade for Editor/Input display mapping.
///
/// This combines WrapMap and FoldMap to provide a unified API:
/// - BufferPos ↔ DisplayPos conversion
/// - Fold management (candidates, toggle, query)
/// - Automatic projection updates on text/layout changes

use std::ops::Range;

use gpui::{App, Font, Pixels};
use ropey::Rope;

use super::fold_map::FoldMap;
use super::text_wrapper::{LineItem, TextWrapper};
use super::types::{BufferPos, DisplayPos};
use super::wrap_map::WrapMap;
use crate::highlighter::FoldRange;

/// DisplayMap is the main interface for Editor/Input coordinate mapping.
///
/// It manages the two-layer projection:
/// 1. Buffer → Wrap (soft-wrapping)
/// 2. Wrap → Display (folding)
///
/// Editor/Input only needs to work with BufferPos and DisplayPos.
pub struct DisplayMap {
    wrap_map: WrapMap,
    fold_map: FoldMap,
}

impl DisplayMap {
    pub fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            wrap_map: WrapMap::new(font, font_size, wrap_width),
            fold_map: FoldMap::new(),
        }
    }

    // ==================== Core Coordinate Mapping ====================

    /// Convert buffer position to display position
    pub fn buffer_pos_to_display_pos(&self, pos: BufferPos) -> DisplayPos {
        // Buffer → Wrap
        let wrap_pos = self.wrap_map.buffer_pos_to_wrap_pos(pos);

        // Wrap → Display
        if let Some(display_row) = self.fold_map.wrap_row_to_display_row(wrap_pos.row) {
            DisplayPos::new(display_row, wrap_pos.col)
        } else {
            // Cursor is in a folded region, find nearest visible row
            let display_row = self.fold_map.nearest_visible_display_row(wrap_pos.row);
            DisplayPos::new(display_row, 0) // Column 0 at fold boundary
        }
    }

    /// Convert display position to buffer position
    pub fn display_pos_to_buffer_pos(&self, pos: DisplayPos) -> BufferPos {
        // Display → Wrap
        let wrap_row = self
            .fold_map
            .display_row_to_wrap_row(pos.row)
            .unwrap_or(0);

        // Wrap → Buffer
        let wrap_pos = super::types::WrapPos::new(wrap_row, pos.col);
        self.wrap_map.wrap_pos_to_buffer_pos(wrap_pos)
    }

    // ==================== Display Row Queries ====================

    /// Get total number of visible display rows
    pub fn display_row_count(&self) -> usize {
        self.fold_map.display_row_count()
    }

    /// Check if a display row is visible
    pub fn is_display_row_visible(&self, display_row: usize) -> bool {
        display_row < self.display_row_count()
    }

    // ==================== Buffer Line Queries ====================

    /// Get the buffer line for a given display row
    pub fn display_row_to_buffer_line(&self, display_row: usize) -> usize {
        // Display → Wrap
        let wrap_row = self
            .fold_map
            .display_row_to_wrap_row(display_row)
            .unwrap_or(0);

        // Wrap → Buffer line
        self.wrap_map.wrap_row_to_buffer_line(wrap_row)
    }

    /// Get the display row range for a buffer line: [start, end)
    /// Returns None if the buffer line is completely hidden
    pub fn buffer_line_to_display_row_range(&self, line: usize) -> Option<Range<usize>> {
        // Buffer line → Wrap row range
        let wrap_row_range = self.wrap_map.buffer_line_to_wrap_row_range(line);

        // Find first and last visible display rows in this range
        let mut first_display_row = None;
        let mut last_display_row = None;

        for wrap_row in wrap_row_range {
            if let Some(display_row) = self.fold_map.wrap_row_to_display_row(wrap_row) {
                if first_display_row.is_none() {
                    first_display_row = Some(display_row);
                }
                last_display_row = Some(display_row);
            }
        }

        if let (Some(start), Some(end)) = (first_display_row, last_display_row) {
            Some(start..end + 1)
        } else {
            None // Completely folded
        }
    }

    /// Check if a buffer line is completely hidden
    pub fn is_buffer_line_hidden(&self, line: usize) -> bool {
        self.buffer_line_to_display_row_range(line).is_none()
    }

    // ==================== Fold Management ====================

    /// Set fold candidates (from tree-sitter/LSP)
    pub fn set_fold_candidates(&mut self, candidates: Vec<FoldRange>) {
        self.fold_map.set_candidates(candidates);
        self.rebuild_fold_projection();
    }

    /// Set a fold at the given start_line (must be in candidates)
    pub fn set_folded(&mut self, start_line: usize, folded: bool) {
        self.fold_map.set_folded(start_line, folded);
        self.rebuild_fold_projection();
    }

    /// Toggle fold at the given start_line
    pub fn toggle_fold(&mut self, start_line: usize) {
        self.fold_map.toggle_fold(start_line);
        self.rebuild_fold_projection();
    }

    /// Check if a line is currently folded
    pub fn is_folded_at(&self, start_line: usize) -> bool {
        self.fold_map.is_folded_at(start_line)
    }

    /// Check if a line is a fold candidate
    pub fn is_fold_candidate(&self, start_line: usize) -> bool {
        self.fold_map.is_fold_candidate(start_line)
    }

    /// Get all fold candidates
    pub fn fold_candidates(&self) -> &[FoldRange] {
        self.fold_map.fold_candidates()
    }

    /// Get all currently folded ranges
    pub fn folded_ranges(&self) -> &[FoldRange] {
        self.fold_map.folded_ranges()
    }

    /// Clear all folds
    pub fn clear_folds(&mut self) {
        self.fold_map.clear_folds();
        self.rebuild_fold_projection();
    }

    // ==================== Text and Layout Updates ====================

    /// Update text (incremental or full)
    pub fn on_text_changed(
        &mut self,
        changed_text: &Rope,
        range: &Range<usize>,
        new_text: &Rope,
        cx: &mut App,
    ) {
        self.wrap_map
            .on_text_changed(changed_text, range, new_text, cx);
        self.rebuild_fold_projection();
    }

    /// Update layout parameters (wrap width or font)
    pub fn on_layout_changed(&mut self, wrap_width: Option<Pixels>, cx: &mut App) {
        self.wrap_map.on_layout_changed(wrap_width, cx);
        self.rebuild_fold_projection();
    }

    /// Set font parameters
    pub fn set_font(&mut self, font: Font, font_size: Pixels, cx: &mut App) {
        self.wrap_map.set_font(font, font_size, cx);
        self.rebuild_fold_projection();
    }

    /// Ensure text is prepared (initializes wrapper if needed)
    pub fn ensure_text_prepared(&mut self, text: &Rope, cx: &mut App) {
        let did_initialize = self.wrap_map.ensure_text_prepared(text, cx);
        if did_initialize {
            self.rebuild_fold_projection();
        }
    }

    /// Initialize with text
    pub fn set_text(&mut self, text: &Rope, cx: &mut App) {
        self.wrap_map.set_text(text, cx);
        self.rebuild_fold_projection();
    }

    // ==================== Internal Helpers ====================

    /// Rebuild fold projection after wrap_map or fold state changes
    /// Only rebuilds if there are actually folded ranges
    fn rebuild_fold_projection(&mut self) {
        // Optimization: skip rebuild if no folds are active
        // This avoids expensive O(n) traversal of all wrap rows on every text change
        if !self.fold_map.folded_ranges().is_empty() || !self.fold_map.fold_candidates().is_empty() {
            self.fold_map.rebuild(&self.wrap_map);
        } else {
            // Fast path: mark dirty but don't rebuild yet
            self.fold_map.mark_dirty();
        }
    }

    // ==================== Access to Underlying Layers ====================
    // These are provided for gradual migration from TextWrapper to DisplayMap.
    // TODO: Remove these after full migration to DisplayMap API.

    /// Get access to the underlying WrapMap (for gradual migration)
    ///
    /// This allows existing code to access wrap-related functionality
    /// while we gradually migrate to the DisplayMap API.
    pub fn wrap_map(&self) -> &WrapMap {
        &self.wrap_map
    }

    /// Get access to the underlying FoldMap (for gradual migration)
    ///
    /// This allows existing code to access fold-related functionality
    /// while we gradually migrate to the DisplayMap API.
    pub fn fold_map(&self) -> &FoldMap {
        &self.fold_map
    }

    /// Get access to the underlying wrapper (for rendering/hit-testing)
    pub(crate) fn wrapper(&self) -> &TextWrapper {
        self.wrap_map.wrapper()
    }

    /// Get access to line items (for rendering)
    pub(crate) fn lines(&self) -> &[LineItem] {
        self.wrap_map.lines()
    }

    /// Get the rope text
    pub fn text(&self) -> &Rope {
        self.wrap_map.text()
    }

    /// Calculate how many wrap rows of a buffer line are visible (not folded)
    pub fn visible_wrap_row_count_for_buffer_line(&self, line: usize) -> usize {
        self.wrap_map
            .visible_wrap_row_count_for_line(line, &self.fold_map)
    }

    // ==================== Row Count Queries ====================

    /// Get the wrap row count (before folding)
    pub fn wrap_row_count(&self) -> usize {
        self.wrap_map.wrap_row_count()
    }

    /// Get the buffer line count (logical lines)
    pub fn buffer_line_count(&self) -> usize {
        self.wrap_map.buffer_line_count()
    }
}

// Tests omitted - requires GPUI test context setup

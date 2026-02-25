/// FoldMap: Folding projection layer (Wrap rows → Display rows).
///
/// This module manages code folding by:
/// - Filtering out wrap rows that belong to folded regions
/// - Maintaining bidirectional mapping: wrap_row ↔ display_row
/// - Handling fold state changes and rebuilding the projection
use super::folding::FoldRange;
use super::wrap_map::WrapMap;

/// FoldMap projects wrap rows to display rows by hiding folded regions.
pub struct FoldMap {
    /// Mapping: display_row → wrap_row
    /// index = display_row, value = actual wrap_row
    visible_wrap_rows: Vec<usize>,

    /// Reverse mapping: wrap_row → display_row
    /// index = wrap_row, value = Some(display_row) if visible, None if folded
    wrap_row_to_display_row: Vec<Option<usize>>,

    /// Candidate fold ranges (from tree-sitter/LSP)
    /// Sorted by start_line, unique start_line
    candidates: Vec<FoldRange>,

    /// Currently folded ranges
    /// Subset of candidates, sorted by start_line
    folded: Vec<FoldRange>,

    /// Flag indicating if the fold projection needs rebuilding
    /// Used for lazy evaluation to avoid expensive rebuilds on every text change
    needs_rebuild: bool,

    /// Cached wrap_row_count from last rebuild
    /// Used to detect if WrapMap changed and rebuild is needed
    cached_wrap_row_count: usize,
}

impl FoldMap {
    pub fn new() -> Self {
        Self {
            visible_wrap_rows: Vec::new(),
            wrap_row_to_display_row: Vec::new(),
            candidates: Vec::new(),
            folded: Vec::new(),
            needs_rebuild: true,
            cached_wrap_row_count: 0,
        }
    }

    /// Mark that the fold projection needs rebuilding
    /// This is called when wrap_map changes, but we defer the rebuild until needed
    pub(super) fn mark_dirty(&mut self) {
        self.needs_rebuild = true;
    }

    /// Get total number of visible display rows
    pub fn display_row_count(&self) -> usize {
        self.visible_wrap_rows.len()
    }

    /// Convert wrap_row to display_row
    /// Returns None if the wrap_row is hidden by folding
    pub fn wrap_row_to_display_row(&self, wrap_row: usize) -> Option<usize> {
        self.wrap_row_to_display_row
            .get(wrap_row)
            .copied()
            .flatten()
    }

    /// Convert display_row to wrap_row
    pub fn display_row_to_wrap_row(&self, display_row: usize) -> Option<usize> {
        self.visible_wrap_rows.get(display_row).copied()
    }

    /// Check if a wrap_row is visible
    pub fn is_wrap_row_visible(&self, wrap_row: usize) -> bool {
        self.wrap_row_to_display_row(wrap_row).is_some()
    }

    /// Find the nearest visible display_row for a given wrap_row
    /// (used when cursor falls inside a folded region)
    pub fn nearest_visible_display_row(&self, wrap_row: usize) -> usize {
        if let Some(dr) = self.wrap_row_to_display_row(wrap_row) {
            return dr;
        }

        // Binary search for the largest visible wrap_row <= target wrap_row
        match self.visible_wrap_rows.binary_search(&wrap_row) {
            Ok(idx) => idx,
            Err(insert_pos) => insert_pos.saturating_sub(1),
        }
    }

    /// Set fold candidates (from tree-sitter/LSP), full replacement.
    pub fn set_candidates(&mut self, mut candidates: Vec<FoldRange>) {
        // Sort and deduplicate by start_line
        candidates.sort_by_key(|r| r.start_line);
        candidates.dedup_by_key(|r| r.start_line);
        self.candidates = candidates;

        // Remove any folded ranges that are no longer in candidates
        self.folded.retain(|fold| {
            self.candidates
                .iter()
                .any(|c| c.start_line == fold.start_line)
        });
    }

    /// Merge new candidates extracted from an edited region into existing candidates.
    ///
    /// Replaces candidates within [edit_start_line, edit_end_line] with `new_candidates`,
    /// keeping candidates outside the edit range intact.
    pub fn merge_candidates_for_edit(
        &mut self,
        edit_start_line: usize,
        edit_end_line: usize,
        new_candidates: Vec<FoldRange>,
    ) {
        // Remove old candidates within the edit range (already done by adjust_folds_for_edit)
        // But do it again in case adjust wasn't called or range differs
        self.candidates.retain(|c| {
            c.start_line < edit_start_line || c.start_line > edit_end_line
        });

        // Add new candidates
        self.candidates.extend(new_candidates);
        self.candidates.sort_by_key(|r| r.start_line);
        self.candidates.dedup_by_key(|r| r.start_line);
    }

    /// Set a fold at the given start_line (must be in candidates)
    pub fn set_folded(&mut self, start_line: usize, folded: bool) {
        if folded {
            // Find the candidate range for this start_line
            if let Some(candidate) = self.candidates.iter().find(|c| c.start_line == start_line) {
                // Add to folded if not already present
                if !self.folded.iter().any(|f| f.start_line == start_line) {
                    self.folded.push(*candidate);
                    self.folded.sort_by_key(|r| r.start_line);
                    self.needs_rebuild = true;
                }
            }
        } else {
            // Remove from folded
            self.folded.retain(|f| f.start_line != start_line);
            self.needs_rebuild = true;
        }
    }

    /// Toggle fold at the given start_line
    pub fn toggle_fold(&mut self, start_line: usize) {
        let is_folded = self.is_folded_at(start_line);
        self.set_folded(start_line, !is_folded);
    }

    /// Check if a line is currently folded
    pub fn is_folded_at(&self, start_line: usize) -> bool {
        self.folded.iter().any(|f| f.start_line == start_line)
    }

    /// Check if a line is a fold candidate
    pub fn is_fold_candidate(&self, start_line: usize) -> bool {
        self.candidates.iter().any(|c| c.start_line == start_line)
    }

    /// Get all fold candidates
    pub fn fold_candidates(&self) -> &[FoldRange] {
        &self.candidates
    }

    /// Get all currently folded ranges
    pub fn folded_ranges(&self) -> &[FoldRange] {
        &self.folded
    }

    /// Clear all folds
    pub fn clear_folds(&mut self) {
        self.folded.clear();
    }

    /// Adjust folds and candidates after a text edit.
    ///
    /// - Folds/candidates overlapping the edited line range are removed
    /// - Folds/candidates after the edit are shifted by line_delta
    ///
    /// This avoids expensive full tree traversal on every keystroke.
    pub fn adjust_folds_for_edit(
        &mut self,
        edit_start_line: usize,
        edit_end_line: usize,
        line_delta: isize,
    ) {
        // Adjust folded ranges
        if !self.folded.is_empty() {
            self.folded.retain(|fold| {
                !(fold.start_line <= edit_end_line && fold.end_line >= edit_start_line)
            });

            if line_delta != 0 {
                for fold in &mut self.folded {
                    if fold.start_line > edit_end_line {
                        fold.start_line =
                            (fold.start_line as isize + line_delta).max(0) as usize;
                        fold.end_line = (fold.end_line as isize + line_delta).max(0) as usize;
                    }
                }
            }
        }

        // Adjust candidates the same way
        if !self.candidates.is_empty() {
            self.candidates.retain(|c| {
                !(c.start_line <= edit_end_line && c.end_line >= edit_start_line)
            });

            if line_delta != 0 {
                for c in &mut self.candidates {
                    if c.start_line > edit_end_line {
                        c.start_line = (c.start_line as isize + line_delta).max(0) as usize;
                        c.end_line = (c.end_line as isize + line_delta).max(0) as usize;
                    }
                }
            }
        }

        self.needs_rebuild = true;
    }

    /// Rebuild the fold mapping after wrap_map or fold state changes
    ///
    /// This is the core algorithm that projects wrap rows to display rows.
    pub fn rebuild(&mut self, wrap_map: &WrapMap) {
        let wrap_row_count = wrap_map.wrap_row_count();

        // Performance optimization: skip rebuild if nothing changed
        if !self.needs_rebuild && wrap_row_count == self.cached_wrap_row_count {
            return;
        }

        self.cached_wrap_row_count = wrap_row_count;

        self.visible_wrap_rows.clear();
        self.wrap_row_to_display_row = vec![None; wrap_row_count];

        if self.folded.is_empty() {
            // Fast path: no folds, all wrap rows are visible
            self.visible_wrap_rows = (0..wrap_row_count).collect();
            for (display_row, &wrap_row) in self.visible_wrap_rows.iter().enumerate() {
                self.wrap_row_to_display_row[wrap_row] = Some(display_row);
            }
            self.needs_rebuild = false;
            return;
        }

        // Build set of hidden wrap_row ranges from folded buffer lines
        let mut hidden_ranges = Vec::new();
        for fold in &self.folded {
            // Hide wrap rows from (start_line + 1) to (end_line - 1) (inclusive)
            // Both the first line and last line of the fold remain visible
            let hide_start_line = fold.start_line + 1;
            let hide_end_line = fold.end_line.saturating_sub(1);

            if hide_start_line > hide_end_line {
                continue; // No middle lines to hide (0 or 1 lines between start and end)
            }

            // Get wrap_row ranges for the hidden buffer lines
            let start_wrap_row = wrap_map.buffer_line_to_first_wrap_row(hide_start_line);
            let end_wrap_row = if hide_end_line + 1 < wrap_map.buffer_line_count() {
                wrap_map.buffer_line_to_first_wrap_row(hide_end_line + 1)
            } else {
                wrap_row_count
            };

            if start_wrap_row < end_wrap_row {
                hidden_ranges.push(start_wrap_row..end_wrap_row);
            }
        }

        // Merge overlapping hidden ranges
        hidden_ranges.sort_by_key(|r| r.start);
        let mut merged_hidden = Vec::new();
        for range in hidden_ranges {
            if let Some(last) = merged_hidden.last_mut() {
                if range.start <= *last {
                    // Overlapping or adjacent, merge
                    *last = (*last).max(range.end);
                } else {
                    merged_hidden.push(range.start);
                    merged_hidden.push(range.end);
                }
            } else {
                merged_hidden.push(range.start);
                merged_hidden.push(range.end);
            }
        }

        // Scan all wrap rows and filter out hidden ones
        let mut display_row = 0;
        let mut hidden_iter = merged_hidden.chunks_exact(2);
        let mut current_hidden = hidden_iter.next();

        for wrap_row in 0..wrap_row_count {
            // Check if wrap_row is in current hidden range
            let is_hidden = if let Some(&[start, end]) = current_hidden {
                if wrap_row >= end {
                    current_hidden = hidden_iter.next();
                    if let Some(&[new_start, new_end]) = current_hidden {
                        wrap_row >= new_start && wrap_row < new_end
                    } else {
                        false
                    }
                } else {
                    wrap_row >= start && wrap_row < end
                }
            } else {
                false
            };

            if !is_hidden {
                self.visible_wrap_rows.push(wrap_row);
                self.wrap_row_to_display_row[wrap_row] = Some(display_row);
                display_row += 1;
            }
        }

        self.needs_rebuild = false;
    }
}

// Tests omitted - requires GPUI test context setup

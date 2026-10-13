//! Caret seam (design §7.1–§7.2, minimal spike form).
//!
//! The element records, during prepaint, where every caret leaf landed and
//! how its text was shaped. Handlers (mouse, keys, IME) hit-test against the
//! last *complete* frame; paint never reaches back into the editor entity.
//!
//! This replaces the `InlineInteraction` / `editor_key` seam of design D3 for
//! the spike: the editor shapes its own leaves instead of driving `Inline`,
//! so nothing in `TextView`'s render path changes.

use std::{collections::HashMap, rc::Rc};

use gpui::{Bounds, Pixels, Point, WrappedLine, point, px, size};

use super::{id::BlockId, position::BlockPosition};

/// One hard line of a leaf: its byte offset in the leaf text and the shaped,
/// soft-wrapped layout.
pub(crate) struct LeafLine {
    pub(crate) start: usize,
    pub(crate) top: Pixels,
    pub(crate) line: WrappedLine,
}

/// Geometry of one caret leaf in the last frame.
pub(crate) struct LeafFrame {
    pub(crate) block: BlockId,
    /// Top-left of the text box, window coordinates.
    pub(crate) origin: Point<Pixels>,
    pub(crate) width: Pixels,
    pub(crate) height: Pixels,
    pub(crate) line_height: Pixels,
    /// Atomic leaves (code blocks, rules, custom nodes) only have the caret
    /// offsets 0 (before) and 1 (after).
    pub(crate) atomic: bool,
    pub(crate) lines: Vec<LeafLine>,
}

impl LeafFrame {
    fn text_len(&self) -> usize {
        self.lines
            .last()
            .map(|line| line.start + line.line.len())
            .unwrap_or(0)
    }

    /// Caret point (top of the caret) relative to `origin`.
    pub(crate) fn local_point(&self, offset: usize) -> Point<Pixels> {
        if self.atomic {
            let end = self.lines.last();
            return match (offset, end) {
                (0, _) | (_, None) => point(px(0.), px(0.)),
                (_, Some(last)) => {
                    let p = last
                        .line
                        .position_for_index(last.line.len(), self.line_height)
                        .unwrap_or_default();
                    point(p.x, last.top + p.y)
                }
            };
        }
        for (ix, line) in self.lines.iter().enumerate() {
            let end = line.start + line.line.len();
            let is_last = ix + 1 == self.lines.len();
            if offset <= end || is_last {
                let local = offset.saturating_sub(line.start).min(line.line.len());
                let p = line
                    .line
                    .position_for_index(local, self.line_height)
                    .unwrap_or_default();
                return point(p.x, line.top + p.y);
            }
        }
        point(px(0.), px(0.))
    }

    /// Closest caret offset for a point relative to `origin`.
    pub(crate) fn offset_for_local(&self, local: Point<Pixels>) -> usize {
        if self.atomic {
            return usize::from(local.x > self.width / 2.);
        }
        let Some(line) = self
            .lines
            .iter()
            .rev()
            .find(|line| local.y >= line.top)
            .or(self.lines.first())
        else {
            return 0;
        };
        let inner = point(local.x.max(px(0.)), (local.y - line.top).max(px(0.)));
        let ix = match line
            .line
            .closest_index_for_position(inner, self.line_height)
        {
            Ok(ix) | Err(ix) => ix,
        };
        (line.start + ix).min(self.text_len())
    }
}

/// Immutable per-frame layout, swapped in whole after prepaint.
#[derive(Default)]
pub(crate) struct FrameLayout {
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) leaves: Vec<Rc<LeafFrame>>,
    pub(crate) index: HashMap<BlockId, usize>,
}

impl FrameLayout {
    pub(crate) fn push(&mut self, leaf: LeafFrame) {
        self.index.insert(leaf.block, self.leaves.len());
        self.leaves.push(Rc::new(leaf));
    }

    pub(crate) fn leaf(&self, block: BlockId) -> Option<&LeafFrame> {
        self.index.get(&block).map(|ix| &*self.leaves[*ix])
    }

    /// Caret rectangle in window coordinates.
    pub(crate) fn caret_bounds(&self, pos: BlockPosition, width: Pixels) -> Option<Bounds<Pixels>> {
        let leaf = self.leaf(pos.block)?;
        let p = leaf.origin + leaf.local_point(pos.offset);
        Some(Bounds::new(p, size(width, leaf.line_height)))
    }

    /// Nearest caret position to a window point (vertical then horizontal).
    pub(crate) fn hit_test(&self, at: Point<Pixels>) -> Option<BlockPosition> {
        // Leaves are in document order and laid out top to bottom.
        let ix = self
            .leaves
            .partition_point(|leaf| leaf.origin.y + leaf.height <= at.y)
            .min(self.leaves.len().checked_sub(1)?);
        let leaf = &self.leaves[ix];
        let offset = leaf.offset_for_local(at - leaf.origin);
        Some(BlockPosition::new(leaf.block, offset))
    }

    /// The position one visual line above/below `pos`, keeping `x`.
    pub(crate) fn vertical(
        &self,
        pos: BlockPosition,
        x: Pixels,
        down: bool,
    ) -> Option<BlockPosition> {
        let leaf = self.leaf(pos.block)?;
        let local = leaf.local_point(pos.offset);
        let target_y = if down {
            leaf.origin.y + local.y + leaf.line_height * 1.5
        } else {
            leaf.origin.y + local.y - leaf.line_height * 0.5
        };
        let ix = self.index[&pos.block];
        // Inside the same leaf?
        if target_y >= leaf.origin.y && target_y < leaf.origin.y + leaf.height {
            let offset = leaf.offset_for_local(point(x - leaf.origin.x, target_y - leaf.origin.y));
            return Some(BlockPosition::new(leaf.block, offset));
        }
        let next = if down {
            self.leaves.get(ix + 1)?
        } else {
            self.leaves.get(ix.checked_sub(1)?)?
        };
        let y = if down {
            px(1.)
        } else {
            (next.height - px(1.)).max(px(0.))
        };
        let offset = next.offset_for_local(point(x - next.origin.x, y));
        Some(BlockPosition::new(next.block, offset))
    }
}

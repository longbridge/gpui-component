//! Per-frame geometry, collected through the renderer seam
//! (`text::editor_seam`, design D3).
//!
//! During the editor's prepaint every keyed text leaf reports its
//! `TextLayout` into a [`FrameSink`]; the sink also hands each leaf its
//! selection backgrounds, which `Inline` paints through its existing
//! `range_backgrounds` path. After prepaint the sink becomes the editor's
//! [`FrameLayout`], which handlers hit-test against until the next frame.

use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use gpui::{Bounds, Hsla, Pixels, Point, TextLayout, point, px, size};

use super::{id::BlockId, position::BlockPosition, projection::LeafInfo};
use crate::text::editor_seam::EditorSink;

/// Frame-local sink installed while the editor's children prepaint.
pub(crate) struct FrameSink {
    layouts: RefCell<HashMap<u64, TextLayout>>,
    backgrounds: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
}

impl FrameSink {
    pub(crate) fn new(backgrounds: HashMap<u64, Vec<(Range<usize>, Hsla)>>) -> Self {
        Self {
            layouts: RefCell::default(),
            backgrounds,
        }
    }

    pub(crate) fn into_frame(self, leaves: Rc<Vec<LeafInfo>>) -> FrameLayout {
        let layouts = self.layouts.into_inner();
        let mut frame = FrameLayout::default();
        for info in leaves.iter() {
            if let Some(layout) = layouts.get(&info.block.as_u64()) {
                frame.index.insert(info.block, frame.leaves.len());
                frame.leaves.push(LeafGeom {
                    info: *info,
                    layout: layout.clone(),
                });
            }
        }
        frame
    }
}

impl EditorSink for FrameSink {
    fn report(&self, key: u64, layout: &TextLayout) {
        self.layouts.borrow_mut().insert(key, layout.clone());
    }

    fn backgrounds(&self, key: u64) -> Vec<(Range<usize>, Hsla)> {
        self.backgrounds.get(&key).cloned().unwrap_or_default()
    }
}

pub(crate) struct LeafGeom {
    pub(crate) info: LeafInfo,
    pub(crate) layout: TextLayout,
}

impl LeafGeom {
    pub(crate) fn bounds(&self) -> Bounds<Pixels> {
        self.layout.bounds()
    }

    pub(crate) fn line_height(&self) -> Pixels {
        self.layout.line_height()
    }

    /// Rendered index for a model offset.
    fn index(&self, offset: usize) -> usize {
        if self.info.atomic {
            if offset == 0 { 0 } else { self.layout.len() }
        } else {
            offset.min(self.info.len).min(self.layout.len())
        }
    }

    /// Model offset for a rendered index.
    fn offset(&self, index: usize) -> usize {
        if self.info.atomic {
            usize::from(index * 2 > self.layout.len().max(1))
        } else {
            index.min(self.info.len)
        }
    }

    /// Top of the caret at `offset`, window coordinates.
    pub(crate) fn caret_point(&self, offset: usize) -> Point<Pixels> {
        self.layout
            .position_for_index(self.index(offset))
            .unwrap_or_else(|| self.bounds().origin)
    }

    /// Closest model offset for a window point (glyph halves round, unlike
    /// `TextLayout::index_for_position`).
    pub(crate) fn offset_at(&self, at: Point<Pixels>) -> usize {
        let bounds = self.bounds();
        let line_height = self.line_height();
        let mut top = bounds.top();
        let mut start = 0;
        let lines = self.layout.line_layouts();
        for (ix, line) in lines.iter().enumerate() {
            let height = line.size(line_height).height.max(line_height);
            let is_last = ix + 1 == lines.len();
            if at.y < top + height || is_last {
                let local = point(
                    (at.x - bounds.left()).max(px(0.)),
                    (at.y - top).clamp(px(0.), (height - px(1.)).max(px(0.))),
                );
                let index = match line.closest_index_for_position(local, line_height) {
                    Ok(ix) | Err(ix) => ix,
                };
                return self.offset(start + index);
            }
            top += height;
            start += line.len() + 1;
        }
        0
    }
}

/// The last complete frame's leaf geometry, in document order.
#[derive(Default)]
pub(crate) struct FrameLayout {
    pub(crate) leaves: Vec<LeafGeom>,
    pub(crate) index: HashMap<BlockId, usize>,
}

impl FrameLayout {
    pub(crate) fn leaf(&self, block: BlockId) -> Option<&LeafGeom> {
        self.index.get(&block).map(|ix| &self.leaves[*ix])
    }

    pub(crate) fn caret_bounds(&self, pos: BlockPosition, width: Pixels) -> Option<Bounds<Pixels>> {
        let leaf = self.leaf(pos.block)?;
        Some(Bounds::new(
            leaf.caret_point(pos.offset),
            size(width, leaf.line_height()),
        ))
    }

    /// Nearest caret position to a window point.
    pub(crate) fn hit_test(&self, at: Point<Pixels>) -> Option<BlockPosition> {
        let last = self.leaves.len().checked_sub(1)?;
        let ix = self
            .leaves
            .partition_point(|leaf| leaf.bounds().bottom() <= at.y)
            .min(last);
        let leaf = &self.leaves[ix];
        Some(BlockPosition::new(leaf.info.block, leaf.offset_at(at)))
    }

    /// One visual line above/below `pos`, keeping `x`.
    pub(crate) fn vertical(
        &self,
        pos: BlockPosition,
        x: Pixels,
        down: bool,
    ) -> Option<BlockPosition> {
        let ix = *self.index.get(&pos.block)?;
        let leaf = &self.leaves[ix];
        let caret = leaf.caret_point(pos.offset);
        let lh = leaf.line_height();
        let y = if down {
            caret.y + lh * 1.5
        } else {
            caret.y - lh * 0.5
        };
        let bounds = leaf.bounds();
        if y >= bounds.top() && y < bounds.bottom() {
            return Some(BlockPosition::new(
                leaf.info.block,
                leaf.offset_at(point(x, y)),
            ));
        }
        let next = if down {
            self.leaves.get(ix + 1)?
        } else {
            self.leaves.get(ix.checked_sub(1)?)?
        };
        let b = next.bounds();
        let y = if down {
            b.top() + next.line_height() / 2.
        } else {
            b.bottom() - next.line_height() / 2.
        };
        Some(BlockPosition::new(
            next.info.block,
            next.offset_at(point(x, y)),
        ))
    }

    /// Start or end of the visual (soft-wrapped) line holding `pos`.
    pub(crate) fn line_edge(&self, pos: BlockPosition, end: bool) -> Option<BlockPosition> {
        let leaf = self.leaf(pos.block)?;
        let caret = leaf.caret_point(pos.offset);
        let b = leaf.bounds();
        let x = if end {
            b.right() + px(10000.)
        } else {
            b.left() - px(1.)
        };
        let y = caret.y + leaf.line_height() / 2.;
        Some(BlockPosition::new(pos.block, leaf.offset_at(point(x, y))))
    }
}

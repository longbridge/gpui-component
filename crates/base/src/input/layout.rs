use std::{ops::Range, rc::Rc};

use gpui::{Bounds, Half, Pixels, ShapedLine, TextAlign, px};

use super::{WrappingIndent, display_map::LineLayout};

#[derive(Clone, Default)]
pub struct WhitespaceIndicators {
    pub space: ShapedLine,
    pub tab: ShapedLine,
}

#[derive(Clone)]
pub struct LastLayout {
    pub visible_range: Range<usize>,
    pub visible_buffer_lines: Vec<usize>,
    pub visible_line_byte_offsets: Vec<usize>,
    pub visible_top: Pixels,
    pub visible_range_offset: Range<usize>,
    pub lines: Rc<Vec<LineLayout>>,
    pub line_height: Pixels,
    pub wrap_width: Option<Pixels>,
    pub wrapping_indent: WrappingIndent,
    pub line_number_width: Pixels,
    pub cursor_bounds: Option<Bounds<Pixels>>,
    pub text_align: TextAlign,
    pub content_width: Pixels,
}

impl LastLayout {
    pub fn line(&self, row: usize) -> Option<&LineLayout> {
        let pos = self.visible_buffer_lines.binary_search(&row).ok()?;
        self.lines.get(pos)
    }

    pub fn alignment_offset(&self, line_width: Pixels) -> Pixels {
        match self.text_align {
            TextAlign::Left => px(0.),
            TextAlign::Center => (self.content_width - line_width).half().max(px(0.)),
            TextAlign::Right => (self.content_width - line_width).max(px(0.)),
        }
    }
}

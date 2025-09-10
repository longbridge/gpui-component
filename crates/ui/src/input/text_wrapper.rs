use std::ops::Range;

use gpui::{App, Font, LineFragment, Pixels};
use rope::Rope;

use crate::input::RopeExt as _;

/// A line with soft wrapped lines info.
pub(super) struct LineItem {
    /// Byte range of this line (not includes ending `\n`).
    pub(super) range: Range<usize>,
    /// The soft wrapped lines (byte range) of this line (Include first line).
    ///
    /// FIXME: Here in somecase, the `line_wrapper.wrap_line` has returned different
    /// like the `window.text_system().shape_text`. So, this value may not equal
    /// the actual rendered lines.
    pub(super) wrapped_lines: Vec<Range<usize>>,
}

impl LineItem {
    /// Return the bytes length of this line.
    pub(super) fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    /// Get the total number of lines including wrapped lines.
    #[inline]
    pub(super) fn lines_len(&self) -> usize {
        self.wrapped_lines.len()
    }

    /// Get the height of this line including wrapped lines.
    #[inline]
    pub(super) fn height(&self, line_height: Pixels) -> Pixels {
        self.lines_len() * line_height
    }
}

/// Used to prepare the text with soft wrap to be get lines to displayed in the Editor.
///
/// After use lines to calculate the scroll size of the Editor.
pub(super) struct TextWrapper {
    text: Rope,
    /// Total wrapped lines (Inlucde the first line), value is start and end index of the line.
    soft_lines: usize,
    font: Font,
    font_size: Pixels,
    /// If is none, it means the text is not wrapped
    wrap_width: Option<Pixels>,
    /// The lines by split \n
    lines: Vec<LineItem>,
}

#[allow(unused)]
impl TextWrapper {
    pub(super) fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            text: Rope::new(),
            font,
            font_size,
            wrap_width,
            soft_lines: 0,
            lines: Vec::new(),
        }
    }

    #[inline]
    pub(super) fn set_default_text(&mut self, text: &Rope) {
        self.text = text.clone();
    }

    /// Get the total number of lines including wrapped lines.
    #[inline]
    pub(super) fn len(&self) -> usize {
        self.soft_lines
    }

    #[inline]
    pub(super) fn lines(&self) -> &Vec<LineItem> {
        &self.lines
    }

    /// Get the line item by row index.
    #[inline]
    pub(super) fn line(&self, row: usize) -> Option<&LineItem> {
        self.lines.get(row)
    }

    pub(super) fn set_wrap_width(&mut self, wrap_width: Option<Pixels>, cx: &mut App) {
        if wrap_width == self.wrap_width {
            return;
        }

        self.wrap_width = wrap_width;
        self.update(&self.text.clone(), true, cx);
    }

    pub(super) fn set_font(&mut self, font: Font, font_size: Pixels, cx: &mut App) {
        if self.font.eq(&font) && self.font_size == font_size {
            return;
        }

        self.font = font;
        self.font_size = font_size;
        self.update(&self.text.clone(), true, cx);
    }

    /// Update the text wrapper and recalculate the wrapped lines.
    ///
    /// If the `text` is the same as the current text, do nothing.
    pub(super) fn update(&mut self, text: &Rope, force: bool, cx: &mut App) {
        if self.text.eq(text) && !force {
            return;
        }

        let mut soft_lines = 0;
        let mut lines = vec![];
        let wrap_width = self.wrap_width;
        let mut line_wrapper = cx
            .text_system()
            .line_wrapper(self.font.clone(), self.font_size);
        let mut prev_line_ix = 0;

        for line in text.lines() {
            let line = line.to_string();
            let mut wrapped_lines = vec![];
            let mut prev_boundary_ix = 0;

            // If wrap_width is Pixels::MAX, skip wrapping to disable word wrap
            if let Some(wrap_width) = wrap_width {
                // Here only have wrapped line, if there is no wrap meet, the `line_wraps` result will empty.
                for boundary in line_wrapper.wrap_line(&[LineFragment::text(&line)], wrap_width) {
                    wrapped_lines.push(prev_boundary_ix..boundary.ix);
                    prev_boundary_ix = boundary.ix;
                }
            }

            // Reset of the line
            if !line[prev_boundary_ix..].is_empty() || prev_boundary_ix == 0 {
                wrapped_lines.push(prev_line_ix + prev_boundary_ix..prev_line_ix + line.len());
            }

            soft_lines += wrapped_lines.len();

            lines.push(LineItem {
                range: prev_line_ix..prev_line_ix + line.len(),
                wrapped_lines,
            });

            // +1 for \n
            prev_line_ix += line.len() + 1;
        }

        self.text = text.clone();
        self.soft_lines = soft_lines;
        self.lines = lines;
    }
}

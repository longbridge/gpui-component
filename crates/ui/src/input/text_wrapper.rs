use std::ops::Range;

use gpui::{App, Font, LineFragment, Pixels};
use rope::Rope;

use crate::input::RopeExt as _;

/// A line with soft wrapped lines info.
#[derive(Clone)]
pub(super) struct LineItem {
    /// The original line text.
    line: Rope,
    /// The soft wrapped lines relative byte range (0..line.len) of this line (Include first line).
    ///
    /// FIXME: Here in somecase, the `line_wrapper.wrap_line` has returned different
    /// like the `window.text_system().shape_text`. So, this value may not equal
    /// the actual rendered lines.
    wrapped_lines: Vec<Range<usize>>,
}

impl LineItem {
    /// Get the bytes length of this line.
    #[inline]
    pub(super) fn len(&self) -> usize {
        self.line.len()
    }

    /// Get number of soft wrapped lines of this line (include the first line).
    #[inline]
    pub(super) fn lines_len(&self) -> usize {
        self.wrapped_lines.len()
    }

    /// Get the height of this line item with given line height.
    pub(super) fn height(&self, line_height: Pixels) -> Pixels {
        self.lines_len() as f32 * line_height
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
    pub(super) lines: Vec<LineItem>,
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

    /// Get the line item by row index.
    #[inline]
    pub(super) fn line(&self, row: usize) -> Option<&LineItem> {
        self.lines.iter().skip(row).next()
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

        let wrap_width = self.wrap_width;
        let mut line_wrapper = cx
            .text_system()
            .line_wrapper(self.font.clone(), self.font_size);

        self.lines.clear();
        for line in text.lines() {
            let line_str = line.to_string();
            let mut wrapped_lines = vec![];
            let mut prev_boundary_ix = 0;

            // If wrap_width is Pixels::MAX, skip wrapping to disable word wrap
            if let Some(wrap_width) = wrap_width {
                // Here only have wrapped line, if there is no wrap meet, the `line_wraps` result will empty.
                for boundary in line_wrapper.wrap_line(&[LineFragment::text(&line_str)], wrap_width)
                {
                    wrapped_lines.push(prev_boundary_ix..boundary.ix);
                    prev_boundary_ix = boundary.ix;
                }
            }

            // Reset of the line
            if !line_str[prev_boundary_ix..].is_empty() || prev_boundary_ix == 0 {
                wrapped_lines.push(prev_boundary_ix..line.len());
            }

            self.lines.push(LineItem {
                line: line.clone(),
                wrapped_lines,
            });
        }

        self.text = text.clone();
        self.soft_lines = self.lines.iter().map(|l| l.lines_len()).sum();
    }
}

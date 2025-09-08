use std::ops::Range;

use gpui::{App, Font, LineFragment, Pixels};
use rope::Rope;

use crate::input::RopeExt as _;

#[allow(unused)]
pub(super) struct LineWrap {
    /// The number of soft wrapped lines of this line (Not include first line.)
    ///
    /// FIXME: Here in somecase, the `line_wrapper.wrap_line` has returned different
    /// like the `window.text_system().shape_text`. So, this value may not equal
    /// the actual rendered lines.
    pub(super) wrap_lines: usize,
    /// The range of the line text in the entire text.
    pub(super) range: Range<usize>,
}

/// Used to prepare the text with soft wrap to be get lines to displayed in the TextArea
///
/// After use lines to calculate the scroll size of the TextArea
pub(super) struct TextWrapper {
    pub(super) text: Rope,
    /// The wrapped lines, value is start and end index of the line.
    pub(super) wrapped_lines: Vec<Range<usize>>,
    /// The lines by split \n
    pub(super) lines: Vec<LineWrap>,
    pub(super) font: Font,
    pub(super) font_size: Pixels,
    /// If is none, it means the text is not wrapped
    pub(super) wrap_width: Option<Pixels>,
}

#[allow(unused)]
impl TextWrapper {
    pub(super) fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            text: Rope::new(),
            font,
            font_size,
            wrap_width,
            wrapped_lines: Vec::new(),
            lines: Vec::new(),
        }
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

        let mut wrapped_lines = vec![];
        let mut lines = vec![];
        let wrap_width = self.wrap_width;
        let mut line_wrapper = cx
            .text_system()
            .line_wrapper(self.font.clone(), self.font_size);
        let mut prev_line_ix = 0;

        for line in text.lines() {
            let line = line.to_string();
            let mut line_wraps = vec![];
            let mut prev_boundary_ix = 0;

            // If wrap_width is Pixels::MAX, skip wrapping to disable word wrap
            if let Some(wrap_width) = wrap_width {
                // Here only have wrapped line, if there is no wrap meet, the `line_wraps` result will empty.
                for boundary in line_wrapper.wrap_line(&[LineFragment::text(&line)], wrap_width) {
                    line_wraps.push(prev_boundary_ix..boundary.ix);
                    prev_boundary_ix = boundary.ix;
                }
            }

            lines.push(LineWrap {
                wrap_lines: line_wraps.len(),
                range: prev_line_ix..prev_line_ix + line.len(),
            });

            wrapped_lines.extend(line_wraps);
            // Reset of the line
            if !line[prev_boundary_ix..].is_empty() || prev_boundary_ix == 0 {
                wrapped_lines.push(prev_line_ix + prev_boundary_ix..prev_line_ix + line.len());
            }

            // +1 for \n
            prev_line_ix += line.len() + 1;
        }

        self.text = text.clone();
        self.wrapped_lines = wrapped_lines;
        self.lines = lines;
    }
}

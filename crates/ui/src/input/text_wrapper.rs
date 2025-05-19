use std::ops::Range;

use gpui::{App, Font, LineFragment, Pixels, SharedString};

/// Used to prepare the text with soft_wrap to be get lines to displayed in the TextArea
///
/// After use lines to calculate the scroll size of the TextArea
pub(super) struct TextWrapper {
    pub(super) text: SharedString,
    /// The wrapped lines, value is start and end index of the line (by split \n).
    pub(super) wrapped_lines: Vec<Range<usize>>,
    pub(super) font: Font,
    pub(super) font_size: Pixels,
    /// If is none, it means the text is not wrapped
    pub(super) wrap_width: Option<Pixels>,
}

#[allow(unused)]
impl TextWrapper {
    pub(super) fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            text: SharedString::default(),
            font,
            font_size,
            wrap_width,
            wrapped_lines: Vec::new(),
        }
    }

    pub(super) fn set_wrap_width(&mut self, wrap_width: Option<Pixels>, cx: &mut App) {
        if self.wrap_width == wrap_width {
            return;
        }

        self.wrap_width = wrap_width;
        self.update(self.text.clone(), cx);
    }

    pub(super) fn set_font(&mut self, font: Font, cx: &mut App) {
        self.font = font;
        self.update(self.text.clone(), cx);
    }

    pub(super) fn update(&mut self, text: SharedString, cx: &mut App) {
        let mut wrapped_lines = vec![];
        let wrap_width = self.wrap_width.unwrap_or(Pixels::MAX);
        let mut line_wrapper = cx
            .text_system()
            .line_wrapper(self.font.clone(), self.font_size);

        for line in text.lines() {
            let mut prev_boundary_ix = 0;
            for boundary in line_wrapper.wrap_line(&[LineFragment::text(line)], wrap_width) {
                wrapped_lines.push(prev_boundary_ix..boundary.ix);
                prev_boundary_ix = boundary.ix;
            }

            // Reset of the line
            if !line[prev_boundary_ix..].is_empty() || prev_boundary_ix == 0 {
                wrapped_lines.push(prev_boundary_ix..line.len());
            }
        }

        // Add last empty line.
        if text.chars().last().unwrap_or('\n') == '\n' {
            wrapped_lines.push(text.len()..text.len());
        }

        self.text = text;
        self.wrapped_lines = wrapped_lines;
    }
}

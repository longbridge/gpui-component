use gpui::{HighlightStyle, TextRun, TextStyle};
use std::{ops::Range, rc::Rc};

#[derive(Debug, Clone)]
pub(crate) struct LineHighlightStyle {
    pub(crate) offset: usize,
    pub(crate) styles: Rc<Vec<(Range<usize>, HighlightStyle)>>,
}

impl LineHighlightStyle {
    pub(super) fn to_run(
        &self,
        text_style: &TextStyle,
        marked_range: &Option<Range<usize>>,
        marked_run: &TextRun,
    ) -> Vec<TextRun> {
        self.styles
            .iter()
            .map(|(range, style)| {
                let mut run = text_style.clone().highlight(*style).to_run(range.len());
                if let Some(marked_range) = marked_range {
                    if self.offset + range.start >= marked_range.start
                        && self.offset + range.end <= marked_range.end
                    {
                        run.color = marked_run.color;
                        run.strikethrough = marked_run.strikethrough;
                        run.underline = marked_run.underline;
                    }
                }
                run
            })
            // Add last `\n` Run with len 1
            .chain(std::iter::once(text_style.clone().to_run(1)))
            .collect()
    }
}

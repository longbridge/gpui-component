use gpui::{DefiniteLength, HighlightStyle};
use std::ops::Range;
use std::rc::Rc;

use super::text_wrapper::TextWrapper;
use crate::highlighter::Highlighter;

#[derive(Default, Clone)]
pub enum InputMode {
    #[default]
    SingleLine,
    MultiLine {
        rows: usize,
        height: Option<DefiniteLength>,
    },
    CodeEditor {
        /// Show line number
        line_number: bool,
        highlighter: Option<Rc<Highlighter<'static>>>,
        cache: (u64, Vec<(Range<usize>, HighlightStyle)>),
    },
    AutoGrow {
        rows: usize,
        min_rows: usize,
        max_rows: usize,
    },
}

impl InputMode {
    pub(super) fn set_rows(&mut self, new_rows: usize) {
        match self {
            InputMode::MultiLine { rows, .. } => {
                *rows = new_rows;
            }
            InputMode::AutoGrow {
                rows,
                min_rows,
                max_rows,
            } => {
                *rows = new_rows.clamp(*min_rows, *max_rows);
            }
            _ => {}
        }
    }

    pub(super) fn update_auto_grow(&mut self, text_wrapper: &TextWrapper) {
        match self {
            Self::AutoGrow { .. } => {
                let wrapped_lines = text_wrapper.wrapped_lines.len();
                self.set_rows(wrapped_lines);
            }
            _ => {}
        }
    }

    /// At least 1 row be return.
    pub(super) fn rows(&self) -> usize {
        match self {
            InputMode::MultiLine { rows, .. } => *rows,
            InputMode::AutoGrow { rows, .. } => *rows,
            _ => 1,
        }
        .max(1)
    }

    /// At least 1 row be return.
    #[allow(unused)]
    pub(super) fn min_rows(&self) -> usize {
        match self {
            InputMode::MultiLine { .. } => 1,
            InputMode::AutoGrow { min_rows, .. } => *min_rows,
            _ => 1,
        }
        .max(1)
    }

    #[allow(unused)]
    pub(super) fn max_rows(&self) -> usize {
        match self {
            InputMode::MultiLine { .. } => usize::MAX,
            InputMode::AutoGrow { max_rows, .. } => *max_rows,
            _ => 1,
        }
    }

    pub(super) fn set_height(&mut self, new_height: Option<DefiniteLength>) {
        match self {
            InputMode::MultiLine { height, .. } => {
                *height = new_height;
            }
            _ => {}
        }
    }

    pub(super) fn height(&self) -> Option<DefiniteLength> {
        match self {
            InputMode::MultiLine { height, .. } => *height,
            _ => None,
        }
    }

    pub(super) fn set_code_editor_cache(
        &mut self,
        cache: (u64, Vec<(Range<usize>, HighlightStyle)>),
    ) {
        if let InputMode::CodeEditor { cache: c, .. } = self {
            *c = cache;
        }
    }

    /// Return false if the mode is not [`InputMode::CodeEditor`].
    #[allow(unused)]
    pub(super) fn line_number(&self) -> bool {
        match self {
            InputMode::CodeEditor { line_number, .. } => *line_number,
            _ => false,
        }
    }
}

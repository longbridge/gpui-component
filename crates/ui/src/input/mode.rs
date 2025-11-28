use std::rc::Rc;
use std::{cell::RefCell, ops::Range};

use gpui::{App, SharedString};
use ropey::Rope;
use tree_sitter::InputEdit;

use super::text_wrapper::TextWrapper;
use crate::highlighter::DiagnosticSet;
use crate::highlighter::SyntaxHighlighter;
use crate::input::{RopeExt as _, TabSize};

#[derive(Clone)]
pub enum InputMode {
    Plain {
        multi_line: bool,
        tab: TabSize,
        rows: usize,
    },
    AutoGrow {
        rows: usize,
        min_rows: usize,
        max_rows: usize,
    },
    CodeEditor {
        multi_line: bool,
        tab: TabSize,
        rows: usize,
        /// Show line number
        line_number: bool,
        language: SharedString,
        indent_guides: bool,
        highlighter: Rc<RefCell<Option<SyntaxHighlighter>>>,
        diagnostics: DiagnosticSet,
    },
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Plain {
            multi_line: false,
            tab: TabSize::default(),
            rows: 1,
        }
    }
}

#[allow(unused)]
impl InputMode {
    #[inline]
    pub(super) fn is_single_line(&self) -> bool {
        !self.is_multi_line()
    }

    #[inline]
    pub(super) fn is_code_editor(&self) -> bool {
        matches!(self, InputMode::CodeEditor { .. })
    }

    #[inline]
    pub(super) fn is_auto_grow(&self) -> bool {
        matches!(self, InputMode::AutoGrow { .. })
    }

    #[inline]
    pub(super) fn is_multi_line(&self) -> bool {
        match self {
            InputMode::Plain { multi_line, .. } => *multi_line,
            InputMode::CodeEditor { multi_line, .. } => *multi_line,
            InputMode::AutoGrow { max_rows, .. } => *max_rows > 1,
        }
    }

    pub(super) fn set_rows(&mut self, new_rows: usize) {
        match self {
            InputMode::Plain { rows, .. } => {
                *rows = new_rows;
            }
            InputMode::CodeEditor { rows, .. } => {
                *rows = new_rows;
            }
            InputMode::AutoGrow {
                rows,
                min_rows,
                max_rows,
            } => {
                *rows = new_rows.clamp(*min_rows, *max_rows);
            }
        }
    }

    pub(super) fn update_auto_grow(&mut self, text_wrapper: &TextWrapper) {
        if self.is_single_line() {
            return;
        }

        let wrapped_lines = text_wrapper.len();
        self.set_rows(wrapped_lines);
    }

    /// At least 1 row be return.
    pub(super) fn rows(&self) -> usize {
        match self {
            InputMode::Plain { rows, .. } => *rows,
            InputMode::CodeEditor { rows, .. } => *rows,
            InputMode::AutoGrow { rows, .. } => *rows,
        }
        .max(1)
    }

    /// At least 1 row be return.
    #[allow(unused)]
    pub(super) fn min_rows(&self) -> usize {
        match self {
            InputMode::AutoGrow { min_rows, .. } => *min_rows,
            _ => 1,
        }
        .max(1)
    }

    #[allow(unused)]
    pub(super) fn max_rows(&self) -> usize {
        match self {
            InputMode::Plain { multi_line, .. } | InputMode::CodeEditor { multi_line, .. } => {
                match *multi_line {
                    true => usize::MAX,
                    false => 1,
                }
            }
            InputMode::AutoGrow { max_rows, .. } => *max_rows,
        }
    }

    /// Return false if the mode is not [`InputMode::CodeEditor`].
    #[allow(unused)]
    #[inline]
    pub(super) fn line_number(&self) -> bool {
        match self {
            InputMode::CodeEditor { line_number, .. } => *line_number,
            _ => false,
        }
    }

    pub(super) fn update_highlighter(
        &mut self,
        selected_range: &Range<usize>,
        text: &Rope,
        new_text: &str,
        force: bool,
        cx: &mut App,
    ) {
        match &self {
            InputMode::CodeEditor {
                language,
                highlighter,
                ..
            } => {
                if !force && highlighter.borrow().is_some() {
                    return;
                }

                let mut highlighter = highlighter.borrow_mut();
                if highlighter.is_none() {
                    let new_highlighter = SyntaxHighlighter::new(language);
                    highlighter.replace(new_highlighter);
                }

                let Some(highlighter) = highlighter.as_mut() else {
                    return;
                };

                // When full text changed, the selected_range may be out of bound (The before version).
                let mut selected_range = selected_range.clone();
                selected_range.end = selected_range.end.min(text.len());

                // If insert a chart, this is 1.
                // If backspace or delete, this is -1.
                // If selected to delete, this is the length of the selected text.
                // let changed_len = new_text.len() as isize - selected_range.len() as isize;
                let changed_len = new_text.len() as isize - selected_range.len() as isize;
                let new_end = (selected_range.end as isize + changed_len) as usize;

                let start_pos = text.offset_to_point(selected_range.start);
                let old_end_pos = text.offset_to_point(selected_range.end);
                let new_end_pos = text.offset_to_point(new_end);

                let edit = InputEdit {
                    start_byte: selected_range.start,
                    old_end_byte: selected_range.end,
                    new_end_byte: new_end,
                    start_position: start_pos,
                    old_end_position: old_end_pos,
                    new_end_position: new_end_pos,
                };

                highlighter.update(Some(edit), text);
            }
            _ => {}
        }
    }

    #[allow(unused)]
    pub(super) fn diagnostics(&self) -> Option<&DiagnosticSet> {
        match self {
            InputMode::CodeEditor { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }

    pub(super) fn diagnostics_mut(&mut self) -> Option<&mut DiagnosticSet> {
        match self {
            InputMode::CodeEditor { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }
}

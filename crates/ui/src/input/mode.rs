use std::rc::Rc;
use std::time::Duration;
use std::{cell::RefCell, ops::Range};

use gpui::{App, SharedString, Task};
use ropey::Rope;

use super::display_map::DisplayMap;
use crate::highlighter::DiagnosticSet;
use crate::highlighter::SyntaxHighlighter;
use crate::input::{InputEdit, RopeExt as _, TabSize};

#[allow(dead_code)]
pub(super) struct PendingBackgroundParse {
    pub highlighter: Rc<RefCell<Option<SyntaxHighlighter>>>,
    pub parse_task: Rc<RefCell<Option<Task<()>>>>,
    pub language: SharedString,
    pub text: Rope,
}

#[derive(Clone)]
pub(crate) enum InputMode {
    /// A plain text input mode.
    PlainText {
        multi_line: bool,
        tab: TabSize,
        rows: usize,
    },
    /// An auto grow input mode.
    AutoGrow {
        rows: usize,
        min_rows: usize,
        max_rows: usize,
    },
    /// A code editor input mode.
    CodeEditor {
        multi_line: bool,
        tab: TabSize,
        rows: usize,
        /// Show line number
        line_number: bool,
        language: SharedString,
        indent_guides: bool,
        folding: bool,
        highlighter: Rc<RefCell<Option<SyntaxHighlighter>>>,
        diagnostics: DiagnosticSet,
        parse_task: Rc<RefCell<Option<Task<()>>>>,
    },
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::plain_text()
    }
}

#[allow(unused)]
impl InputMode {
    /// Create a plain input mode with default settings.
    pub(super) fn plain_text() -> Self {
        InputMode::PlainText {
            multi_line: false,
            tab: TabSize::default(),
            rows: 1,
        }
    }

    /// Create a code editor input mode with default settings.
    pub(super) fn code_editor(language: impl Into<SharedString>) -> Self {
        InputMode::CodeEditor {
            rows: 2,
            multi_line: true,
            tab: TabSize::default(),
            language: language.into(),
            highlighter: Rc::new(RefCell::new(None)),
            line_number: true,
            indent_guides: true,
            folding: true,
            diagnostics: DiagnosticSet::new(&Rope::new()),
            parse_task: Rc::new(RefCell::new(None)),
        }
    }

    /// Create an auto grow input mode with given min and max rows.
    pub(super) fn auto_grow(min_rows: usize, max_rows: usize) -> Self {
        InputMode::AutoGrow {
            rows: min_rows,
            min_rows,
            max_rows,
        }
    }

    pub(super) fn multi_line(mut self, multi_line: bool) -> Self {
        match &mut self {
            InputMode::PlainText { multi_line: ml, .. } => *ml = multi_line,
            InputMode::CodeEditor { multi_line: ml, .. } => *ml = multi_line,
            InputMode::AutoGrow { .. } => {}
        }
        self
    }

    #[inline]
    pub(super) fn is_single_line(&self) -> bool {
        !self.is_multi_line()
    }

    #[inline]
    pub(super) fn is_code_editor(&self) -> bool {
        matches!(self, InputMode::CodeEditor { .. })
    }

    /// Return true if the mode is code editor and `folding: true`, `multi_line: true`.
    #[inline]
    pub(crate) fn is_folding(&self) -> bool {
        if cfg!(target_family = "wasm") {
            return false;
        }

        matches!(
            self,
            InputMode::CodeEditor {
                folding: true,
                multi_line: true,
                ..
            }
        )
    }

    #[inline]
    pub(super) fn is_auto_grow(&self) -> bool {
        matches!(self, InputMode::AutoGrow { .. })
    }

    #[inline]
    pub(super) fn is_multi_line(&self) -> bool {
        match self {
            InputMode::PlainText { multi_line, .. } => *multi_line,
            InputMode::CodeEditor { multi_line, .. } => *multi_line,
            InputMode::AutoGrow { max_rows, .. } => *max_rows > 1,
        }
    }

    pub(super) fn set_rows(&mut self, new_rows: usize) {
        match self {
            InputMode::PlainText { rows, .. } => {
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

    pub(super) fn update_auto_grow(&mut self, display_map: &DisplayMap) {
        if self.is_single_line() {
            return;
        }

        let wrapped_lines = display_map.wrap_row_count();
        self.set_rows(wrapped_lines);
    }

    /// At least 1 row be return.
    pub(super) fn rows(&self) -> usize {
        if !self.is_multi_line() {
            return 1;
        }

        match self {
            InputMode::PlainText { rows, .. } => *rows,
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
        if !self.is_multi_line() {
            return 1;
        }

        match self {
            InputMode::AutoGrow { max_rows, .. } => *max_rows,
            _ => usize::MAX,
        }
    }

    /// Return false if the mode is not [`InputMode::CodeEditor`].
    #[inline]
    pub(super) fn line_number(&self) -> bool {
        match self {
            InputMode::CodeEditor {
                line_number,
                multi_line,
                ..
            } => *line_number && *multi_line,
            _ => false,
        }
    }

    /// Update the syntax highlighter with new text.
    ///
    /// Returns `Some(PendingBackgroundParse)` when the synchronous parse
    /// timed out and the caller should dispatch a background parse.
    /// Returns `None` when parsing completed (or no highlighter is active).
    pub(super) fn update_highlighter(
        &mut self,
        selected_range: &Range<usize>,
        old_text: &Rope,
        new_text: &Rope,
        change_text: &str,
        force: bool,
        cx: &mut App,
    ) -> Option<PendingBackgroundParse> {
        match &self {
            InputMode::CodeEditor {
                language,
                highlighter,
                parse_task,
                ..
            } => {
                if !force && highlighter.borrow().is_some() {
                    return None;
                }

                let mut highlighter_ref = highlighter.borrow_mut();
                if highlighter_ref.is_none() {
                    let new_highlighter = SyntaxHighlighter::new(language);
                    highlighter_ref.replace(new_highlighter);
                }

                let Some(h) = highlighter_ref.as_mut() else {
                    return None;
                };

                let edit = replacement_input_edit(old_text, new_text, selected_range, change_text);

                const SYNC_PARSE_TIMEOUT: Duration = Duration::from_millis(2);
                let completed = h.update(Some(edit), new_text, Some(SYNC_PARSE_TIMEOUT));
                if completed {
                    // Sync parse succeeded, cancel any pending background parse.
                    parse_task.borrow_mut().take();
                    None
                } else {
                    // Timed out. Return the data needed for background parsing.
                    let pending = PendingBackgroundParse {
                        language: h.language().clone(),
                        text: new_text.clone(),
                        highlighter: highlighter.clone(),
                        parse_task: parse_task.clone(),
                    };
                    drop(highlighter_ref);
                    Some(pending)
                }
            }
            _ => None,
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

    /// Get a reference to the highlighter (if available)
    pub(super) fn highlighter(&self) -> Option<&Rc<RefCell<Option<SyntaxHighlighter>>>> {
        match self {
            InputMode::CodeEditor { highlighter, .. } => Some(highlighter),
            _ => None,
        }
    }
}

/// Builds the tree-sitter edit for a text replacement.
///
/// Byte offsets and positions for `start`/`old_end` come from `old_text`;
/// `new_end` byte/position come from the post-edit `text`.
fn replacement_input_edit(
    old_text: &Rope,
    new_text: &Rope,
    selected_range: &Range<usize>,
    change_text: &str,
) -> InputEdit {
    let start_byte = selected_range.start.min(old_text.len());
    let old_end_byte = selected_range.end.min(old_text.len()).max(start_byte);
    let new_end_byte = (start_byte + change_text.len()).min(new_text.len());

    InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position: old_text.offset_to_point(start_byte),
        old_end_position: old_text.offset_to_point(old_end_byte),
        new_end_position: new_text.offset_to_point(new_end_byte),
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::replacement_input_edit;
    use crate::{
        highlighter::DiagnosticSet,
        input::{Point, TabSize, mode::InputMode},
    };

    #[test]
    fn test_replacement_input_edit_backspace_at_end_uses_old_range() {
        let old_text = Rope::from_str("-=");
        let text = Rope::from_str("-");
        let edit = replacement_input_edit(&old_text, &text, &(1..2), "");

        assert_eq!(edit.start_byte, 1);
        assert_eq!(edit.old_end_byte, 2);
        assert_eq!(edit.new_end_byte, 1);
        assert_eq!(edit.start_position, Point::new(0, 1));
        assert_eq!(edit.old_end_position, Point::new(0, 2));
        assert_eq!(edit.new_end_position, Point::new(0, 1));
    }

    #[test]
    #[cfg(not(target_family = "wasm"))]
    fn test_replacement_input_edit_shifts_tree_sitter_included_ranges() {
        let old_source = "[1,2]";
        let new_source = "[1,2";
        let old_text = Rope::from_str(old_source);
        let text = Rope::from_str(new_source);
        let edit = replacement_input_edit(&old_text, &text, &(4..5), "");

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_json::LANGUAGE.into())
            .expect("JSON parser should load");
        parser
            .set_included_ranges(&[tree_sitter::Range {
                start_byte: 0,
                end_byte: old_source.len(),
                start_point: Point::new(0, 0),
                end_point: Point::new(0, old_source.len()),
            }])
            .expect("included range should be valid");

        let mut tree = parser
            .parse(old_source, None)
            .expect("old JSON should parse");
        tree.edit(&edit);
        let included_range = tree
            .included_ranges()
            .pop()
            .expect("tree should keep the included range");

        assert_eq!(included_range.end_byte, new_source.len());
        assert_eq!(included_range.end_point, Point::new(0, new_source.len()));
    }

    #[test]
    fn test_code_editor() {
        let mode = InputMode::code_editor("rust");
        assert_eq!(mode.is_code_editor(), true);
        assert_eq!(mode.is_multi_line(), true);
        assert_eq!(mode.is_single_line(), false);
        assert_eq!(mode.line_number(), true);
        assert_eq!(mode.has_indent_guides(), true);
        assert_eq!(mode.max_rows(), usize::MAX);
        assert_eq!(mode.min_rows(), 1);
        assert_eq!(mode.is_folding(), true);

        let mode = InputMode::CodeEditor {
            multi_line: false,
            line_number: true,
            indent_guides: true,
            folding: true,
            rows: 0,
            tab: Default::default(),
            language: "rust".into(),
            highlighter: Default::default(),
            diagnostics: DiagnosticSet::new(&Rope::new()),
            parse_task: Default::default(),
        };
        assert_eq!(mode.is_code_editor(), true);
        assert_eq!(mode.is_multi_line(), false);
        assert_eq!(mode.is_single_line(), true);
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.has_indent_guides(), false);
        assert_eq!(mode.max_rows(), 1);
        assert_eq!(mode.min_rows(), 1);
        assert_eq!(mode.is_folding(), false);
    }

    #[test]
    fn test_plain() {
        let mode = InputMode::PlainText {
            multi_line: true,
            tab: TabSize::default(),
            rows: 5,
        };
        assert_eq!(mode.is_code_editor(), false);
        assert_eq!(mode.is_multi_line(), true);
        assert_eq!(mode.is_single_line(), false);
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.rows(), 5);
        assert_eq!(mode.max_rows(), usize::MAX);
        assert_eq!(mode.min_rows(), 1);

        let mode = InputMode::plain_text();
        assert_eq!(mode.is_code_editor(), false);
        assert_eq!(mode.is_multi_line(), false);
        assert_eq!(mode.is_single_line(), true);
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.max_rows(), 1);
        assert_eq!(mode.min_rows(), 1);
    }

    #[test]
    fn test_auto_grow() {
        let mut mode = InputMode::auto_grow(2, 5);
        assert_eq!(mode.is_code_editor(), false);
        assert_eq!(mode.is_multi_line(), true);
        assert_eq!(mode.is_single_line(), false);
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.rows(), 2);
        assert_eq!(mode.max_rows(), 5);
        assert_eq!(mode.min_rows(), 2);

        mode.set_rows(4);
        assert_eq!(mode.rows(), 4);

        mode.set_rows(1);
        assert_eq!(mode.rows(), 2);

        mode.set_rows(10);
        assert_eq!(mode.rows(), 5);
    }
}

//! A text input field that allows the user to enter text.
//!
//! Based on the `Input` example from the `gpui` crate.
//! https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs
use serde::Deserialize;
use smallvec::SmallVec;
use std::cell::RefCell;
use std::ops::{Deref, Range};
use std::rc::Rc;
use unicode_segmentation::*;

use gpui::{
    actions, div, impl_internal_actions, point, prelude::FluentBuilder as _, px, relative, App,
    AppContext, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point,
    Render, ScrollHandle, ScrollWheelEvent, SharedString, Styled as _, Subscription,
    UTF16Selection, Window, WrappedLine,
};

// TODO:
// - Move cursor to skip line eof empty chars.

use super::{
    blink_cursor::BlinkCursor,
    change::Change,
    element::TextElement,
    mask_pattern::MaskPattern,
    mode::{InputMode, TabSize},
    number_input,
    text_wrapper::TextWrapper,
};
use crate::input::hover_popover::DiagnosticPopover;
use crate::input::marker::Marker;
use crate::{history::History, scroll::ScrollbarState, Root};

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Enter {
    /// Is confirm with secondary.
    pub secondary: bool,
}

impl_internal_actions!(input, [Enter]);

actions!(
    input,
    [
        Backspace,
        Delete,
        DeleteToBeginningOfLine,
        DeleteToEndOfLine,
        DeleteToPreviousWordStart,
        DeleteToNextWordEnd,
        Indent,
        Outdent,
        IndentInline,
        OutdentInline,
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        MoveHome,
        MoveEnd,
        MovePageUp,
        MovePageDown,
        SelectUp,
        SelectDown,
        SelectLeft,
        SelectRight,
        SelectAll,
        SelectToStartOfLine,
        SelectToEndOfLine,
        SelectToStart,
        SelectToEnd,
        SelectToPreviousWordStart,
        SelectToNextWordEnd,
        ShowCharacterPalette,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        MoveToStartOfLine,
        MoveToEndOfLine,
        MoveToStart,
        MoveToEnd,
        MoveToPreviousWord,
        MoveToNextWord,
        Escape
    ]
);

#[derive(Clone)]
pub enum InputEvent {
    Change(SharedString),
    PressEnter { secondary: bool },
    Focus,
    Blur,
}

pub(super) const CONTEXT: &str = "Input";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-backspace", DeleteToBeginningOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-delete", DeleteToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-backspace", DeleteToPreviousWordStart, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-backspace", DeleteToPreviousWordStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-delete", DeleteToNextWordEnd, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-delete", DeleteToNextWordEnd, Some(CONTEXT)),
        KeyBinding::new("enter", Enter { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("secondary-enter", Enter { secondary: true }, Some(CONTEXT)),
        KeyBinding::new("escape", Escape, Some(CONTEXT)),
        KeyBinding::new("up", MoveUp, Some(CONTEXT)),
        KeyBinding::new("down", MoveDown, Some(CONTEXT)),
        KeyBinding::new("left", MoveLeft, Some(CONTEXT)),
        KeyBinding::new("right", MoveRight, Some(CONTEXT)),
        KeyBinding::new("pageup", MovePageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", MovePageDown, Some(CONTEXT)),
        KeyBinding::new("tab", IndentInline, Some(CONTEXT)),
        KeyBinding::new("shift-tab", OutdentInline, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-]", Indent, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-]", Indent, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-[", Outdent, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-[", Outdent, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("shift-up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("shift-down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", MoveHome, Some(CONTEXT)),
        KeyBinding::new("end", MoveEnd, Some(CONTEXT)),
        KeyBinding::new("shift-home", SelectToStartOfLine, Some(CONTEXT)),
        KeyBinding::new("shift-end", SelectToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-shift-a", SelectToStartOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-shift-e", SelectToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("shift-cmd-left", SelectToStartOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("shift-cmd-right", SelectToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-left", SelectToPreviousWordStart, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-left", SelectToPreviousWordStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-right", SelectToNextWordEnd, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-right", SelectToNextWordEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-a", MoveHome, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-left", MoveHome, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-e", MoveEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-right", MoveEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", Undo, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", Redo, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-up", MoveToStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-down", MoveToEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-left", MoveToPreviousWord, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-right", MoveToNextWord, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-left", MoveToPreviousWord, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-right", MoveToNextWord, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-up", SelectToStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-down", SelectToEnd, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", Undo, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", Redo, Some(CONTEXT)),
    ]);

    number_input::init(cx);
}

#[derive(Clone)]
pub(super) struct LastLayout {
    /// The last layout lines.
    pub(super) lines: Rc<SmallVec<[WrappedLine; 1]>>,
    /// The line_height of text layout, this will change will InputElement painted.
    pub(super) line_height: Pixels,
    /// The visible range (no wrap) of lines in the viewport.
    pub(super) visible_range: Range<usize>,
}

impl Deref for LastLayout {
    type Target = Rc<SmallVec<[WrappedLine; 1]>>;

    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

/// InputState to keep editing state of the [`super::TextInput`].
pub struct InputState {
    pub(super) focus_handle: FocusHandle,
    pub(super) mode: InputMode,
    pub(super) text: SharedString,
    pub(super) text_wrapper: TextWrapper,
    pub(super) history: History<Change>,
    pub(super) blink_cursor: Entity<BlinkCursor>,
    pub(super) loading: bool,
    /// Range in UTF-8 length for the selected text.
    ///
    /// - "Hello 世界💝" = 16
    /// - "💝" = 4
    pub(super) selected_range: Range<usize>,
    /// Range for save the selected word, use to keep word range when drag move.
    pub(super) selected_word_range: Option<Range<usize>>,
    pub(super) selection_reversed: bool,
    /// The marked range is the temporary insert text on IME typing.
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) last_layout: Option<LastLayout>,
    pub(super) last_cursor_offset: Option<usize>,
    /// The input container bounds
    pub(super) input_bounds: Bounds<Pixels>,
    /// The text bounds
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    pub(super) last_selected_range: Option<Range<usize>>,
    pub(super) selecting: bool,
    pub(super) disabled: bool,
    pub(super) masked: bool,
    pub(super) clean_on_escape: bool,
    pub(super) pattern: Option<regex::Regex>,
    pub(super) validate: Option<Box<dyn Fn(&str) -> bool + 'static>>,
    pub(crate) scroll_handle: ScrollHandle,
    pub(super) scroll_state: ScrollbarState,
    /// The size of the scrollable content.
    pub(crate) scroll_size: gpui::Size<Pixels>,
    pub(crate) line_number_width: Pixels,

    /// The mask pattern for formatting the input text
    pub(crate) mask_pattern: MaskPattern,
    pub(super) placeholder: SharedString,

    /// Popover
    diagnostic_popover: Option<Entity<DiagnosticPopover>>,

    /// To remember the horizontal column (x-coordinate) of the cursor position for keep column for move up/down.
    preferred_x_offset: Option<Pixels>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<InputEvent> for InputState {}

impl InputState {
    /// Create a Input state with default [`InputMode::SingleLine`] mode.
    ///
    /// See also: [`Self::multi_line`], [`Self::auto_grow`] to set other mode.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let blink_cursor = cx.new(|_| BlinkCursor::new());
        let history = History::new().group_interval(std::time::Duration::from_secs(1));

        let _subscriptions = vec![
            // Observe the blink cursor to repaint the view when it changes.
            cx.observe(&blink_cursor, |_, _, cx| cx.notify()),
            // Blink the cursor when the window is active, pause when it's not.
            cx.observe_window_activation(window, |input, window, cx| {
                if window.is_window_active() {
                    let focus_handle = input.focus_handle.clone();
                    if focus_handle.is_focused(window) {
                        input.blink_cursor.update(cx, |blink_cursor, cx| {
                            blink_cursor.start(cx);
                        });
                    }
                }
            }),
            cx.on_focus(&focus_handle, window, Self::on_focus),
            cx.on_blur(&focus_handle, window, Self::on_blur),
        ];

        let text_style = window.text_style();

        Self {
            focus_handle: focus_handle.clone(),
            text: "".into(),
            text_wrapper: TextWrapper::new(
                text_style.font(),
                text_style.font_size.to_pixels(window.rem_size()),
                None,
            ),
            blink_cursor,
            history,
            selected_range: 0..0,
            selected_word_range: None,
            selection_reversed: false,
            marked_range: None,
            input_bounds: Bounds::default(),
            selecting: false,
            disabled: false,
            masked: false,
            clean_on_escape: false,
            loading: false,
            pattern: None,
            validate: None,
            mode: InputMode::SingleLine,
            last_layout: None,
            last_bounds: None,
            last_selected_range: None,
            last_cursor_offset: None,
            scroll_handle: ScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
            scroll_size: gpui::size(px(0.), px(0.)),
            preferred_x_offset: None,
            line_number_width: px(0.),
            placeholder: SharedString::default(),
            mask_pattern: MaskPattern::default(),
            diagnostic_popover: None,
            _subscriptions,
        }
    }

    /// Set Input to use [`InputMode::MultiLine`] mode.
    ///
    /// Default rows is 2.
    pub fn multi_line(mut self) -> Self {
        self.mode = InputMode::MultiLine {
            rows: 2,
            height: None,
            tab: TabSize::default(),
        };
        self
    }

    /// Set Input to use [`InputMode::AutoGrow`] mode with min, max rows limit.
    pub fn auto_grow(mut self, min_rows: usize, max_rows: usize) -> Self {
        self.mode = InputMode::AutoGrow {
            rows: min_rows,
            min_rows: min_rows,
            max_rows: max_rows,
        };
        self
    }

    /// Set Input to use [`InputMode::CodeEditor`] mode.
    ///
    /// Default options:
    ///
    /// - line_number: true
    /// - tab_size: 2
    /// - hard_tabs: false
    /// - height: full
    ///
    /// If `highlighter` is None, will use the default highlighter.
    ///
    /// Code Editor aim for help used to simple code editing or display, not a full-featured code editor.
    ///
    /// ## Features
    ///
    /// - Syntax Highlighting
    /// - Auto Indent
    /// - Line Number
    pub fn code_editor(mut self, language: impl Into<SharedString>) -> Self {
        let language: SharedString = language.into();
        self.mode = InputMode::CodeEditor {
            rows: 2,
            tab: TabSize::default(),
            language,
            highlighter: Rc::new(RefCell::new(None)),
            line_number: true,
            height: Some(relative(1.)),
            markers: Rc::new(vec![]),
        };
        self
    }

    /// Set placeholder
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set enable/disable line number, only for [`InputMode::CodeEditor`] mode.
    pub fn line_number(mut self, line_number: bool) -> Self {
        if let InputMode::CodeEditor { line_number: l, .. } = &mut self.mode {
            *l = line_number;
        }
        self
    }

    /// Set line number, only for [`InputMode::CodeEditor`] mode.
    pub fn set_line_number(&mut self, line_number: bool, _: &mut Window, cx: &mut Context<Self>) {
        if let InputMode::CodeEditor { line_number: l, .. } = &mut self.mode {
            *l = line_number;
        }
        cx.notify();
    }

    /// Set the tab size for the input.
    ///
    /// Only for [`InputMode::MultiLine`] and [`InputMode::CodeEditor`] mode.
    pub fn tab_size(mut self, tab: TabSize) -> Self {
        match &mut self.mode {
            InputMode::MultiLine { tab: t, .. } => *t = tab,
            InputMode::CodeEditor { tab: t, .. } => *t = tab,
            _ => {}
        }
        self
    }

    /// Set the number of rows for the multi-line Textarea.
    ///
    /// This is only used when `multi_line` is set to true.
    ///
    /// default: 2
    pub fn rows(mut self, rows: usize) -> Self {
        match &mut self.mode {
            InputMode::MultiLine { rows: r, .. } => *r = rows,
            InputMode::AutoGrow {
                max_rows: max_r,
                rows: r,
                ..
            } => {
                *r = rows;
                *max_r = rows;
            }
            _ => {}
        }
        self
    }

    /// Set highlighter language for for [`InputMode::CodeEditor`] mode.
    pub fn set_highlighter(
        &mut self,
        new_language: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        match &mut self.mode {
            InputMode::CodeEditor {
                language,
                highlighter,
                ..
            } => {
                *language = new_language.into();
                *highlighter.borrow_mut() = None;
            }
            _ => {}
        }
        cx.notify();
    }

    fn reset_highlighter(&mut self, cx: &mut Context<Self>) {
        match &mut self.mode {
            InputMode::CodeEditor { highlighter, .. } => {
                *highlighter.borrow_mut() = None;
            }
            _ => {}
        }
        cx.notify();
    }

    /// Set markers, only for [`InputMode::CodeEditor`] mode.
    ///
    /// For example to set the diagnostic markers in the code editor.
    pub fn set_markers(&mut self, markers: Vec<Marker>, _: &mut Window, _: &mut Context<Self>) {
        let mut markers = markers;
        for marker in &mut markers {
            marker.prepare(self);
        }
        self.mode.set_markers(markers);
    }

    /// Set placeholder
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    /// Called after moving the cursor. Updates preferred_x_offset if we know where the cursor now is.
    fn update_preferred_x_offset(&mut self, _cx: &mut Context<Self>) {
        let (Some(_), Some(bounds)) = (&self.last_layout, &self.last_bounds) else {
            return;
        };

        // Find which line and sub-line the cursor is on and its position
        let (_, _, cursor_pos) = self.line_and_position_for_offset(self.cursor_offset());

        if let Some(pos) = cursor_pos {
            self.preferred_x_offset = Some(pos.x + bounds.origin.x);
        }
    }

    /// Find which line and sub-line the given offset belongs to, along with the position within that sub-line.
    ///
    /// Returns:
    ///
    /// - The index of the line (zero-based) containing the offset.
    /// - The index of the sub-line (zero-based) within the line containing the offset.
    /// - The position of the offset.
    pub(super) fn line_and_position_for_offset(
        &self,
        offset: usize,
    ) -> (usize, usize, Option<Point<Pixels>>) {
        let Some(last_layout) = &self.last_layout else {
            return (0, 0, None);
        };
        let line_height = last_layout.line_height;

        let mut prev_lines_offset = 0;
        let mut y_offset = px(0.);
        for (line_index, line) in last_layout.lines.iter().enumerate() {
            let local_offset = offset.saturating_sub(prev_lines_offset);
            if let Some(pos) = line.position_for_index(local_offset, line_height) {
                let sub_line_index = (pos.y.0 / line_height.0) as usize;
                let adjusted_pos = point(pos.x, pos.y + y_offset);
                return (line_index, sub_line_index, Some(adjusted_pos));
            }

            y_offset += line.size(line_height).height;
            prev_lines_offset += line.len() + 1;
        }
        (0, 0, None)
    }

    /// Move the cursor vertically by one line (up or down) while preserving the column if possible.
    ///
    /// move_lines: Number of lines to move vertically (positive for down, negative for up).
    fn move_vertical(&mut self, move_lines: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }

        let (Some(last_layout), Some(bounds)) = (&self.last_layout, &self.last_bounds) else {
            return;
        };

        let offset = self.cursor_offset();
        let preferred_x_offset = self.preferred_x_offset;
        let line_height = last_layout.line_height;
        let (current_line, current_sub_line, current_pos) =
            self.line_and_position_for_offset(offset);

        let Some(current_pos) = current_pos else {
            return;
        };

        let current_x = self
            .preferred_x_offset
            .unwrap_or_else(|| current_pos.x + bounds.origin.x);

        let mut new_line = current_line;
        let mut new_sub_line = current_sub_line as i32;
        new_sub_line += if move_lines > 0 { 1 } else { -1 };

        // Handle moving above the first line
        if move_lines < 0 && current_line == 0 && new_sub_line < 0 {
            // Move cursor to the beginning of the text
            self.move_to(0, window, cx);
            self.preferred_x_offset = preferred_x_offset;
            return;
        }

        if new_sub_line < 0 {
            new_line = new_line
                .saturating_add_signed(move_lines)
                .max(0)
                .min(last_layout.lines.len().saturating_sub(1));
            new_sub_line = last_layout.lines[new_line].wrap_boundaries.len() as i32;
        } else {
            let max_sub_line = last_layout.lines[current_line].wrap_boundaries.len() as i32;
            if new_sub_line > max_sub_line {
                if new_line < last_layout.lines.len() - 1 {
                    new_line = new_line
                        .saturating_add_signed(move_lines)
                        .max(0)
                        .min(last_layout.lines.len().saturating_sub(1));
                    new_sub_line = 0;
                } else {
                    new_sub_line = max_sub_line;
                }
            }
        }

        // If after adjustment, still at the same position, do not proceed
        if new_line == current_line && new_sub_line == current_sub_line as i32 {
            return;
        }

        let target_line = &last_layout.lines[new_line];
        let line_x = current_x - bounds.origin.x;
        let target_sub_line = new_sub_line as usize;

        let approx_pos = point(line_x, px(target_sub_line as f32 * line_height.0));
        let index_res = target_line.index_for_position(approx_pos, line_height);

        let new_local_index = match index_res {
            Ok(i) => i,
            Err(i) => i,
        };

        let mut prev_lines_offset = 0;
        for (_, line) in last_layout
            .lines
            .iter()
            .enumerate()
            .take_while(|(i, _)| *i < new_line)
        {
            prev_lines_offset += line.len() + 1;
        }

        let new_offset = (prev_lines_offset + new_local_index).min(self.text.len());
        self.selected_range = new_offset..new_offset;
        self.pause_blink_cursor(cx);
        // Set back the preferred_x_offset
        self.preferred_x_offset = preferred_x_offset;
        cx.notify();
    }

    #[inline]
    pub(super) fn is_multi_line(&self) -> bool {
        matches!(
            self.mode,
            InputMode::MultiLine { .. } | InputMode::AutoGrow { .. } | InputMode::CodeEditor { .. }
        )
    }

    #[inline]
    pub(super) fn is_single_line(&self) -> bool {
        matches!(self.mode, InputMode::SingleLine)
    }

    #[inline]
    pub(super) fn is_auto_grow(&self) -> bool {
        matches!(self.mode, InputMode::AutoGrow { .. })
    }

    /// Set the text of the input field.
    ///
    /// And the selection_range will be reset to 0..0.
    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.history.ignore = true;
        self.replace_text(value, window, cx);
        self.history.ignore = false;
        // Ensure cursor to start when set text
        if self.is_single_line() {
            self.selected_range = self.text.len()..self.text.len();
        } else {
            self.selected_range = 0..0;
        }
        // Move scroll to top
        self.scroll_handle.set_offset(point(px(0.), px(0.)));

        cx.notify();
    }

    /// Insert text at the current cursor position.
    ///
    /// And the cursor will be moved to the end of inserted text.
    pub fn insert(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text: SharedString = text.into();
        let range_utf16 = self.range_to_utf16(&(self.cursor_offset()..self.cursor_offset()));
        self.replace_text_in_range(Some(range_utf16), &text, window, cx);
        self.selected_range = self.selected_range.end..self.selected_range.end;
    }

    /// Replace text at the current cursor position.
    ///
    /// And the cursor will be moved to the end of replaced text.
    pub fn replace(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text: SharedString = text.into();
        self.replace_text_in_range(None, &text, window, cx);
        self.selected_range = self.selected_range.end..self.selected_range.end;
    }

    fn replace_text(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text: SharedString = text.into();
        let range = 0..self.text.chars().map(|c| c.len_utf16()).sum();
        self.replace_text_in_range(Some(range), &text, window, cx);
        self.reset_highlighter(cx);
    }

    /// Set with disabled mode.
    ///
    /// See also: [`Self::set_disabled`], [`Self::is_disabled`].
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the disabled state of the input field.
    ///
    /// See also: [`Self::disabled`], [`Self::is_disabled`].
    pub fn set_disabled(&mut self, disabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }

    /// Return is the input field is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Set with password masked state.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Set the password masked state of the input field.
    pub fn set_masked(&mut self, masked: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.masked = masked;
        cx.notify();
    }

    /// Set true to clear the input by pressing Escape key.
    pub fn clean_on_escape(mut self) -> Self {
        self.clean_on_escape = true;
        self
    }

    /// Set the regular expression pattern of the input field.
    pub fn pattern(mut self, pattern: regex::Regex) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Set the regular expression pattern of the input field with reference.
    pub fn set_pattern(
        &mut self,
        pattern: regex::Regex,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.pattern = Some(pattern);
    }

    /// Set the validation function of the input field.
    pub fn validate(mut self, f: impl Fn(&str) -> bool + 'static) -> Self {
        self.validate = Some(Box::new(f));
        self
    }

    /// Set true to show indicator at the input right.
    pub fn set_loading(&mut self, loading: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Set the default value of the input field.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.text = value.into();
        self.text_wrapper.text = self.text.clone();
        self
    }

    /// Return the value of the input field.
    pub fn value(&self) -> &SharedString {
        &self.text
    }

    /// Return the value without mask.
    pub fn unmask_value(&self) -> SharedString {
        self.mask_pattern.unmask(&self.text).into()
    }

    /// Focus the input field.
    pub fn focus(&self, window: &mut Window, _: &mut Context<Self>) {
        self.focus_handle.focus(window);
    }

    pub(super) fn left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), window, cx);
        } else {
            self.move_to(self.selected_range.start, window, cx)
        }
    }

    pub(super) fn right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), window, cx);
        } else {
            self.move_to(self.selected_range.end, window, cx)
        }
    }

    pub(super) fn up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }

        if !self.selected_range.is_empty() {
            self.move_to(
                self.previous_boundary(self.selected_range.start.saturating_sub(1)),
                window,
                cx,
            );
        }
        self.pause_blink_cursor(cx);
        self.move_vertical(-1, window, cx);
    }

    pub(super) fn down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }

        if !self.selected_range.is_empty() {
            self.move_to(
                self.next_boundary(self.selected_range.end.saturating_sub(1)),
                window,
                cx,
            );
        }

        self.pause_blink_cursor(cx);
        self.move_vertical(1, window, cx);
    }

    pub(super) fn page_up(&mut self, _: &MovePageUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }

        let Some(last_layout) = &self.last_layout else {
            return;
        };

        let display_lines = (self.input_bounds.size.height / last_layout.line_height) as isize;
        self.move_vertical(-display_lines, window, cx);
    }

    pub(super) fn page_down(
        &mut self,
        _: &MovePageDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_single_line() {
            return;
        }

        let Some(last_layout) = &self.last_layout else {
            return;
        };

        let display_lines = (self.input_bounds.size.height / last_layout.line_height) as isize;
        self.move_vertical(display_lines, window, cx);
    }

    pub(super) fn select_left(
        &mut self,
        _: &SelectLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.previous_boundary(self.cursor_offset()), window, cx);
    }

    pub(super) fn select_right(
        &mut self,
        _: &SelectRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.next_boundary(self.cursor_offset()), window, cx);
    }

    pub(super) fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }
        let offset = self.start_of_line(window, cx).saturating_sub(1);
        self.select_to(self.previous_boundary(offset), window, cx);
    }

    pub(super) fn select_down(
        &mut self,
        _: &SelectDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_single_line() {
            return;
        }
        let offset = (self.end_of_line(window, cx) + 1).min(self.text.len());
        self.select_to(self.next_boundary(offset), window, cx);
    }

    pub(super) fn select_all(
        &mut self,
        _: &SelectAll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_to(0, window, cx);
        self.select_to(self.text.len(), window, cx)
    }

    pub(super) fn home(&mut self, _: &MoveHome, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        let offset = self.start_of_line(window, cx);
        self.move_to(offset, window, cx);
    }

    pub(super) fn end(&mut self, _: &MoveEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        let offset = self.end_of_line(window, cx);
        self.move_to(offset, window, cx);
    }

    pub(super) fn move_to_start(
        &mut self,
        _: &MoveToStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_to(0, window, cx);
    }

    pub(super) fn move_to_end(
        &mut self,
        _: &MoveToEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let end = self.text.len();
        self.move_to(end, window, cx);
    }

    pub(super) fn move_to_previous_word(
        &mut self,
        _: &MoveToPreviousWord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.previous_start_of_word();
        self.move_to(offset, window, cx);
    }

    pub(super) fn move_to_next_word(
        &mut self,
        _: &MoveToNextWord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.next_end_of_word();
        self.move_to(offset, window, cx);
    }

    pub(super) fn select_to_start(
        &mut self,
        _: &SelectToStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(0, window, cx);
    }

    pub(super) fn select_to_end(
        &mut self,
        _: &SelectToEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let end = self.text.len();
        self.select_to(end, window, cx);
    }

    pub(super) fn select_to_start_of_line(
        &mut self,
        _: &SelectToStartOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.start_of_line(window, cx);
        self.select_to(self.previous_boundary(offset), window, cx);
    }

    pub(super) fn select_to_end_of_line(
        &mut self,
        _: &SelectToEndOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.end_of_line(window, cx);
        self.select_to(self.next_boundary(offset), window, cx);
    }

    pub(super) fn select_to_previous_word(
        &mut self,
        _: &SelectToPreviousWordStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.previous_start_of_word();
        self.select_to(offset, window, cx);
    }

    pub(super) fn select_to_next_word(
        &mut self,
        _: &SelectToNextWordEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.next_end_of_word();
        self.select_to(offset, window, cx);
    }

    /// Return the start offset of the previous word.
    fn previous_start_of_word(&mut self) -> usize {
        let offset = self.selected_range.start;
        let prev_str = self.text_for_range_utf8(0..offset);
        UnicodeSegmentation::split_word_bound_indices(prev_str)
            .filter(|(_, s)| !s.trim_start().is_empty())
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Return the next end offset of the next word.
    fn next_end_of_word(&mut self) -> usize {
        let offset = self.cursor_offset();
        let next_str = self.text_for_range_utf8(offset..self.text.len());
        UnicodeSegmentation::split_word_bound_indices(next_str)
            .find(|(_, s)| !s.trim_start().is_empty())
            .map(|(i, s)| offset + i + s.len())
            .unwrap_or(self.text.len())
    }

    /// Get start of line
    fn start_of_line(&mut self, window: &mut Window, cx: &mut Context<Self>) -> usize {
        if self.is_single_line() {
            return 0;
        }

        let offset = self.previous_boundary(self.cursor_offset());
        let line = self
            .text_for_range(self.range_to_utf16(&(0..offset + 1)), &mut None, window, cx)
            .unwrap_or_default()
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        line
    }

    /// Get start line of selection start or end (The min value).
    ///
    /// This is means is always get the first line of selection.
    fn start_of_line_of_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) -> usize {
        if self.is_single_line() {
            return 0;
        }

        let mut offset =
            self.previous_boundary(self.selected_range.start.min(self.selected_range.end));
        if self.text.chars().nth(offset) == Some('\r') {
            offset += 1;
        }

        let line = self
            .text_for_range(self.range_to_utf16(&(0..offset + 1)), &mut None, window, cx)
            .unwrap_or_default()
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        line
    }

    /// Get end of line
    fn end_of_line(&mut self, window: &mut Window, cx: &mut Context<Self>) -> usize {
        if self.is_single_line() {
            return self.text.len();
        }

        let offset = self.next_boundary(self.cursor_offset());
        // ignore if offset is "\n"
        if self
            .text_for_range(
                self.range_to_utf16(&(offset.saturating_sub(1)..offset)),
                &mut None,
                window,
                cx,
            )
            .unwrap_or_default()
            .eq("\n")
        {
            return offset;
        }

        let line = self
            .text_for_range(
                self.range_to_utf16(&(offset..self.text.len())),
                &mut None,
                window,
                cx,
            )
            .unwrap_or_default()
            .find('\n')
            .map(|i| i + offset)
            .unwrap_or(self.text.len());
        line
    }

    /// Get indent string of next line.
    ///
    /// To get current and next line indent, to return more depth one.
    pub(super) fn indent_of_next_line(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> String {
        if self.is_single_line() {
            return "".into();
        }

        let mut current_indent = String::new();
        let mut next_indent = String::new();
        let current_line_start_pos = self.start_of_line(window, cx);
        let next_line_start_pos = self.end_of_line(window, cx);
        for c in self.text.chars().skip(current_line_start_pos) {
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' || c == '\r' {
                break;
            }
            current_indent.push(c);
        }

        for c in self.text.chars().skip(next_line_start_pos) {
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' || c == '\r' {
                break;
            }
            next_indent.push(c);
        }

        if next_indent.len() > current_indent.len() {
            return next_indent;
        } else {
            return current_indent;
        }
    }

    pub(super) fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), window, cx)
        }
        self.replace_text_in_range(None, "", window, cx);
        self.pause_blink_cursor(cx);
    }

    pub(super) fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), window, cx)
        }
        self.replace_text_in_range(None, "", window, cx);
        self.pause_blink_cursor(cx);
    }

    pub(super) fn delete_to_beginning_of_line(
        &mut self,
        _: &DeleteToBeginningOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut offset = self.start_of_line(window, cx);
        if offset == self.cursor_offset() {
            offset = offset.saturating_sub(1);
        }
        self.replace_text_in_range(
            Some(self.range_to_utf16(&(offset..self.cursor_offset()))),
            "",
            window,
            cx,
        );

        self.pause_blink_cursor(cx);
    }

    pub(super) fn delete_to_end_of_line(
        &mut self,
        _: &DeleteToEndOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut offset = self.end_of_line(window, cx);
        if offset == self.cursor_offset() {
            offset = (offset + 1).clamp(0, self.text.len());
        }
        self.replace_text_in_range(
            Some(self.range_to_utf16(&(self.cursor_offset()..offset))),
            "",
            window,
            cx,
        );
        self.pause_blink_cursor(cx);
    }

    pub(super) fn delete_previous_word(
        &mut self,
        _: &DeleteToPreviousWordStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.previous_start_of_word();
        self.replace_text_in_range(
            Some(self.range_to_utf16(&(offset..self.cursor_offset()))),
            "",
            window,
            cx,
        );
        self.pause_blink_cursor(cx);
    }

    pub(super) fn delete_next_word(
        &mut self,
        _: &DeleteToNextWordEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.next_end_of_word();
        self.replace_text_in_range(
            Some(self.range_to_utf16(&(self.cursor_offset()..offset))),
            "",
            window,
            cx,
        );
        self.pause_blink_cursor(cx);
    }

    pub(super) fn enter(&mut self, action: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_multi_line() {
            // Get current line indent
            let indent = if self.mode.is_code_editor() {
                self.indent_of_next_line(window, cx)
            } else {
                "".to_string()
            };

            // Add newline and indent
            let new_line_text = format!("\n{}", indent);
            self.replace_text_in_range(None, &new_line_text, window, cx);
        }

        cx.emit(InputEvent::PressEnter {
            secondary: action.secondary,
        });
    }

    pub(super) fn indent_inline(
        &mut self,
        _: &IndentInline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.indent(false, window, cx);
    }

    pub(super) fn indent_block(&mut self, _: &Indent, window: &mut Window, cx: &mut Context<Self>) {
        self.indent(true, window, cx);
    }

    pub(super) fn outdent_inline(
        &mut self,
        _: &OutdentInline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.outdent(false, window, cx);
    }

    pub(super) fn outdent_block(
        &mut self,
        _: &Outdent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.outdent(true, window, cx);
    }

    pub(super) fn indent(&mut self, block: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab_size) = self.mode.tab_size() else {
            cx.propagate();
            return;
        };

        let tab_indent = tab_size.to_string();
        let selected_range = self.selected_range.clone();
        let mut added_len = 0;
        let is_selected = !self.selected_range.is_empty();

        if is_selected || block {
            let start_offset = self.start_of_line_of_selection(window, cx);
            let mut offset = start_offset;

            let selected_text = self
                .text_for_range(
                    self.range_to_utf16(&(offset..selected_range.end)),
                    &mut None,
                    window,
                    cx,
                )
                .unwrap_or("".into());

            for line in selected_text.split('\n') {
                self.replace_text_in_range(
                    Some(self.range_to_utf16(&(offset..offset))),
                    &tab_indent,
                    window,
                    cx,
                );
                added_len += tab_indent.len();
                // +1 for "\n", the `\r` is included in the `line`.
                offset += line.len() + tab_indent.len() + 1;
            }

            if is_selected {
                self.selected_range = start_offset..selected_range.end + added_len;
            } else {
                self.selected_range =
                    selected_range.start + added_len..selected_range.end + added_len;
            }
        } else {
            // Selected none
            let offset = self.selected_range.start;
            self.replace_text_in_range(
                Some(self.range_to_utf16(&(offset..offset))),
                &tab_indent,
                window,
                cx,
            );
            added_len = tab_indent.len();

            self.selected_range = selected_range.start + added_len..selected_range.end + added_len;
        }
    }

    pub(super) fn outdent(&mut self, block: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab_size) = self.mode.tab_size() else {
            cx.propagate();
            return;
        };

        let tab_indent = tab_size.to_string();
        let selected_range = self.selected_range.clone();
        let mut removed_len = 0;
        let is_selected = !self.selected_range.is_empty();

        if is_selected || block {
            let start_offset = self.start_of_line_of_selection(window, cx);
            let mut offset = start_offset;

            let selected_text = self
                .text_for_range(
                    self.range_to_utf16(&(offset..selected_range.end)),
                    &mut None,
                    window,
                    cx,
                )
                .unwrap_or("".into());

            for line in selected_text.split('\n') {
                if line.starts_with(tab_indent.as_ref()) {
                    self.replace_text_in_range(
                        Some(self.range_to_utf16(&(offset..offset + tab_indent.len()))),
                        "",
                        window,
                        cx,
                    );
                    removed_len += tab_indent.len();

                    // +1 for "\n"
                    offset += line.len().saturating_sub(tab_indent.len()) + 1;
                } else {
                    offset += line.len() + 1;
                }
            }

            if is_selected {
                self.selected_range = start_offset..selected_range.end.saturating_sub(removed_len);
            } else {
                self.selected_range = selected_range.start.saturating_sub(removed_len)
                    ..selected_range.end.saturating_sub(removed_len);
            }
        } else {
            // Selected none
            let start_offset = self.selected_range.start;
            let offset = self.start_of_line_of_selection(window, cx);
            if self
                .text_for_range_utf8(offset..self.text.len())
                .starts_with(tab_indent.as_ref())
            {
                self.replace_text_in_range(
                    Some(self.range_to_utf16(&(offset..offset + tab_indent.len()))),
                    "",
                    window,
                    cx,
                );
                removed_len = tab_indent.len();
                let new_offset = start_offset.saturating_sub(removed_len);
                self.selected_range = new_offset..new_offset;
            }
        }
    }

    pub(super) fn clean(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text("", window, cx);
    }

    pub(super) fn escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.marked_range.is_some() {
            self.unmark_text(window, cx);
        }
        if self.selected_range.len() > 0 {
            return self.unselect(window, cx);
        }

        if self.clean_on_escape {
            return self.clean(window, cx);
        }

        cx.propagate();
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // If there have IME marked range and is empty (Means pressed Esc to abort IME typing)
        // Clear the marked range.
        if let Some(marked_range) = &self.marked_range {
            if marked_range.len() == 0 {
                self.marked_range = None;
            }
        }

        self.selecting = true;
        let offset = self.index_for_mouse_position(event.position, window, cx);
        // Double click to select word
        if event.button == MouseButton::Left && event.click_count == 2 {
            self.select_word(offset, window, cx);
            return;
        }

        if event.modifiers.shift {
            self.select_to(offset, window, cx);
        } else {
            self.move_to(offset, window, cx)
        }
    }

    pub(super) fn on_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.selecting = false;
        self.selected_word_range = None;
    }

    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.index_for_mouse_position(event.position, window, cx);
        if let Some(marker) = self.mode.marker_for_offset(offset) {
            if let Some(diagnostic_popover) = self.diagnostic_popover.as_ref() {
                if diagnostic_popover.read(cx).marker.range == marker.range {
                    diagnostic_popover.update(cx, |this, cx| {
                        this.show(cx);
                    });

                    return;
                }
            }

            self.diagnostic_popover = Some(DiagnosticPopover::new(marker, cx.entity(), cx));
            cx.notify();
        } else {
            if let Some(diagnostic_popover) = self.diagnostic_popover.as_mut() {
                diagnostic_popover.update(cx, |this, cx| {
                    this.check_to_hide(event.position, cx);
                })
            }
        }
    }

    pub(super) fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_height = self
            .last_layout
            .as_ref()
            .map(|layout| layout.line_height)
            .unwrap_or(window.line_height());
        let delta = event.delta.pixel_delta(line_height);
        self.update_scroll_offset(Some(self.scroll_handle.offset() + delta), cx);
        self.diagnostic_popover = None;
    }

    fn update_scroll_offset(&mut self, offset: Option<Point<Pixels>>, cx: &mut Context<Self>) {
        let mut offset = offset.unwrap_or(self.scroll_handle.offset());

        let safe_y_range =
            (-self.scroll_size.height + self.input_bounds.size.height).min(px(0.0))..px(0.);
        let safe_x_range =
            (-self.scroll_size.width + self.input_bounds.size.width).min(px(0.0))..px(0.);

        offset.y = offset.y.clamp(safe_y_range.start, safe_y_range.end);
        offset.x = offset.x.clamp(safe_x_range.start, safe_x_range.end);
        self.scroll_handle.set_offset(offset);
        cx.notify();
    }

    pub(super) fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    pub(super) fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }

        let selected_text = self
            .text_for_range_utf8(self.selected_range.clone())
            .to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
    }

    pub(super) fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }

        let selected_text = self
            .text_for_range_utf8(self.selected_range.clone())
            .to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
        self.replace_text_in_range(None, "", window, cx);
    }

    pub(super) fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(clipboard) = cx.read_from_clipboard() {
            let mut new_text = clipboard.text().unwrap_or_default();
            if !self.is_multi_line() {
                new_text = new_text.replace('\n', "");
            }

            self.replace_text_in_range(None, &new_text, window, cx);
        }
    }

    fn push_history(
        &mut self,
        range: &Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.history.ignore {
            return;
        }

        let old_text = self
            .text_for_range(self.range_to_utf16(&range), &mut None, window, cx)
            .unwrap_or("".to_string());

        let new_range = range.start..range.start + new_text.len();

        self.history.push(Change::new(
            range.clone(),
            &old_text,
            new_range.clone(),
            new_text,
        ));
    }

    pub(super) fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;
        if let Some(changes) = self.history.undo() {
            for change in changes {
                let range_utf16 = self.range_to_utf16(&change.new_range);
                self.replace_text_in_range(Some(range_utf16), &change.old_text, window, cx);
            }
        }
        self.history.ignore = false;
    }

    pub(super) fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;
        if let Some(changes) = self.history.redo() {
            for change in changes {
                let range_utf16 = self.range_to_utf16(&change.old_range);
                self.replace_text_in_range(Some(range_utf16), &change.new_text, window, cx);
            }
        }
        self.history.ignore = false;
    }

    /// Move the cursor to the given offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// Ensure the offset use self.next_boundary or self.previous_boundary to get the correct offset.
    fn move_to(&mut self, offset: usize, _: &mut Window, cx: &mut Context<Self>) {
        let offset = offset.clamp(0, self.text.len());
        self.selected_range = offset..offset;
        self.pause_blink_cursor(cx);
        self.update_preferred_x_offset(cx);
        cx.notify()
    }

    pub(super) fn cursor_offset(&self) -> usize {
        if let Some(marked_range) = &self.marked_range {
            return marked_range.end;
        }

        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(
        &self,
        position: Point<Pixels>,
        _window: &Window,
        _cx: &App,
    ) -> usize {
        // If the text is empty, always return 0
        if self.text.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(last_layout)) =
            (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };

        let line_height = last_layout.line_height;

        // TIP: About the IBeam cursor
        //
        // If cursor style is IBeam, the mouse mouse position is in the middle of the cursor (This is special in OS)

        // The position is relative to the bounds of the text input
        //
        // bounds.origin:
        //
        // - included the input padding.
        // - included the scroll offset.
        let inner_position = position - bounds.origin - point(self.line_number_width, px(0.));

        let mut index = 0;
        let mut y_offset = px(0.);

        for (_, line) in last_layout.lines.iter().enumerate() {
            let line_origin = self.line_origin_with_y_offset(&mut y_offset, &line, line_height);
            let pos = inner_position - line_origin;

            // Return offset by use closest_index_for_x if is single line mode.
            if self.is_single_line() {
                return line.unwrapped_layout.closest_index_for_x(pos.x);
            }

            let index_result = line.closest_index_for_position(pos, line_height);
            if let Ok(v) = index_result {
                index += v;
                break;
            } else if let Ok(_) = line.index_for_position(point(px(0.), pos.y), line_height) {
                // Click in the this line but not in the text, move cursor to the end of the line.
                // The fallback index is saved in Err from `index_for_position` method.
                index += index_result.unwrap_err();
                break;
            } else if line.text.trim_end_matches(|c| c == '\r').len() == 0 {
                // empty line on Windows is `\r`, other is ''
                let line_bounds = Bounds {
                    origin: line_origin,
                    size: gpui::size(bounds.size.width, line_height),
                };
                let pos = inner_position;
                index += line.len();
                if line_bounds.contains(&pos) {
                    break;
                }
            } else {
                index += line.len();
            }

            // +1 for revert `lines` split `\n`
            index += 1;
        }

        if index > self.text.len() {
            self.text.len()
        } else {
            index
        }
    }

    /// Returns a y offsetted point for the line origin.
    fn line_origin_with_y_offset(
        &self,
        y_offset: &mut Pixels,
        line: &WrappedLine,
        line_height: Pixels,
    ) -> Point<Pixels> {
        // NOTE: About line.wrap_boundaries.len()
        //
        // If only 1 line, the value is 0
        // If have 2 line, the value is 1
        if self.is_multi_line() {
            let p = point(px(0.), *y_offset);
            let height = line_height + line.wrap_boundaries.len() as f32 * line_height;
            *y_offset = *y_offset + height;
            p
        } else {
            point(px(0.), px(0.))
        }
    }

    /// Select the text from the current cursor position to the given offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// Ensure the offset use self.next_boundary or self.previous_boundary to get the correct offset.
    fn select_to(&mut self, offset: usize, _: &mut Window, cx: &mut Context<Self>) {
        let offset = offset.clamp(0, self.text.len());
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };

        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }

        // Ensure keep word selected range
        if let Some(word_range) = self.selected_word_range.as_ref() {
            if self.selected_range.start > word_range.start {
                self.selected_range.start = word_range.start;
            }
            if self.selected_range.end < word_range.end {
                self.selected_range.end = word_range.end;
            }
        }
        if self.selected_range.is_empty() {
            self.update_preferred_x_offset(cx);
        }
        cx.notify()
    }

    /// Select the word at the given offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// FIXME: When click on a non-word character, the word is not selected.
    fn select_word(&mut self, offset: usize, window: &mut Window, cx: &mut Context<Self>) {
        #[inline(always)]
        fn is_word(c: char) -> bool {
            c.is_alphanumeric() || matches!(c, '_')
        }

        let mut start = offset;
        let mut end = start;
        let prev_text = self
            .text_for_range(self.range_to_utf16(&(0..start)), &mut None, window, cx)
            .unwrap_or_default();
        let next_text = self
            .text_for_range(
                self.range_to_utf16(&(end..self.text.len())),
                &mut None,
                window,
                cx,
            )
            .unwrap_or_default();

        let prev_chars = prev_text.chars().rev();
        let next_chars = next_text.chars();

        let pre_chars_count = prev_chars.clone().count();
        for (ix, c) in prev_chars.enumerate() {
            if !is_word(c) {
                break;
            }

            if ix < pre_chars_count {
                start = start.saturating_sub(c.len_utf8());
            }
        }

        for (_, c) in next_chars.enumerate() {
            if !is_word(c) {
                break;
            }

            end += c.len_utf8();
        }

        if start == end {
            return;
        }

        self.selected_range = start..end;
        self.selected_word_range = Some(self.selected_range.clone());
        cx.notify()
    }

    fn unselect(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.next_boundary(self.cursor_offset());
        self.selected_range = offset..offset;
        cx.notify()
    }

    pub(super) fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.text.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    pub(super) fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.text.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    pub(super) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(super) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.text.len())
    }

    /// Returns the true to let InputElement to render cursor, when Input is focused and current BlinkCursor is visible.
    pub(crate) fn show_cursor(&self, window: &Window, cx: &App) -> bool {
        self.focus_handle.is_focused(window) && self.blink_cursor.read(cx).visible()
    }

    fn on_focus(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.start(cx);
        });
        cx.emit(InputEvent::Focus);
    }

    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.unselect(window, cx);
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.stop(cx);
        });
        Root::update(window, cx, |root, _, _| {
            root.focused_input = None;
        });
        cx.emit(InputEvent::Blur);
    }

    fn pause_blink_cursor(&mut self, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.pause(cx);
        });
    }

    pub(super) fn on_key_down(&mut self, _: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
    }

    pub(super) fn on_drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text.is_empty() {
            return;
        }

        if self.last_layout.is_none() {
            return;
        }

        if !self.focus_handle.is_focused(window) {
            return;
        }

        if !self.selecting {
            return;
        }

        let offset = self.index_for_mouse_position(event.position, window, cx);
        self.select_to(offset, window, cx);
    }

    fn is_valid_input(&self, new_text: &str) -> bool {
        if new_text.is_empty() {
            return true;
        }

        if let Some(validate) = &self.validate {
            if !validate(new_text) {
                return false;
            }
        }

        if !self.mask_pattern.is_valid(new_text) {
            return false;
        }

        self.pattern
            .as_ref()
            .map(|p| p.is_match(new_text))
            .unwrap_or(true)
    }

    /// Set the mask pattern for formatting the input text.
    ///
    /// The pattern can contain:
    /// - 9: Any digit or dot
    /// - A: Any letter
    /// - *: Any character
    /// - Other characters will be treated as literal mask characters
    ///
    /// Example: "(999)999-999" for phone numbers
    pub fn mask_pattern(mut self, pattern: impl Into<MaskPattern>) -> Self {
        self.mask_pattern = pattern.into();
        if let Some(placeholder) = self.mask_pattern.placeholder() {
            self.placeholder = placeholder.into();
        }
        self
    }

    pub fn set_mask_pattern(
        &mut self,
        pattern: impl Into<MaskPattern>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mask_pattern = pattern.into();
        if let Some(placeholder) = self.mask_pattern.placeholder() {
            self.placeholder = placeholder.into();
        }
        cx.notify();
    }

    pub(super) fn set_input_bounds(&mut self, new_bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let wrap_width_changed = self.input_bounds.size.width != new_bounds.size.width;
        self.input_bounds = new_bounds;

        // Update text_wrapper wrap_width if changed.
        if wrap_width_changed {
            self.text_wrapper
                .set_wrap_width(Some(new_bounds.size.width), cx);
            self.mode.update_auto_grow(&self.text_wrapper);
        }
    }

    fn text_for_range_utf8(&mut self, range: impl Into<Range<usize>>) -> &str {
        let range = self.range_from_utf16(&self.range_to_utf16(&range.into()));
        &self.text[range]
    }
}

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        adjusted_range.replace(self.range_to_utf16(&range));
        Some(self.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    /// Replace text in range.
    ///
    /// - If the new text is invalid, it will not be replaced.
    /// - If `range_utf16` is not provided, the current selected range will be used.
    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        let pending_text: SharedString = (self.text_for_range_utf8(0..range.start).to_owned()
            + new_text
            + self.text_for_range_utf8(range.end..self.text.len()))
        .into();
        // Check if the new text is valid
        if !self.is_valid_input(&pending_text) {
            return;
        }

        let mask_text = self.mask_pattern.mask(&pending_text);
        let new_text_len = (new_text.len() + mask_text.len()).saturating_sub(pending_text.len());
        let new_offset = (range.start + new_text_len).min(mask_text.len());

        self.push_history(&range, &new_text, window, cx);
        self.text = mask_text.clone();
        self.mode
            .update_highlighter(&range, self.text.clone(), &new_text, cx);
        self.mode.clear_markers();
        self.text_wrapper.update(self.text.clone(), false, cx);
        self.selected_range = new_offset..new_offset;
        self.marked_range.take();
        self.update_preferred_x_offset(cx);
        self.update_scroll_offset(None, cx);
        self.mode.update_auto_grow(&self.text_wrapper);
        cx.emit(InputEvent::Change(self.unmask_value()));
        cx.notify();
    }

    /// Mark text is the IME temporary insert on typing.
    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let pending_text: SharedString = (self.text_for_range_utf8(0..range.start).to_owned()
            + new_text
            + self.text_for_range_utf8(range.end..self.text.len()))
        .into();
        if !self.is_valid_input(&pending_text) {
            return;
        }

        self.push_history(&range, new_text, window, cx);
        self.text = pending_text;
        self.mode
            .update_highlighter(&range, self.text.clone(), &new_text, cx);
        self.mode.clear_markers();
        self.text_wrapper.update(self.text.clone(), false, cx);
        if new_text.is_empty() {
            // Cancel selection, when cancel IME input.
            self.selected_range = range.start..range.start;
            self.marked_range = None;
        } else {
            self.marked_range = Some(range.start..range.start + new_text.len());
            self.selected_range = new_selected_range_utf16
                .as_ref()
                .map(|range_utf16| self.range_from_utf16(range_utf16))
                .map(|new_range| new_range.start + range.start..new_range.end + range.end)
                .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        }
        self.mode.update_auto_grow(&self.text_wrapper);
        cx.emit(InputEvent::Change(self.unmask_value()));
        cx.notify();
    }

    /// Used to position IME candidates.
    /// TODO: Fix position of IME candidates in multi-line text input.
    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let line_height = last_layout.line_height;
        let range = self.range_from_utf16(&range_utf16);

        let mut start_origin = None;
        let mut end_origin = None;
        let line_number_origin = point(self.line_number_width, px(0.));
        let mut y_offset = px(0.);
        let mut index_offset = 0;

        for line in last_layout.lines.iter() {
            if start_origin.is_some() && end_origin.is_some() {
                break;
            }

            if start_origin.is_none() {
                if let Some(p) =
                    line.position_for_index(range.start.saturating_sub(index_offset), line_height)
                {
                    start_origin = Some(p + point(px(0.), y_offset));
                }
            }

            if end_origin.is_none() {
                if let Some(p) =
                    line.position_for_index(range.end.saturating_sub(index_offset), line_height)
                {
                    end_origin = Some(p + point(px(0.), y_offset));
                }
            }

            index_offset += line.len() + 1;
            y_offset += line.size(line_height).height;
        }

        let start_origin = start_origin.unwrap_or_default();
        let mut end_origin = end_origin.unwrap_or_default();
        // Ensure at same line.
        end_origin.y = start_origin.y;

        Some(Bounds::from_corners(
            bounds.origin + line_number_origin + start_origin,
            // + line_height for show IME panel under the cursor line.
            bounds.origin + line_number_origin + point(end_origin.x, end_origin.y + line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let last_layout = self.last_layout.as_ref()?;
        let line_height = last_layout.line_height;
        let line_point = self.last_bounds?.localize(&point)?;

        for line in last_layout.lines.iter() {
            if let Ok(utf8_index) = line.index_for_position(line_point, line_height) {
                return Some(self.offset_to_utf16(utf8_index));
            }
        }

        None
    }
}

impl Focusable for InputState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InputState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.text_wrapper.update(self.text.clone(), false, cx);
        self.mode
            .update_highlighter(&(0..0), self.text.clone(), "", cx);

        div()
            .id("text-element")
            .flex_1()
            .when(self.is_multi_line(), |this| this.h_full())
            .flex_grow()
            .overflow_x_hidden()
            .child(TextElement::new(cx.entity().clone()).placeholder(self.placeholder.clone()))
            .children(self.diagnostic_popover.clone())
    }
}

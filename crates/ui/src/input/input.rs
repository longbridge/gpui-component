//! A text input field that allows the user to enter text.
//!
//! Based on the `Input` example from the `gpui` crate.
//! https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs

use serde::Deserialize;
use smallvec::SmallVec;
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;
use unicode_segmentation::*;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    actions, div, impl_internal_actions, point, px, relative, AnyElement, App, AppContext, Bounds,
    ClipboardItem, Context, DefiniteLength, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point, Rems, Render,
    ScrollHandle, ScrollWheelEvent, SharedString, Styled as _, Subscription, UTF16Selection,
    Window, WrappedLine,
};

// TODO:
// - Move cursor to skip line eof empty chars.

use super::blink_cursor::BlinkCursor;
use super::change::Change;
use super::element::TextElement;
use super::number_input;

use crate::button::{Button, ButtonVariants as _};
use crate::history::History;
use crate::indicator::Indicator;
use crate::input::clear_button;
use crate::scroll::{Scrollbar, ScrollbarAxis, ScrollbarState};
use crate::{h_flex, StyledExt};
use crate::{ActiveTheme, Root};
use crate::{IconName, Size};
use crate::{Sizable, StyleSized};

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
        Up,
        Down,
        Left,
        Right,
        SelectUp,
        SelectDown,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
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
        TextChanged,
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

const CONTEXT: &str = "Input";

pub fn init(cx: &mut App) {
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
        KeyBinding::new("up", Up, Some(CONTEXT)),
        KeyBinding::new("down", Down, Some(CONTEXT)),
        KeyBinding::new("left", Left, Some(CONTEXT)),
        KeyBinding::new("right", Right, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("shift-up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("shift-down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", Home, Some(CONTEXT)),
        KeyBinding::new("end", End, Some(CONTEXT)),
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
        KeyBinding::new("ctrl-a", Home, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-left", Home, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-e", End, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-right", End, Some(CONTEXT)),
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

pub struct TextInput {
    pub(super) focus_handle: FocusHandle,
    pub(super) text: SharedString,
    multi_line: bool,
    pub(super) history: History<Change>,
    pub(super) blink_cursor: Entity<BlinkCursor>,
    pub(super) prefix: Option<Box<dyn Fn(&mut Window, &mut Context<Self>) -> AnyElement + 'static>>,
    pub(super) suffix: Option<Box<dyn Fn(&mut Window, &mut Context<Self>) -> AnyElement + 'static>>,
    pub(super) loading: bool,
    pub(super) placeholder: SharedString,
    /// Range in UTF-8 length for the selected text.
    ///
    /// - "Hello 世界💝" = 16
    /// - "💝" = 4
    pub(super) selected_range: Range<usize>,
    /// Range for save the selected word, use to keep word range when drag move.
    pub(super) selected_word_range: Option<Range<usize>>,
    pub(super) selection_reversed: bool,
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) last_layout: Option<SmallVec<[WrappedLine; 1]>>,
    pub(super) last_cursor_offset: Option<usize>,
    /// The line_height of text layout, this will change will InputElement painted.
    pub(super) last_line_height: Pixels,
    /// The input container bounds
    pub(super) input_bounds: Bounds<Pixels>,
    /// The text bounds
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    pub(super) last_selected_range: Option<Range<usize>>,
    pub(super) selecting: bool,
    pub(super) disabled: bool,
    pub(super) masked: bool,
    pub(super) mask_toggle: bool,
    pub(super) appearance: bool,
    pub(super) cleanable: bool,
    pub(super) clean_on_escape: bool,
    pub(super) size: Size,
    pub(super) rows: usize,
    pub(super) min_rows: usize,
    pub(super) max_rows: Option<usize>,
    /// For special case, e.g.: NumberInput + - button
    pub(super) no_gap: bool,
    pub(super) height: Option<gpui::DefiniteLength>,
    pattern: Option<regex::Regex>,
    validate: Option<Box<dyn Fn(&str) -> bool + 'static>>,
    pub(crate) scroll_handle: ScrollHandle,
    scrollbar_state: Rc<Cell<ScrollbarState>>,
    /// The size of the scrollable content.
    pub(crate) scroll_size: gpui::Size<Pixels>,
    /// To remember the horizontal column (x-coordinate) of the cursor position.
    preferred_x_offset: Option<Pixels>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<InputEvent> for TextInput {}

impl TextInput {
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

        Self {
            focus_handle: focus_handle.clone(),
            text: "".into(),
            multi_line: false,
            blink_cursor,
            history,
            placeholder: "".into(),
            selected_range: 0..0,
            selected_word_range: None,
            selection_reversed: false,
            marked_range: None,
            input_bounds: Bounds::default(),
            selecting: false,
            disabled: false,
            masked: false,
            mask_toggle: false,
            appearance: true,
            cleanable: false,
            clean_on_escape: false,
            loading: false,
            prefix: None,
            suffix: None,
            no_gap: false,
            size: Size::Medium,
            height: None,
            pattern: None,
            validate: None,
            rows: 2,
            min_rows: 2,
            max_rows: None,
            last_layout: None,
            last_bounds: None,
            last_selected_range: None,
            last_line_height: px(20.),
            last_cursor_offset: None,
            scroll_handle: ScrollHandle::new(),
            scrollbar_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::size(px(0.), px(0.)),
            preferred_x_offset: None,
            _subscriptions,
        }
    }

    /// Use the text input field as a multi-line Textarea.
    pub fn multi_line(mut self) -> Self {
        self.multi_line = true;
        self
    }

    /// Called after moving the cursor. Updates preferred_x_offset if we know where the cursor now is.
    fn update_preferred_x_offset(&mut self, _cx: &mut Context<Self>) {
        if let (Some(lines), Some(bounds)) = (&self.last_layout, &self.last_bounds) {
            let offset = self.cursor_offset();
            let line_height = self.last_line_height;

            // Find which line and sub-line the cursor is on and its position
            let (_line_index, _sub_line_index, cursor_pos) =
                self.line_and_position_for_offset(offset, lines, line_height);

            if let Some(pos) = cursor_pos {
                // Adjust by scroll offset
                let scroll_offset = bounds.origin;
                self.preferred_x_offset = Some(pos.x + scroll_offset.x);
            }
        }
    }

    /// Find which line and sub-line the given offset belongs to, along with the position within that sub-line.
    fn line_and_position_for_offset(
        &self,
        offset: usize,
        lines: &[WrappedLine],
        line_height: Pixels,
    ) -> (usize, usize, Option<Point<Pixels>>) {
        let mut prev_lines_offset = 0;
        let mut y_offset = px(0.);
        for (line_index, line) in lines.iter().enumerate() {
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
    /// direction: -1 for up, +1 for down
    fn move_vertical(&mut self, direction: i32, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }

        let (Some(lines), Some(bounds)) = (&self.last_layout, &self.last_bounds) else {
            return;
        };

        let offset = self.cursor_offset();
        let line_height = self.last_line_height;
        let (current_line_index, current_sub_line, current_pos) =
            self.line_and_position_for_offset(offset, lines, line_height);

        let Some(current_pos) = current_pos else {
            return;
        };

        let current_x = self
            .preferred_x_offset
            .unwrap_or_else(|| current_pos.x + bounds.origin.x);

        let mut new_line_index = current_line_index;
        let mut new_sub_line = current_sub_line as i32;

        new_sub_line += direction;

        // Handle moving above the first line
        if direction == -1 && new_line_index == 0 && new_sub_line < 0 {
            // Move cursor to the beginning of the text
            self.move_to(0, window, cx);
            return;
        }

        if new_sub_line < 0 {
            if new_line_index > 0 {
                new_line_index -= 1;
                new_sub_line = lines[new_line_index].wrap_boundaries.len() as i32;
            } else {
                new_sub_line = 0;
            }
        } else {
            let max_sub_line = lines[new_line_index].wrap_boundaries.len() as i32;
            if new_sub_line > max_sub_line {
                if new_line_index < lines.len() - 1 {
                    new_line_index += 1;
                    new_sub_line = 0;
                } else {
                    new_sub_line = max_sub_line;
                }
            }
        }

        // If after adjustment, still at the same position, do not proceed
        if new_line_index == current_line_index && new_sub_line == current_sub_line as i32 {
            return;
        }

        let target_line = &lines[new_line_index];
        let line_x = current_x - bounds.origin.x;
        let target_sub_line = new_sub_line as usize;

        let approx_pos = point(line_x, px(target_sub_line as f32 * line_height.0));
        let index_res = target_line.index_for_position(approx_pos, line_height);

        let new_local_index = match index_res {
            Ok(i) => i + 1,
            Err(i) => i,
        };

        let mut prev_lines_offset = 0;
        for (i, l) in lines.iter().enumerate() {
            if i == new_line_index {
                break;
            }
            prev_lines_offset += l.len() + 1;
        }

        let new_offset = (prev_lines_offset + new_local_index).min(self.text.len());
        self.selected_range = new_offset..new_offset;
        self.pause_blink_cursor(cx);
        cx.notify();
    }

    #[inline]
    pub(super) fn is_multi_line(&self) -> bool {
        self.multi_line
    }

    #[inline]
    pub(super) fn is_single_line(&self) -> bool {
        !self.multi_line
    }

    /// Set the number of rows for the multi-line Textarea.
    ///
    /// This is only used when `multi_line` is set to true.
    ///
    /// default: 2
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self.min_rows = rows;
        self
    }

    /// Set the maximum number of rows for the multi-line Textarea.
    ///
    /// If max_rows is more than rows, then will enable auto-grow.
    ///
    /// This is only used when `multi_line` is set to true.
    ///
    /// default: None
    pub fn max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = Some(max_rows);
        self
    }

    /// Set the text of the input field.
    ///
    /// And the selection_range will be reset to 0..0.
    pub fn set_text(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.history.ignore = true;
        self.replace_text(text, window, cx);
        self.history.ignore = false;
        // Ensure cursor to start when set text
        self.selected_range = 0..0;

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
        let range = self.range_to_utf16(&(self.cursor_offset()..self.cursor_offset()));
        self.replace_text_in_range(Some(range), &text, window, cx);
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
    }

    /// Set the disabled state of the input field.
    pub fn set_disabled(&mut self, disabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }

    /// Set with masked state.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Set the masked state of the input field.
    pub fn set_masked(&mut self, masked: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.masked = masked;
        cx.notify();
    }

    /// Set to enable toggle button for mask state.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Set the prefix element of the input field.
    pub fn set_prefix<F, E>(&mut self, builder: F, _: &mut Window, cx: &mut Context<Self>)
    where
        F: Fn(&Window, &Context<Self>) -> E + 'static,
        E: IntoElement,
    {
        self.prefix = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        cx.notify();
    }

    /// Set the suffix element of the input field.
    pub fn set_suffix<F, E>(&mut self, builder: F, _: &mut Window, cx: &mut Context<Self>)
    where
        F: Fn(&Window, &Context<Self>) -> E + 'static,
        E: IntoElement,
    {
        self.suffix = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        cx.notify();
    }

    /// Set the Input size
    pub fn set_size(&mut self, size: Size, _window: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }

    /// Set the appearance of the input field.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set the prefix element of the input field, for example a search Icon.
    pub fn prefix<F, E>(mut self, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut Context<Self>) -> E + 'static,
        E: IntoElement,
    {
        self.prefix = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Set the suffix element of the input field, for example a clear button.
    pub fn suffix<F, E>(mut self, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut Context<Self>) -> E + 'static,
        E: IntoElement,
    {
        self.suffix = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Set full height of the input (Multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self
    }

    /// Set height of the input (Multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set the placeholder text of the input field.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the placeholder text of the input field with reference.
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        _: &Window,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    /// Set true to show the clear button when the input field is not empty.
    pub fn cleanable(mut self) -> Self {
        self.cleanable = true;
        self
    }

    /// Set true to clear the input by pressing Escape key.
    pub fn clean_on_escape(mut self) -> Self {
        self.clean_on_escape = true;
        self
    }

    /// Set true to not use gap between input and prefix, suffix, and clear button.
    ///
    /// Default: false
    pub(super) fn no_gap(mut self) -> Self {
        self.no_gap = true;
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

    /// Return the text of the input field.
    pub fn text(&self) -> &SharedString {
        &self.text
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    /// Focus the input field.
    pub fn focus(&self, window: &mut Window, _: &mut Context<Self>) {
        self.focus_handle.focus(window);
    }

    fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), window, cx);
        } else {
            self.move_to(self.selected_range.start, window, cx)
        }
    }

    fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), window, cx);
        } else {
            self.move_to(self.selected_range.end, window, cx)
        }
    }

    fn up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }
        self.pause_blink_cursor(cx);
        self.move_vertical(-1, window, cx);
    }

    fn down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }
        self.pause_blink_cursor(cx);
        self.move_vertical(1, window, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), window, cx);
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), window, cx);
    }

    fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }
        let offset = self.start_of_line(window, cx).saturating_sub(1);
        self.select_to(offset, window, cx);
    }

    fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_single_line() {
            return;
        }
        let offset = (self.end_of_line(window, cx) + 1).min(self.text.len());
        self.select_to(self.next_boundary(offset), window, cx);
    }

    fn select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, window, cx);
        self.select_to(self.text.len(), window, cx)
    }

    fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        let offset = self.start_of_line(window, cx);
        self.move_to(offset, window, cx);
    }

    fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
        let offset = self.end_of_line(window, cx);
        self.move_to(offset, window, cx);
    }

    fn move_to_start(&mut self, _: &MoveToStart, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, window, cx);
    }

    fn move_to_end(&mut self, _: &MoveToEnd, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.text.len();
        self.move_to(end, window, cx);
    }

    fn move_to_previous_word(
        &mut self,
        _: &MoveToPreviousWord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.previous_start_of_word();
        self.move_to(offset, window, cx);
    }

    fn move_to_next_word(
        &mut self,
        _: &MoveToNextWord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.next_end_of_word();
        self.move_to(offset, window, cx);
    }

    fn select_to_start(&mut self, _: &SelectToStart, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, window, cx);
    }

    fn select_to_end(&mut self, _: &SelectToEnd, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.text.len();
        self.select_to(end, window, cx);
    }

    fn select_to_start_of_line(
        &mut self,
        _: &SelectToStartOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.start_of_line(window, cx);
        self.select_to(self.previous_boundary(offset), window, cx);
    }

    fn select_to_end_of_line(
        &mut self,
        _: &SelectToEndOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.end_of_line(window, cx);
        self.select_to(self.next_boundary(offset), window, cx);
    }

    fn select_to_previous_word(
        &mut self,
        _: &SelectToPreviousWordStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.previous_start_of_word();
        self.select_to(offset, window, cx);
    }

    fn select_to_next_word(
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
        let prev_str = &self.text[..offset].to_string();
        UnicodeSegmentation::split_word_bound_indices(prev_str as &str)
            .filter(|(_, s)| !s.trim_start().is_empty())
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Return the next end offset of the next word.
    fn next_end_of_word(&mut self) -> usize {
        let offset = self.cursor_offset();
        let next_str = &self.text[offset..].to_string();
        UnicodeSegmentation::split_word_bound_indices(next_str as &str)
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

    /// Get end of line
    fn end_of_line(&mut self, window: &mut Window, cx: &mut Context<Self>) -> usize {
        if self.is_single_line() {
            return self.text.len();
        }

        let offset = self.next_boundary(self.cursor_offset());
        // ignore if offset is "\n"
        if self
            .text_for_range(
                self.range_to_utf16(&(offset - 1..offset)),
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

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), window, cx)
        }
        self.replace_text_in_range(None, "", window, cx);
        self.pause_blink_cursor(cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), window, cx)
        }
        self.replace_text_in_range(None, "", window, cx);
        self.pause_blink_cursor(cx);
    }

    fn delete_to_beginning_of_line(
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

    fn delete_to_end_of_line(
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

    fn delete_previous_word(
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

    fn delete_next_word(
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

    fn enter(&mut self, action: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_multi_line() {
            let is_eof = self.selected_range.end == self.text.len();
            self.replace_text_in_range(None, "\n", window, cx);

            // Move cursor to the start of the next line
            let mut new_offset = self.next_boundary(self.cursor_offset()) - 1;
            if is_eof {
                new_offset += 1;
            }
            self.move_to(new_offset, window, cx);
        }

        cx.emit(InputEvent::PressEnter {
            secondary: action.secondary,
        });
    }

    fn check_to_auto_grow(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if !self.is_multi_line() {
            return;
        }
        let Some(max_rows) = self.max_rows else {
            return;
        };

        let changed_rows = ((self.scroll_size.height - self.input_bounds.size.height)
            / self.last_line_height) as isize;

        self.rows = (self.rows as isize + changed_rows)
            .clamp(self.min_rows as isize, max_rows as isize)
            .max(0) as usize;
        cx.notify();
    }

    fn clean(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text("", window, cx);
    }

    fn escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.len() > 0 {
            return self.unselect(window, cx);
        }

        if self.clean_on_escape {
            return self.clean(window, cx);
        }

        cx.propagate();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        self.selecting = false;
        self.selected_word_range = None;
    }

    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(self.last_line_height);
        self.update_scroll_offset(Some(self.scroll_handle.offset() + delta), cx);
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

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }

        let selected_text = self.text[self.selected_range.clone()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }

        let selected_text = self.text[self.selected_range.clone()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
        self.replace_text_in_range(None, "", window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(clipboard) = cx.read_from_clipboard() {
            let mut new_text = clipboard.text().unwrap_or_default();
            if !self.multi_line {
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

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;
        if let Some(changes) = self.history.undo() {
            for change in changes {
                let range_utf16 = self.range_to_utf16(&change.new_range);
                self.replace_text_in_range(Some(range_utf16), &change.old_text, window, cx);
            }
        }
        self.history.ignore = false;
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
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

        let (Some(bounds), Some(lines)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };

        let line_height = self.last_line_height;

        // TIP: About the IBeam cursor
        //
        // If cursor style is IBeam, the mouse mouse position is in the middle of the cursor (This is special in OS)

        // The position is relative to the bounds of the text input
        //
        // bounds.origin:
        //
        // - included the input padding.
        // - included the scroll offset.
        let inner_position = position - bounds.origin;

        let mut index = 0;
        let mut y_offset = px(0.);

        for line in lines.iter() {
            let line_origin = self.line_origin_with_y_offset(&mut y_offset, &line, line_height);
            let pos = inner_position - line_origin;
            let closest_index = line.unwrapped_layout.closest_index_for_x(pos.x);

            // Return offset by use closest_index_for_x if is single line mode.
            if self.is_single_line() {
                return closest_index;
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
            } else if line.len() == 0 {
                // empty line
                let line_bounds = Bounds {
                    origin: line_origin,
                    size: gpui::size(bounds.size.width, line_height),
                };
                let pos = inner_position;
                if line_bounds.contains(&pos) {
                    break;
                }
            } else {
                index += line.len();
            }

            // add 1 for \n
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
    fn select_word(&mut self, offset: usize, window: &mut Window, cx: &mut Context<Self>) {
        fn is_word(c: char) -> bool {
            c.is_alphanumeric() || matches!(c, '_')
        }

        let mut start = self.offset_to_utf16(offset);
        let mut end = start;
        let prev_text = self
            .text_for_range(0..start, &mut None, window, cx)
            .unwrap_or_default();
        let next_text = self
            .text_for_range(end..self.text.len(), &mut None, window, cx)
            .unwrap_or_default();

        let prev_chars = prev_text.chars().rev().peekable();
        let next_chars = next_text.chars().peekable();

        for (_, c) in prev_chars.enumerate() {
            if !is_word(c) {
                break;
            }

            start -= c.len_utf16();
        }

        for (_, c) in next_chars.enumerate() {
            if !is_word(c) {
                break;
            }

            end += c.len_utf16();
        }

        self.selected_range = self.range_from_utf16(&(start..end));
        self.selected_word_range = Some(self.selected_range.clone());
        cx.notify()
    }

    fn unselect(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.next_boundary(self.cursor_offset());
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
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

    fn offset_to_utf16(&self, offset: usize) -> usize {
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

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
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

    fn on_key_down(&mut self, _: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
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

        self.pattern
            .as_ref()
            .map(|p| p.is_match(new_text))
            .unwrap_or(true)
    }

    fn render_toggle_mask_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        if !self.mask_toggle {
            return None;
        }

        Some(
            Button::new("toggle-mask")
                .icon(IconName::Eye)
                .xsmall()
                .ghost()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        this.set_masked(false, window, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        this.set_masked(true, window, cx);
                    }),
                ),
        )
    }
}

impl Sizable for TextInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl EntityInputHandler for TextInput {
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

        let pending_text: SharedString =
            (self.text[0..range.start].to_owned() + new_text + &self.text[range.end..]).into();
        if !self.is_valid_input(&pending_text) {
            return;
        }

        self.push_history(&range, new_text, window, cx);
        self.text = pending_text;
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        self.update_preferred_x_offset(cx);
        self.update_scroll_offset(None, cx);
        self.check_to_auto_grow(window, cx);
        cx.emit(InputEvent::Change(self.text.clone()));
        cx.notify();
    }

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
        let pending_text: SharedString =
            (self.text[0..range.start].to_owned() + new_text + &self.text[range.end..]).into();
        if !self.is_valid_input(&pending_text) {
            return;
        }

        self.push_history(&range, new_text, window, cx);
        self.text = pending_text;
        self.marked_range = Some(range.start..range.start + new_text.len());
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.emit(InputEvent::Change(self.text.clone()));
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
        let line_height = self.last_line_height;
        let lines = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);

        let mut start_origin = None;
        let mut end_origin = None;
        let mut y_offset = px(0.);
        let mut index_offset = 0;

        for line in lines.iter() {
            if let Some(p) =
                line.position_for_index(range.start.saturating_sub(index_offset), line_height)
            {
                start_origin = Some(p + point(px(0.), y_offset));
            }
            if let Some(p) =
                line.position_for_index(range.end.saturating_sub(index_offset), line_height)
            {
                end_origin = Some(p + point(px(0.), y_offset));
            }

            y_offset += line.size(line_height).height;
            if start_origin.is_some() && end_origin.is_some() {
                break;
            }

            index_offset += line.len();
        }

        let start_origin = start_origin.unwrap_or_default();
        let end_origin = end_origin.unwrap_or_default();

        Some(Bounds::from_corners(
            bounds.origin + start_origin,
            // + line_height for show IME panel under the cursor line.
            bounds.origin + point(end_origin.x, end_origin.y + line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_height = self.last_line_height;
        let line_point = self.last_bounds?.localize(&point)?;
        let lines = self.last_layout.as_ref()?;

        for line in lines.iter() {
            if let Ok(utf8_index) = line.index_for_position(line_point, line_height) {
                return Some(self.offset_to_utf16(utf8_index));
            }
        }

        None
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const LINE_HEIGHT: Rems = Rems(1.25);
        let focused = self.focus_handle.is_focused(window);
        let mut gap_x = match self.size {
            Size::Small => px(4.),
            Size::Large => px(8.),
            _ => px(4.),
        };
        if self.no_gap {
            gap_x = px(0.);
        }

        let prefix = self.prefix.as_ref().map(|build| build(window, cx));
        let suffix = self.suffix.as_ref().map(|build| build(window, cx));
        let show_clear_button =
            self.cleanable && !self.loading && !self.text.is_empty() && self.is_single_line();
        let bg = if self.disabled {
            cx.theme().muted
        } else {
            cx.theme().background
        };

        div()
            .flex()
            .id("input")
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .when(!self.disabled, |this| {
                this.on_action(cx.listener(Self::backspace))
                    .on_action(cx.listener(Self::delete))
                    .on_action(cx.listener(Self::delete_to_beginning_of_line))
                    .on_action(cx.listener(Self::delete_to_end_of_line))
                    .on_action(cx.listener(Self::delete_previous_word))
                    .on_action(cx.listener(Self::delete_next_word))
                    .on_action(cx.listener(Self::enter))
                    .on_action(cx.listener(Self::escape))
            })
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .when(self.multi_line, |this| {
                this.on_action(cx.listener(Self::up))
                    .on_action(cx.listener(Self::down))
                    .on_action(cx.listener(Self::select_up))
                    .on_action(cx.listener(Self::select_down))
            })
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::select_to_start_of_line))
            .on_action(cx.listener(Self::select_to_end_of_line))
            .on_action(cx.listener(Self::select_to_previous_word))
            .on_action(cx.listener(Self::select_to_next_word))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::move_to_start))
            .on_action(cx.listener(Self::move_to_end))
            .on_action(cx.listener(Self::move_to_previous_word))
            .on_action(cx.listener(Self::move_to_next_word))
            .on_action(cx.listener(Self::select_to_start))
            .on_action(cx.listener(Self::select_to_end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::redo))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .size_full()
            .line_height(LINE_HEIGHT)
            .input_py(self.size)
            .input_h(self.size)
            .cursor_text()
            .when(self.multi_line, |this| {
                this.h_auto()
                    .when_some(self.height, |this, height| this.h(height))
            })
            .when(self.appearance, |this| {
                this.bg(bg)
                    .border_color(cx.theme().input)
                    .border_1()
                    .rounded(cx.theme().radius)
                    .when(cx.theme().shadow, |this| this.shadow_sm())
                    .when(focused, |this| this.focused_border(cx))
            })
            .when(prefix.is_none(), |this| this.input_pl(self.size))
            .input_pr(self.size)
            .items_center()
            .gap(gap_x)
            .children(prefix)
            .child(
                div()
                    .id("text-element")
                    .flex_1()
                    .when(self.is_multi_line(), |this| this.h_full())
                    .flex_grow()
                    .overflow_x_hidden()
                    .child(TextElement::new(cx.entity().clone())),
            )
            .child(
                h_flex()
                    .id("suffix")
                    .absolute()
                    .gap(gap_x)
                    .when(self.appearance, |this| this.bg(bg))
                    .items_center()
                    .when(suffix.is_none(), |this| this.pr_1())
                    .right_0()
                    .when(self.loading, |this| {
                        this.child(Indicator::new().color(cx.theme().muted_foreground))
                    })
                    .children(self.render_toggle_mask_button(window, cx))
                    .when(show_clear_button, |this| {
                        this.child(
                            clear_button(cx).on_click(cx.listener(|view, _, window, cx| {
                                view.clean(window, cx);
                            })),
                        )
                    })
                    .children(suffix),
            )
            .when(self.is_multi_line(), |this| {
                let entity_id = cx.entity().entity_id();
                if self.last_layout.is_some() {
                    let scroll_size = self.scroll_size;

                    this.relative().child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right(px(1.))
                            .bottom_0()
                            .child(
                                Scrollbar::vertical(
                                    entity_id,
                                    self.scrollbar_state.clone(),
                                    self.scroll_handle.clone(),
                                    scroll_size,
                                )
                                .axis(ScrollbarAxis::Vertical),
                            ),
                    )
                } else {
                    this
                }
            })
    }
}

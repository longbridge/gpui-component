use aho_corasick::AhoCorasick;
use rust_i18n::t;
use std::{ops::Range, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Empty, Entity, FocusHandle, Focusable, Half,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render, Styled, Subscription,
    Window, actions, div, prelude::FluentBuilder as _,
};
use ropey::Rope;

use crate::{
    ActiveTheme, Disableable, ElementExt, IconName, Selectable, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{
        Enter, Escape, IndentInline, Input, InputEvent, InputState, OutdentInline, Replace,
        RopeExt as _, Search, movement::MoveDirection,
    },
    label::Label,
    v_flex,
};

const CONTEXT: &'static str = "SearchPanel";

actions!(input, [Tab]);

#[derive(Debug, Clone)]
pub struct SearchMatcher {
    text: Rope,
    pub query: Option<AhoCorasick>,

    pub(super) matched_ranges: Rc<Vec<Range<usize>>>,
    pub(super) current_match_ix: usize,
    /// Is in replacing mode, if true, the next update will update the current match index based on matched ranges.
    replacing: bool,
}

impl SearchMatcher {
    pub fn new() -> Self {
        Self {
            text: "".into(),
            query: None,
            matched_ranges: Rc::new(Vec::new()),
            current_match_ix: 0,
            replacing: false,
        }
    }

    /// Update source text and re-match
    pub(crate) fn update(&mut self, text: &Rope) {
        if self.text.eq(text) {
            // The replacement may equal to the query, in this case the text is
            // unchanged, just leave the replacing state.
            self.replacing = false;
            return;
        }

        self.text = text.clone();
        self.update_matches();
    }

    fn update_matches(&mut self) {
        let mut new_ranges = Vec::new();
        if let Some(query) = &self.query {
            let text = self.text.to_string();
            // FIXME: Use stream find
            let matches = query.stream_find_iter(text.as_bytes());

            for query_match in matches.into_iter() {
                let query_match = query_match.expect("query match for select all action");
                new_ranges.push(query_match.range());
            }
        }
        self.matched_ranges = Rc::new(new_ranges);
        if !self.replacing {
            self.current_match_ix = 0;
        } else if self.matched_ranges.is_empty() {
            self.current_match_ix = 0;
        } else {
            self.current_match_ix = self.current_match_ix.min(self.matched_ranges.len() - 1);
        }
        self.replacing = false;
    }

    /// Update the search query and reset the current match index.
    pub fn update_query(&mut self, query: &str, case_insensitive: bool) {
        if query.len() > 0 {
            self.query = Some(
                AhoCorasick::builder()
                    .ascii_case_insensitive(case_insensitive)
                    .build(&[query.to_string()])
                    .expect("failed to build AhoCorasick query in SearchMatcher"),
            );
        } else {
            self.query = None;
        }
        self.update_matches();
    }

    /// Returns the number of matches found.
    #[allow(unused)]
    #[inline]
    fn len(&self) -> usize {
        self.matched_ranges.len()
    }

    fn peek(&self) -> Option<Range<usize>> {
        let next_match_ix = self.next_ix()?;
        self.matched_ranges.get(next_match_ix).cloned()
    }

    fn next_ix(&self) -> Option<usize> {
        if self.matched_ranges.is_empty() {
            None
        } else if self.has_next_match_without_wrap() {
            Some(self.current_match_ix + 1)
        } else {
            Some(0)
        }
    }

    fn has_next_match_without_wrap(&self) -> bool {
        self.current_match_ix < self.matched_ranges.len().saturating_sub(1)
    }

    fn label(&self) -> String {
        if self.len() == 0 {
            return "0/0".to_string();
        }
        format!("{}/{}", self.current_match_ix + 1, self.len())
    }

    /// Update the current match index based on the given offset.
    fn update_cursor_by_offset(&mut self, offset: usize) {
        for (ix, range) in self.matched_ranges.iter().enumerate() {
            self.current_match_ix = ix;
            if range.contains(&offset) || range.end >= offset {
                return;
            }
        }
    }
}

impl Iterator for SearchMatcher {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_match_ix = self.next_ix()?;
        self.current_match_ix = next_match_ix;
        self.matched_ranges.get(next_match_ix).cloned()
    }
}

impl DoubleEndedIterator for SearchMatcher {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.matched_ranges.is_empty() {
            return None;
        }

        if self.current_match_ix == 0 {
            self.current_match_ix = self.matched_ranges.len();
        }

        self.current_match_ix -= 1;
        let item = self.matched_ranges[self.current_match_ix].clone();

        Some(item)
    }
}

pub(super) struct SearchPanel {
    editor: Entity<InputState>,
    search_input: Entity<InputState>,
    replace_input: Entity<InputState>,
    case_insensitive: bool,
    replace_mode: bool,
    matcher: SearchMatcher,
    input_width: Pixels,

    open: bool,
    _subscriptions: Vec<Subscription>,
}

impl InputState {
    /// Update the search matcher when text changes.
    pub(super) fn update_search(&mut self, cx: &mut App) {
        let Some(search_panel) = self.search_panel.as_ref() else {
            return;
        };

        let text = self.text.clone();
        search_panel.update(cx, |this, _| {
            this.matcher.update(&text);
        });
    }

    pub(super) fn on_action_search(
        &mut self,
        _: &Search,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_search_panel(false, window, cx);
    }

    pub(super) fn on_action_replace(
        &mut self,
        _: &Replace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_search_panel(true, window, cx);
    }

    /// Open (or reuse) the search panel, the `replace_mode` to expand the
    /// replace field, the focus is always on the search input.
    fn open_search_panel(
        &mut self,
        replace_mode: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.searchable {
            return;
        }
        // Fallback to only search, if the input is not replaceable.
        let replace_mode = replace_mode && self.replaceable;

        let search_panel = match self.search_panel.as_ref() {
            Some(panel) => panel.clone(),
            None => SearchPanel::new(cx.entity(), window, cx),
        };

        let text = self.text.clone();
        let editor = cx.entity();
        let selected_text = Rope::from(self.selected_text());
        // Read the visible range here, the panel can not read this editor while
        // this editor is being updated.
        let visible_range_offset = self
            .last_layout
            .as_ref()
            .map(|l| l.visible_range_offset.clone());
        search_panel.update(cx, |this, cx| {
            this.editor = editor;
            this.matcher.update(&text);
            this.show(
                &selected_text,
                replace_mode,
                visible_range_offset,
                window,
                cx,
            );
        });
        self.search_panel = Some(search_panel);
        cx.notify();
    }
}

impl SearchPanel {
    fn next_scroll_direction(
        previous_match_ix: usize,
        current_match_ix: usize,
    ) -> Option<MoveDirection> {
        if current_match_ix <= previous_match_ix {
            None
        } else {
            Some(MoveDirection::Down)
        }
    }

    fn prev_scroll_direction(
        previous_match_ix: usize,
        current_match_ix: usize,
    ) -> Option<MoveDirection> {
        if current_match_ix >= previous_match_ix {
            None
        } else {
            Some(MoveDirection::Up)
        }
    }

    pub fn new(editor: Entity<InputState>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        let search_input = cx.new(|cx| InputState::new(window, cx));
        let replace_input = cx.new(|cx| InputState::new(window, cx));

        cx.new(|cx| {
            let _subscriptions =
                vec![
                    cx.subscribe(&search_input, |this: &mut Self, _, ev: &InputEvent, cx| {
                        // Handle search input changes
                        match ev {
                            InputEvent::Change => {
                                let visible_range_offset = this
                                    .editor
                                    .read(cx)
                                    .last_layout
                                    .as_ref()
                                    .map(|l| l.visible_range_offset.clone());
                                this.update_search_query(visible_range_offset, cx);
                            }
                            _ => {}
                        }
                    }),
                ];

            Self {
                editor,
                search_input,
                replace_input,
                case_insensitive: true,
                replace_mode: false,
                matcher: SearchMatcher::new(),
                open: true,
                input_width: Pixels::ZERO,
                _subscriptions,
            }
        })
    }

    pub(super) fn show(
        &mut self,
        selected_text: &Rope,
        replace_mode: bool,
        visible_range_offset: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open = true;
        self.replace_mode = replace_mode;
        self.search_input
            .read(cx)
            .focus_handle
            .clone()
            .focus(window, cx);

        self.search_input.update(cx, |this, cx| {
            if selected_text.len() > 0 {
                this.set_value(selected_text.to_string(), window, cx);
            }
            this.select_all(&super::SelectAll, window, cx);
        });

        // The `set_value` does not emit `InputEvent::Change`, so update the query
        // here to match the value of the search input.
        self.update_search_query(visible_range_offset, cx);
    }

    /// Update the matcher by the value of the search input.
    ///
    /// The `visible_range_offset` is to select the nearest match of the visible range,
    /// it is passed in, because the editor may be borrowed by the caller.
    fn update_search_query(
        &mut self,
        visible_range_offset: Option<Range<usize>>,
        cx: &mut Context<Self>,
    ) {
        let query = self.search_input.read(cx).value();
        self.matcher
            .update_query(query.as_str(), self.case_insensitive);

        if let Some(visible_range_offset) = visible_range_offset {
            self.matcher
                .update_cursor_by_offset(visible_range_offset.start);
        }
        cx.notify();
    }

    fn replaceable(&self, cx: &App) -> bool {
        let editor = self.editor.read(cx);
        editor.replaceable
    }

    pub(super) fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.editor.read(cx).focus_handle.clone().focus(window, cx);
        cx.notify();
    }

    fn on_action_enter(&mut self, action: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        if action.shift {
            self.prev(window, cx);
        } else {
            self.next(window, cx);
        }
    }

    fn on_action_escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        self.hide(window, cx);
    }

    fn on_action_tab(&mut self, _: &IndentInline, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(window, cx);
    }

    fn on_action_tab_prev(
        &mut self,
        _: &OutdentInline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cycle_focus(window, cx);
    }

    /// Cycle focus between the search and the replace input, to keep the Tab key
    /// staying in the panel.
    ///
    /// There are only 2 inputs, so the forward and the backward are the same.
    fn cycle_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.replace_mode || !self.replaceable(cx) {
            return;
        }

        let search_focus_handle = self.search_input.read(cx).focus_handle.clone();
        let focus_handle = if search_focus_handle.is_focused(window) {
            self.replace_input.read(cx).focus_handle.clone()
        } else {
            search_focus_handle
        };
        focus_handle.focus(window, cx);
    }

    fn on_action_replace(&mut self, _: &Replace, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_replace_mode(window, cx);
    }

    /// Toggle the replace field, and move focus to the field that is going to be used.
    fn toggle_replace_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.replaceable(cx) {
            return;
        }

        self.replace_mode = !self.replace_mode;
        let focus_handle = if self.replace_mode {
            self.replace_input.read(cx).focus_handle.clone()
        } else {
            self.search_input.read(cx).focus_handle.clone()
        };
        focus_handle.focus(window, cx);
        cx.notify();
    }

    fn prev(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let previous_match_ix = self.matcher.current_match_ix;
        if let Some(range) = self.matcher.next_back() {
            let direction =
                Self::prev_scroll_direction(previous_match_ix, self.matcher.current_match_ix);
            self.editor.update(cx, |state, cx| {
                state.scroll_to(range.start, direction, cx);
            });
        }
    }

    fn next(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let previous_match_ix = self.matcher.current_match_ix;
        if let Some(range) = self.matcher.next() {
            let direction =
                Self::next_scroll_direction(previous_match_ix, self.matcher.current_match_ix);
            self.editor.update(cx, |state, cx| {
                state.scroll_to(range.end, direction, cx);
            });
        }
    }

    pub(super) fn matcher(&self) -> Option<&SearchMatcher> {
        if !self.open {
            return None;
        }

        Some(&self.matcher)
    }

    fn replace_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.replaceable(cx) {
            self.replace_mode = false;
            cx.notify();
            return;
        }

        let new_text = self.replace_input.read(cx).value();
        if let Some(range) = self
            .matcher
            .matched_ranges
            .get(self.matcher.current_match_ix)
            .cloned()
        {
            let text_state = self.editor.clone();
            let next_range = self.matcher.peek().unwrap_or(range.clone());
            // The replaced match is dropped from the `matched_ranges` after the text
            // updated, the rest of the matches shift left by one. So keep the
            // `current_match_ix` unchanged to let it point to the next match, only the
            // last one needs to wrap to the first.
            let direction = if self.matcher.has_next_match_without_wrap() {
                Some(MoveDirection::Down)
            } else {
                self.matcher.current_match_ix = 0;
                None
            };
            self.matcher.replacing = true;
            cx.spawn_in(window, async move |_, cx| {
                cx.update(|window, cx| {
                    text_state.update(cx, |state, cx| {
                        let range_utf16 = state.range_to_utf16(&range);
                        state.scroll_to(next_range.end, direction, cx);
                        state.replace_text_in_range_silent(
                            Some(range_utf16),
                            new_text.as_str(),
                            window,
                            cx,
                        );
                    });
                })
            })
            .detach();
        }
    }

    fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.replaceable(cx) {
            self.replace_mode = false;
            cx.notify();
            return;
        }

        let new_text = self.replace_input.read(cx).value();
        let ranges = self.matcher.matched_ranges.clone();
        if ranges.is_empty() {
            return;
        }
        self.matcher.replacing = true;

        let editor = self.editor.clone();
        cx.spawn_in(window, async move |_, cx| {
            cx.update(|window, cx| {
                editor.update(cx, |state, cx| {
                    // Replace from the end to avoid messing up the ranges.
                    let mut rope = state.text.clone();
                    for range in ranges.iter().rev() {
                        rope.replace(range.clone(), new_text.as_str());
                    }
                    state.replace_text_in_range_silent(
                        Some(0..state.text.len()),
                        &rope.to_string(),
                        window,
                        cx,
                    );
                    state.scroll_to(0, Some(MoveDirection::Down), cx);
                });
            })
        })
        .detach();
    }
}

impl Focusable for SearchPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.search_input.read(cx).focus_handle.clone()
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        let has_matches = self.matcher.len() > 0;
        let allow_replace = self.replaceable(cx);
        if !allow_replace {
            self.replace_mode = false;
        }

        v_flex()
            .id("search-panel")
            .occlude()
            .track_focus(&self.focus_handle(cx))
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_enter))
            .on_action(cx.listener(Self::on_action_escape))
            .on_action(cx.listener(Self::on_action_tab))
            .on_action(cx.listener(Self::on_action_tab_prev))
            .on_action(cx.listener(Self::on_action_replace))
            .font_family(cx.theme().font_family.clone())
            .items_center()
            .py_2()
            .px_3()
            .w_full()
            .gap_1()
            .bg(cx.theme().tokens.popover)
            .border_b_1()
            .rounded(cx.theme().radius.half())
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                Input::new(&self.search_input)
                                    .focus_bordered(false)
                                    .suffix(
                                        Button::new("case-insensitive")
                                            .selected(!self.case_insensitive)
                                            .toggled(!self.case_insensitive)
                                            .xsmall()
                                            .compact()
                                            .text()
                                            .icon(IconName::CaseSensitive)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.case_insensitive = !this.case_insensitive;
                                                let visible_range_offset = this
                                                    .editor
                                                    .read(cx)
                                                    .last_layout
                                                    .as_ref()
                                                    .map(|l| l.visible_range_offset.clone());
                                                this.update_search_query(visible_range_offset, cx);
                                                cx.notify();
                                            })),
                                    )
                                    .small()
                                    .w_full()
                                    .shadow_none(),
                            )
                            .on_prepaint({
                                let view = cx.entity();
                                move |bounds, _, cx| {
                                    view.update(cx, |r, _| r.input_width = bounds.size.width)
                                }
                            }),
                    )
                    .when(allow_replace, |this| {
                        this.child(
                            Button::new("replace-mode")
                                .xsmall()
                                .ghost()
                                .icon(IconName::Replace)
                                .selected(self.replace_mode)
                                .toggled(self.replace_mode)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_replace_mode(window, cx);
                                })),
                        )
                    })
                    .child(
                        Button::new("prev")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ChevronLeft)
                            .disabled(!has_matches)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.prev(window, cx);
                            })),
                    )
                    .child(
                        Button::new("next")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ChevronRight)
                            .disabled(!has_matches)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.next(window, cx);
                            })),
                    )
                    .child(
                        Label::new(self.matcher.label())
                            .when(!has_matches, |this| {
                                this.text_color(cx.theme().muted_foreground)
                            })
                            .text_left()
                            .min_w_16(),
                    )
                    .child(div().w_7())
                    .child(
                        Button::new("close")
                            .xsmall()
                            .ghost()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_action_escape(&Escape, window, cx);
                            })),
                    ),
            )
            .when(self.replace_mode && allow_replace, |this| {
                this.child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            Input::new(&self.replace_input)
                                .focus_bordered(false)
                                .small()
                                .w(self.input_width)
                                .shadow_none(),
                        )
                        .child(
                            Button::new("replace-one")
                                .small()
                                .label(t!("Input.Replace"))
                                .disabled(!has_matches)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace_next(window, cx);
                                })),
                        )
                        .child(
                            Button::new("replace-all")
                                .small()
                                .label(t!("Input.Replace All"))
                                .disabled(!has_matches)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace_all(window, cx);
                                })),
                        ),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search() {
        let mut matcher = SearchMatcher::new();
        matcher.update(&Rope::from("Hello 世界 this is a Is test string."));
        matcher.update_query("Is", true);

        assert_eq!(matcher.len(), 3);
        let mut matches = matcher.clone();
        assert_eq!(matches.current_match_ix, 0);
        assert_eq!(matches.next(), Some(18..20));
        assert_eq!(matches.next(), Some(23..25));
        assert_eq!(matches.current_match_ix, 2);
        assert_eq!(matches.next(), Some(15..17));
        assert_eq!(matches.current_match_ix, 0);
        assert_eq!(matches.next_back(), Some(23..25));
        assert_eq!(matches.current_match_ix, 2);
        assert_eq!(matches.next_back(), Some(18..20));
        assert_eq!(matches.current_match_ix, 1);
        assert_eq!(matches.next_back(), Some(15..17));
        assert_eq!(matches.current_match_ix, 0);
        assert_eq!(matches.next_back(), Some(23..25));

        matcher.update_query("IS", false);
        assert_eq!(matcher.len(), 0);
        assert_eq!(matcher.next(), None);
        assert_eq!(matcher.next_back(), None);
    }

    #[test]
    fn test_search_label() {
        let mut matcher = SearchMatcher::new();
        matcher.update(&Rope::from("Hello 世界 this is a Is test string."));
        matcher.update_query("Is", true);
        assert_eq!(matcher.label(), "1/3");
        matcher.next();
        assert_eq!(matcher.label(), "2/3");
        matcher.next();
        assert_eq!(matcher.label(), "3/3");
        matcher.next();
        assert_eq!(matcher.label(), "1/3");

        matcher.update_query("IS", false);
        assert_eq!(matcher.label(), "0/0");
    }

    #[test]
    fn test_select_range_start() {
        let mut matcher = SearchMatcher::new();
        matcher.matched_ranges = Rc::new(vec![5..10, 15..20, 25..30]);
        matcher.update_cursor_by_offset(0);
        assert_eq!(matcher.current_match_ix, 0);

        matcher.update_cursor_by_offset(5);
        assert_eq!(matcher.current_match_ix, 0);

        matcher.update_cursor_by_offset(12);
        assert_eq!(matcher.current_match_ix, 1);

        matcher.update_cursor_by_offset(16);
        assert_eq!(matcher.current_match_ix, 1);

        matcher.update_cursor_by_offset(30);
        assert_eq!(matcher.current_match_ix, 2);

        matcher.update_cursor_by_offset(31);
        assert_eq!(matcher.current_match_ix, 2);
    }

    #[test]
    fn test_next_scroll_direction_returns_down_without_wrap() {
        assert!(matches!(
            SearchPanel::next_scroll_direction(0, 1),
            Some(MoveDirection::Down)
        ));
    }

    #[test]
    fn test_next_scroll_direction_returns_none_on_wrap() {
        assert!(SearchPanel::next_scroll_direction(2, 0).is_none());
    }

    #[test]
    fn test_next_scroll_direction_returns_none_for_single_match() {
        assert!(SearchPanel::next_scroll_direction(0, 0).is_none());
    }

    #[test]
    fn test_next_ix_wraps_to_start() {
        let mut matcher = SearchMatcher::new();
        matcher.matched_ranges = Rc::new(vec![5..10, 15..20, 25..30]);
        matcher.current_match_ix = 2;

        assert_eq!(matcher.next_ix(), Some(0));
    }

    #[test]
    fn test_prev_scroll_direction_returns_up_without_wrap() {
        assert!(matches!(
            SearchPanel::prev_scroll_direction(2, 1),
            Some(MoveDirection::Up)
        ));
    }

    #[test]
    fn test_prev_scroll_direction_returns_none_on_wrap() {
        assert!(SearchPanel::prev_scroll_direction(0, 2).is_none());
    }

    #[test]
    fn test_prev_scroll_direction_returns_none_for_single_match() {
        assert!(SearchPanel::prev_scroll_direction(0, 0).is_none());
    }

    /// Replacing a match should move to the next match, not skip over it.
    #[test]
    fn test_replace_keeps_current_match_index_on_next_match() {
        let mut matcher = SearchMatcher::new();
        matcher.update(&Rope::from("foo foo foo"));
        matcher.update_query("foo", true);
        assert_eq!(matcher.label(), "1/3");

        // Replace the 1st match, the remaining matches shift left by one.
        assert!(matcher.has_next_match_without_wrap());
        matcher.replacing = true;
        matcher.update(&Rope::from("bar foo foo"));
        assert_eq!(matcher.current_match_ix, 0);
        assert_eq!(matcher.matched_ranges[0], 4..7);
        assert_eq!(matcher.label(), "1/2");

        // Replace the 2nd match (the last one), it should wrap to the first.
        matcher.current_match_ix = 1;
        assert!(!matcher.has_next_match_without_wrap());
        matcher.current_match_ix = 0;
        matcher.replacing = true;
        matcher.update(&Rope::from("bar foo bar"));
        assert_eq!(matcher.current_match_ix, 0);
        assert_eq!(matcher.matched_ranges[0], 4..7);
        assert_eq!(matcher.label(), "1/1");
    }

    #[test]
    fn test_update_matches_clamps_current_match_index_while_replacing() {
        let mut matcher = SearchMatcher::new();
        matcher.update(&Rope::from("foo foo foo"));
        matcher.update_query("foo", true);
        matcher.current_match_ix = 2;
        matcher.replacing = true;

        matcher.update(&Rope::from("foo xoo foo"));

        assert_eq!(matcher.len(), 2);
        assert_eq!(matcher.current_match_ix, 1);
        assert_eq!(matcher.label(), "2/2");
        assert!(!matcher.replacing);
    }
}

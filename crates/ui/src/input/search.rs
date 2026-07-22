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
        Enter, Escape, IndentInline, Input, InputEvent, InputState, RopeExt as _, Search,
        movement::MoveDirection,
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
    input_width: Pixels,
    visible: bool,
    _subscriptions: Vec<Subscription>,
}

impl InputState {
    pub(super) fn on_action_search(
        &mut self,
        _: &Search,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.searchable {
            return;
        }

        let search_panel = match self.search_panel.as_ref() {
            Some(panel) => panel.clone(),
            None => SearchPanel::new(cx.entity(), window, cx),
        };

        self.search_matcher.update(&self.text);

        let editor = cx.entity();
        let selected_text = Rope::from(self.selected_text());
        let query = if selected_text.len() > 0 {
            selected_text.to_string()
        } else {
            search_panel
                .read(cx)
                .search_input
                .read(cx)
                .value()
                .to_string()
        };
        let case_insensitive = search_panel.read(cx).case_insensitive;

        search_panel.update(cx, |this, cx| {
            this.editor = editor;
            this.show(&selected_text, window, cx);
        });

        self.set_search_query(query, case_insensitive, cx);
        self.search_panel = Some(search_panel);

        cx.notify();
    }

    /// Set the active search query.
    pub fn set_search_query(
        &mut self,
        query: impl AsRef<str>,
        case_insensitive: bool,
        cx: &mut Context<Self>,
    ) {
        let query = query.as_ref();
        let visible_range_offset = self
            .last_layout
            .as_ref()
            .map(|l| l.visible_range_offset.clone());

        self.search_matcher.update(&self.text);
        self.search_matcher.update_query(query, case_insensitive);

        if let Some(visible_range_offset) = visible_range_offset {
            self.search_matcher
                .update_cursor_by_offset(visible_range_offset.start);
        }

        cx.notify();
    }

    /// Clear the active search query.
    pub fn clear_search(&mut self, cx: &mut Context<Self>) {
        self.search_matcher.update_query("", true);

        cx.notify();
    }

    /// Move to the next search match.
    pub fn search_next(&mut self, cx: &mut Context<Self>) {
        let previous_match_ix = self.search_matcher.current_match_ix;

        if let Some(range) = self.search_matcher.next() {
            let direction =
                next_scroll_direction(previous_match_ix, self.search_matcher.current_match_ix);

            self.scroll_to(range.end, direction, cx);

            cx.notify();
        }
    }

    /// Move to the previous search match.
    pub fn search_previous(&mut self, cx: &mut Context<Self>) {
        let previous_match_ix = self.search_matcher.current_match_ix;

        if let Some(range) = self.search_matcher.next_back() {
            let direction =
                prev_scroll_direction(previous_match_ix, self.search_matcher.current_match_ix);

            self.scroll_to(range.start, direction, cx);

            cx.notify();
        }
    }

    /// Return the number of active search matches.
    pub fn search_match_count(&self) -> usize {
        self.active_search_matcher()
            .map(SearchMatcher::len)
            .unwrap_or(0)
    }

    /// Return the zero-based index of the current search match.
    pub fn current_search_match_index(&self) -> Option<usize> {
        let matcher = self.active_search_matcher()?;

        if matcher.len() == 0 {
            None
        } else {
            Some(matcher.current_match_ix)
        }
    }

    pub(super) fn active_search_matcher(&self) -> Option<&SearchMatcher> {
        if self.search_matcher.query.is_none() {
            return None;
        }

        Some(&self.search_matcher)
    }

    fn search_panel_status(&self) -> (bool, String) {
        (self.search_matcher.len() > 0, self.search_matcher.label())
    }

    /// Replace the current search match.
    ///
    /// Returns true when a match was replaced.
    pub fn replace_current_search_match(
        &mut self,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.replaceable {
            return false;
        }

        if self.search_matcher.query.is_none() {
            return false;
        }

        let previous_match_ix = self.search_matcher.current_match_ix;
        let Some(range) = self
            .search_matcher
            .matched_ranges
            .get(self.search_matcher.current_match_ix)
            .cloned()
        else {
            return false;
        };

        self.search_matcher.replacing = true;

        let next_match_ix = self.search_matcher.next_ix().unwrap_or(previous_match_ix);
        let next_range = self.search_matcher.peek().unwrap_or(range.clone());

        self.search_matcher.current_match_ix = next_match_ix;

        let direction = next_scroll_direction(previous_match_ix, next_match_ix);
        let range_utf16 = self.range_to_utf16(&range);

        self.scroll_to(next_range.end, direction, cx);
        self.replace_text_in_range_silent(Some(range_utf16), new_text, window, cx);

        true
    }

    /// Replace all active search matches and return the number of replacements.
    pub fn replace_all_search_matches(
        &mut self,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        if !self.replaceable {
            return 0;
        }

        if self.search_matcher.query.is_none() {
            return 0;
        }

        let ranges = self.search_matcher.matched_ranges.clone();

        if ranges.is_empty() {
            return 0;
        }

        self.search_matcher.replacing = true;

        let count = ranges.len();
        let mut rope = self.text.clone();

        for range in ranges.iter().rev() {
            rope.replace(range.clone(), new_text);
        }

        let range_utf16 = self.range_to_utf16(&(0..self.text.len()));

        self.replace_text_in_range_silent(Some(range_utf16), &rope.to_string(), window, cx);
        self.scroll_to(0, Some(MoveDirection::Down), cx);

        count
    }
}

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

impl SearchPanel {
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
                                this.update_search_query(cx);
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
                visible: true,
                input_width: Pixels::ZERO,
                _subscriptions,
            }
        })
    }

    pub(super) fn show(
        &mut self,
        selected_text: &Rope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.visible = true;
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
    }

    fn update_search_query(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value();

        self.editor.update(cx, |editor, cx| {
            editor.set_search_query(query.as_str(), self.case_insensitive, cx)
        });

        cx.notify();
    }

    fn replaceable(&self, cx: &App) -> bool {
        let editor = self.editor.read(cx);
        editor.replaceable
    }

    pub(super) fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.visible = false;
        self.editor.update(cx, |editor, cx| editor.clear_search(cx));
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
        self.editor.focus_handle(cx).focus(window, cx);
    }

    fn prev(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |state, cx| state.search_previous(cx));

        cx.notify();
    }

    fn next(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |state, cx| state.search_next(cx));

        cx.notify();
    }

    fn replace_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.replaceable(cx) {
            self.replace_mode = false;
            cx.notify();
            return;
        }

        let new_text = self.replace_input.read(cx).value();

        self.editor.update(cx, |state, cx| {
            state.replace_current_search_match(new_text.as_str(), window, cx)
        });

        cx.notify();
    }

    fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.replaceable(cx) {
            self.replace_mode = false;
            cx.notify();
            return;
        }

        let new_text = self.replace_input.read(cx).value();

        self.editor.update(cx, |state, cx| {
            state.replace_all_search_matches(new_text.as_str(), window, cx)
        });

        cx.notify();
    }
}

impl Focusable for SearchPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.search_input.read(cx).focus_handle.clone()
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return Empty.into_any_element();
        }

        let (has_matches, label) = self.editor.read(cx).search_panel_status();
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
                                            .xsmall()
                                            .compact()
                                            .ghost()
                                            .icon(IconName::CaseSensitive)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.case_insensitive = !this.case_insensitive;
                                                this.update_search_query(cx);
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
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace_mode = !this.replace_mode;
                                    if this.replace_mode {
                                        this.replace_input
                                            .read(cx)
                                            .focus_handle
                                            .clone()
                                            .focus(window, cx);
                                    } else {
                                        this.search_input
                                            .read(cx)
                                            .focus_handle
                                            .clone()
                                            .focus(window, cx);
                                    }
                                    cx.notify();
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
                        Label::new(label)
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
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_builtin_search_panel_updates_editor_search(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let editor = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .searchable(true)
                    .default_value("foo bar foo")
            });

            editor.update(cx, |state, cx| state.on_action_search(&Search, window, cx));

            let search_panel = editor
                .read(cx)
                .search_panel
                .as_ref()
                .expect("search panel")
                .clone();

            search_panel.update(cx, |panel, cx| {
                assert!(panel.visible);

                panel.search_input.update(cx, |search_input, cx| {
                    search_input.set_value("foo", window, cx)
                });

                panel.update_search_query(cx);
            });

            let state = editor.read(cx);

            assert_eq!(state.search_match_count(), 2);
            assert_eq!(state.current_search_match_index(), Some(0));
        });
    }

    #[gpui::test]
    fn test_set_search_query_does_not_create_search_panel(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let editor = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .default_value("foo bar foo")
            });

            editor.update(cx, |state, cx| {
                state.set_search_query("foo", true, cx);

                assert_eq!(state.search_match_count(), 2);
                assert_eq!(state.current_search_match_index(), Some(0));
                assert!(state.search_panel.is_none());
            });
        });
    }

    #[gpui::test]
    fn test_search_navigation_wraps(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let editor = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .default_value("foo bar foo")
            });

            editor.update(cx, |state, cx| {
                state.set_search_query("foo", true, cx);
                state.search_next(cx);

                assert_eq!(state.current_search_match_index(), Some(1));

                state.search_next(cx);

                assert_eq!(state.current_search_match_index(), Some(0));

                state.search_previous(cx);

                assert_eq!(state.current_search_match_index(), Some(1));
            });
        });
    }

    #[gpui::test]
    fn test_clear_search_disables_navigation(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let editor = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .default_value("foo bar foo")
            });

            editor.update(cx, |state, cx| {
                state.set_search_query("foo", true, cx);
                state.clear_search(cx);

                assert_eq!(state.search_match_count(), 0);
                assert_eq!(state.current_search_match_index(), None);

                state.search_next(cx);

                assert_eq!(state.current_search_match_index(), None);
            });
        });
    }

    #[gpui::test]
    fn test_replace_search_matches_edits_editor_text(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let editor = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .default_value("foo foo foo")
            });

            editor.update(cx, |state, cx| {
                state.set_search_query("foo", true, cx);

                assert!(state.replace_current_search_match("bar", window, cx));
                assert_eq!(state.value().as_ref(), "bar foo foo");
                assert_eq!(state.search_match_count(), 2);
                assert_eq!(state.replace_all_search_matches("zap", window, cx), 2);
                assert_eq!(state.value().as_ref(), "bar zap zap");
                assert_eq!(state.search_match_count(), 0);
            });
        });
    }

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
            next_scroll_direction(0, 1),
            Some(MoveDirection::Down)
        ));
    }

    #[test]
    fn test_next_scroll_direction_returns_none_on_wrap() {
        assert!(next_scroll_direction(2, 0).is_none());
    }

    #[test]
    fn test_next_scroll_direction_returns_none_for_single_match() {
        assert!(next_scroll_direction(0, 0).is_none());
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
            prev_scroll_direction(2, 1),
            Some(MoveDirection::Up)
        ));
    }

    #[test]
    fn test_prev_scroll_direction_returns_none_on_wrap() {
        assert!(prev_scroll_direction(0, 2).is_none());
    }

    #[test]
    fn test_prev_scroll_direction_returns_none_for_single_match() {
        assert!(prev_scroll_direction(0, 0).is_none());
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

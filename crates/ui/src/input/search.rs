use aho_corasick::AhoCorasick;
use std::{ops::Range, rc::Rc};

use gpui::{
    div, App, AppContext as _, Context, Empty, Entity, Half, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled, Subscription, Window,
};
use rope::Rope;

use crate::{
    button::{Button, ButtonVariants},
    divider::Divider,
    h_flex,
    input::{Enter, Escape, InputEvent, InputState, RopeExt, Search, TextInput},
    ActiveTheme, IconName, Selectable, Sizable,
};

#[derive(Debug, Clone)]
pub struct SearchMatcher {
    text: Rope,
    pub query: Option<AhoCorasick>,

    pub(super) matched_ranges: Rc<Vec<Range<usize>>>,
    pub(super) current_match_ix: usize,
}

impl SearchMatcher {
    pub fn new() -> Self {
        Self {
            text: "".into(),
            query: None,
            matched_ranges: Rc::new(Vec::new()),
            current_match_ix: 0,
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
            let matches = query.stream_find_iter(self.text.bytes_in_range(0..self.text.len()));

            for query_match in matches.into_iter() {
                let query_match = query_match.expect("query match for select all action");
                new_ranges.push(query_match.range());
            }
        }
        self.matched_ranges = Rc::new(new_ranges);
    }

    /// Update the search query and reset the current match index.
    pub fn update_query(&mut self, query: &str, case_insensitive: bool) {
        if query.len() > 0 {
            self.query = Some(
                AhoCorasick::builder()
                    .ascii_case_insensitive(case_insensitive)
                    .build(&[query.to_string()])
                    .unwrap(),
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
}

impl Iterator for SearchMatcher {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.matched_ranges.is_empty() {
            return None;
        }

        let item = self.matched_ranges.get(self.current_match_ix).cloned();
        if self.current_match_ix < self.matched_ranges.len().saturating_sub(1) {
            self.current_match_ix += 1;
        } else {
            self.current_match_ix = 0;
        }

        item
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

        let item = self.matched_ranges[self.current_match_ix - 1].clone();
        self.current_match_ix -= 1;

        Some(item)
    }
}

pub(super) struct SearchPanel {
    text_state: Entity<InputState>,
    search_input: Entity<InputState>,
    case_insensitive: bool,
    matcher: SearchMatcher,

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
        if !self.searchable {
            return;
        }

        let search_panel = match self.search_panel.as_ref() {
            Some(panel) => panel.clone(),
            None => SearchPanel::new(cx.entity(), window, cx),
        };

        let text = self.text.clone();
        let text_state = cx.entity();
        search_panel.update(cx, |this, cx| {
            this.text_state = text_state;
            this.matcher.update(&text);
            this.show(window, cx);
        });
        self.search_panel = Some(search_panel);
        cx.notify();
    }
}

impl SearchPanel {
    pub fn new(text_state: Entity<InputState>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        cx.new(|cx| {
            let _subscriptions = vec![cx.subscribe(
                &search_input,
                |this: &mut Self, search_input, ev: &InputEvent, cx| {
                    // Handle search input changes
                    match ev {
                        InputEvent::Change => {
                            let value = search_input.read(cx).value();
                            this.matcher
                                .update_query(value.as_str(), this.case_insensitive);
                        }
                        _ => {}
                    }
                },
            )];

            Self {
                text_state,
                search_input,
                case_insensitive: true,
                matcher: SearchMatcher::new(),
                open: true,
                _subscriptions,
            }
        })
    }

    pub(super) fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.search_input.read(cx).focus_handle.focus(window);

        self.update_search(cx);
        self.search_input.update(cx, |this, cx| {
            this.select_all(&super::SelectAll, window, cx);
        });
        cx.notify();
    }

    fn update_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value();
        self.matcher
            .update_query(query.as_str(), self.case_insensitive);
        self.update_text_selection(cx);
    }

    pub(super) fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.text_state.read(cx).focus_handle.focus(window);
        cx.notify();
    }

    fn on_escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        self.hide(window, cx);
    }

    fn on_enter(&mut self, enter: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        if enter.secondary {
            self.prev(window, cx);
        } else {
            self.next(window, cx);
        }
    }

    fn update_text_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(range) = self
            .matcher
            .matched_ranges
            .get(self.matcher.current_match_ix)
            .cloned()
        {
            let state = self.text_state.clone();
            cx.spawn(async move |_, cx| {
                _ = cx.update(|cx| {
                    state.update(cx, |state, cx| {
                        state.selected_range = range.into();
                        cx.notify();
                    });
                });
            })
            .detach();
        }
    }

    fn prev(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.matcher.next_back() {
            self.text_state.update(cx, |state, cx| {
                let row = state.text.offset_to_point(range.start).row as usize;
                state.scroll_to_row(row, cx);
            });
        }
    }

    fn next(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.matcher.next() {
            self.text_state.update(cx, |state, cx| {
                let row = state.text.offset_to_point(range.start).row as usize;
                state.scroll_to_row(row, cx);
            });
        }
    }

    pub(super) fn matcher(&self) -> Option<&SearchMatcher> {
        if !self.open {
            return None;
        }

        Some(&self.matcher)
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        h_flex()
            .id("search-panel")
            .font_family(".SystemUIFont")
            .items_center()
            .on_action(cx.listener(Self::on_escape))
            .on_action(cx.listener(Self::on_enter))
            .py_2()
            .px_3()
            .w_full()
            .gap_5()
            .bg(cx.theme().popover)
            .border_b_1()
            .rounded(cx.theme().radius.half())
            .border_color(cx.theme().border)
            .justify_between()
            .child(
                h_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        div().flex_1().mr_2().child(
                            TextInput::new(&self.search_input)
                                .prefix(IconName::Search)
                                .small()
                                .w_full()
                                .cleanable()
                                .shadow_none(),
                        ),
                    )
                    .child(
                        Button::new("case-insensitive")
                            .selected(!self.case_insensitive)
                            .xsmall()
                            .ghost()
                            .icon(IconName::CaseSensitive)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.case_insensitive = !this.case_insensitive;
                                this.update_search(cx);
                                cx.notify();
                            })),
                    )
                    .child(Divider::vertical().mx_1())
                    .child(
                        Button::new("prev")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.prev(window, cx);
                            })),
                    )
                    .child(
                        Button::new("next")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ArrowRight)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.next(window, cx);
                            })),
                    ),
            )
            .child(
                Button::new("close")
                    .xsmall()
                    .ghost()
                    .icon(IconName::Close)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.on_escape(&Escape, window, cx);
                    })),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search() {
        let mut search = SearchMatcher::new();
        search.update(&Rope::from("Hello 世界 this is a Is test string."));
        search.update_query("Is", true);

        assert_eq!(search.len(), 3);
        let mut matches = search.clone().into_iter();
        assert_eq!(matches.next(), Some(15..17));
        assert_eq!(matches.next(), Some(18..20));
        assert_eq!(matches.next(), Some(23..25));
        assert_eq!(matches.current_match_ix, 0);

        assert_eq!(matches.next_back(), Some(23..25));
        assert_eq!(matches.next_back(), Some(18..20));
        assert_eq!(matches.next_back(), Some(15..17));
        assert_eq!(matches.current_match_ix, 0);
        assert_eq!(matches.next_back(), Some(23..25));

        search.update_query("IS", false);
        assert_eq!(search.len(), 0);
        assert_eq!(search.next(), None);
        assert_eq!(search.next_back(), None);
    }
}

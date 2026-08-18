use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, AvailableSpace, Context, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, ListSizingBehavior, ParentElement,
    Pixels, Render, Role, ScrollStrategy, SharedString, Size, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, size,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, StyledExt as _, VirtualListScrollHandle,
    actions::{Cancel, Confirm, SelectDown, SelectUp},
    command::{
        command::CommandOptions,
        item::{CommandEntry, CommandItem},
    },
    h_flex,
    input::{Input, InputEvent, InputState},
    scroll::Scrollbar,
    v_flex, v_virtual_list,
};

pub(crate) const CONTEXT: &str = "Command";

/// The row a separator occupies: a one-pixel rule with a little air on
/// either side. Fixed, so that only the item and heading rows need measuring.
const SEPARATOR_ROW_HEIGHT: f32 = 9.;

pub(crate) fn init(cx: &mut App) {
    let context: Option<&str> = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, context),
        KeyBinding::new("enter", Confirm { secondary: false }, context),
        KeyBinding::new("up", SelectUp, context),
        KeyBinding::new("down", SelectDown, context),
    ]);
}

/// Events emitted by a [`CommandState`].
#[derive(Clone)]
pub enum CommandEvent {
    /// The search query changed.
    ///
    /// Applications can answer this by fetching results and calling
    /// [`CommandState::set_entries`]. Returned items still participate in the
    /// command palette's local matching.
    Query(SharedString),
    /// The highlighted item moved to the item with this value.
    Select(SharedString),
    /// The item with this value was clicked or confirmed with Enter.
    Confirm(SharedString),
    /// Escape was pressed with an empty query.
    Cancel,
}

/// One rendered line of the list.
///
/// Groups are flattened into headings and items so the list is a single
/// sequence of rows, which is what the virtual list scrolls over.
#[derive(Clone, PartialEq)]
enum CommandRow {
    Heading(SharedString),
    /// Holds the index into [`CommandState::matched`].
    Item(usize),
    Separator,
}

/// An item that survived the current query, and where it landed.
#[derive(Clone)]
struct MatchedItem {
    entry_ix: usize,
    item_ix: usize,
    row_ix: usize,
    disabled: bool,
}

/// The heights the row sizes are built from, remeasured every frame so that a
/// theme or font change resizes the rows.
#[derive(Clone, Copy, PartialEq, Default)]
struct RowHeights {
    item: Pixels,
    heading: Pixels,
}

/// The state of a [`crate::command::Command`] palette: its commands, its query
/// and which command is highlighted.
pub struct CommandState {
    focus_handle: FocusHandle,
    query_input: Entity<InputState>,
    scroll_handle: VirtualListScrollHandle,
    entries: Vec<CommandEntry>,
    rows: Vec<CommandRow>,
    row_sizes: Rc<Vec<Size<Pixels>>>,
    row_heights: RowHeights,
    needs_measure: bool,
    matched: Vec<MatchedItem>,
    selected_index: usize,
    /// Set by the builders, which run before the entity exists and so cannot
    /// filter yet; consumed by the first render.
    needs_update: bool,
    loading: bool,
    pending_scroll: Option<usize>,
    /// The placeholder last written to the query input, so that `render` only
    /// writes when it changed — `set_placeholder` notifies, and an
    /// unconditional notify from `render` would redraw every frame.
    applied_placeholder: SharedString,
    pub(crate) options: CommandOptions,
    _subscriptions: Vec<Subscription>,
}

impl CommandState {
    /// Create an empty palette.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query_input = cx.new(|cx| InputState::new(window, cx));

        let _subscriptions = vec![cx.subscribe(&query_input, Self::on_query_input_event)];

        Self {
            focus_handle: cx.focus_handle(),
            query_input,
            scroll_handle: VirtualListScrollHandle::new(),
            entries: Vec::new(),
            rows: Vec::new(),
            row_sizes: Rc::new(Vec::new()),
            row_heights: RowHeights::default(),
            needs_measure: true,
            matched: Vec::new(),
            selected_index: 0,
            needs_update: true,
            loading: false,
            pending_scroll: None,
            applied_placeholder: SharedString::default(),
            options: CommandOptions::default(),
            _subscriptions,
        }
    }

    /// Add an ungrouped item.
    pub fn item(mut self, item: CommandItem) -> Self {
        self.entries.push(CommandEntry::Item(item));
        self.needs_update = true;
        self
    }

    /// Add a group of items.
    pub fn group(mut self, group: impl Into<CommandEntry>) -> Self {
        self.entries.push(group.into());
        self.needs_update = true;
        self
    }

    /// Add a separator between the previous and the next group.
    pub fn separator(mut self) -> Self {
        self.entries.push(CommandEntry::Separator);
        self.needs_update = true;
        self
    }

    /// Replace every entry in the palette.
    pub fn set_entries(
        &mut self,
        entries: impl IntoIterator<Item = CommandEntry>,
        cx: &mut Context<Self>,
    ) {
        self.entries = entries.into_iter().collect();
        self.update_matches(cx);
        self.reset_selection();
        self.pending_scroll = Some(0);
        cx.notify();
    }

    /// The current search query.
    pub fn query(&self, cx: &App) -> SharedString {
        self.query_input.read(cx).value()
    }

    /// Replace the search query, as if it had been typed.
    ///
    /// The input suppresses its own change event for a programmatic write, so
    /// the re-filter and the [`CommandEvent::Query`] happen here instead.
    pub fn set_query(
        &mut self,
        query: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query_input
            .update(cx, |input, cx| input.set_value(query, window, cx));
        self.on_query_changed(cx);
    }

    /// The index of the highlighted item, among the items matching the query.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// The value of the highlighted item, when the query matched anything.
    pub fn selected_value(&self) -> Option<SharedString> {
        self.item_at(self.selected_index)
            .map(|item| item.value().clone())
    }

    /// The number of items matching the current query.
    pub fn matched_count(&self) -> usize {
        self.matched.len()
    }

    /// Move focus to the search field.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.query_input.focus_handle(cx).focus(window, cx);
    }

    /// Show or hide the search field's spinner, and suppress the empty message
    /// while it spins.
    ///
    /// Turn it on while a [`CommandEvent::Query`] is being answered.
    pub fn set_loading(&mut self, loading: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.loading = loading;
        self.query_input
            .update(cx, |input, cx| input.set_loading(loading, window, cx));
        cx.notify();
    }

    /// Whether the search field is showing its spinner.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    // MARK: Matching

    fn item_at(&self, matched_ix: usize) -> Option<&CommandItem> {
        let matched = self.matched.get(matched_ix)?;

        match self.entries.get(matched.entry_ix)? {
            CommandEntry::Item(item) => Some(item),
            CommandEntry::Group(group) => group.items.get(matched.item_ix),
            CommandEntry::Separator => None,
        }
    }

    /// Recompute the visible rows and the matching items for the current query.
    ///
    fn update_matches(&mut self, cx: &App) {
        let query = self.query(cx);
        let query = query.trim();

        let mut rows: Vec<CommandRow> = Vec::new();
        let mut matched: Vec<MatchedItem> = Vec::new();
        // A separator is only drawn once something follows it, which drops the
        // leading, trailing and doubled separators a filtered list leaves behind.
        let mut pending_separator = false;

        for (entry_ix, entry) in self.entries.iter().enumerate() {
            match entry {
                CommandEntry::Separator => pending_separator = !rows.is_empty(),
                CommandEntry::Item(item) => {
                    if !item.matches(query) {
                        continue;
                    }

                    if pending_separator {
                        rows.push(CommandRow::Separator);
                        pending_separator = false;
                    }

                    matched.push(MatchedItem {
                        entry_ix,
                        item_ix: 0,
                        row_ix: rows.len(),
                        disabled: item.is_disabled(),
                    });
                    rows.push(CommandRow::Item(matched.len() - 1));
                }
                CommandEntry::Group(group) => {
                    let visible = group
                        .items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| item.matches(query))
                        .map(|(item_ix, item)| (item_ix, item.is_disabled()))
                        .collect::<Vec<_>>();

                    if visible.is_empty() {
                        continue;
                    }

                    if pending_separator {
                        rows.push(CommandRow::Separator);
                        pending_separator = false;
                    }

                    if let Some(heading) = group.heading() {
                        rows.push(CommandRow::Heading(heading.clone()));
                    }

                    for (item_ix, disabled) in visible {
                        matched.push(MatchedItem {
                            entry_ix,
                            item_ix,
                            row_ix: rows.len(),
                            disabled,
                        });
                        rows.push(CommandRow::Item(matched.len() - 1));
                    }
                }
            }
        }

        self.rows = rows;
        self.matched = matched;
        self.needs_measure = true;
        self.selected_index = self
            .selected_index
            .min(self.matched.len().saturating_sub(1));
        self.rebuild_row_sizes();
    }

    /// Move the highlight to the first item that can be confirmed.
    fn reset_selection(&mut self) {
        self.selected_index = self
            .matched
            .iter()
            .position(|matched| !matched.disabled)
            .unwrap_or(0);
    }

    fn on_query_input_event(
        &mut self,
        _: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        self.on_query_changed(cx);
    }

    /// Re-filter for the query that is now in the field, and report it.
    fn on_query_changed(&mut self, cx: &mut Context<Self>) {
        self.update_matches(cx);
        self.reset_selection();
        self.pending_scroll = Some(0);
        cx.emit(CommandEvent::Query(self.query(cx)));
        cx.notify();
    }

    // MARK: Actions

    fn select(&mut self, matched_ix: usize, cx: &mut Context<Self>) {
        if self.selected_index == matched_ix {
            return;
        }

        self.selected_index = matched_ix;
        self.pending_scroll = self.matched.get(matched_ix).map(|matched| matched.row_ix);

        if let Some(value) = self.selected_value() {
            cx.emit(CommandEvent::Select(value));
        }

        cx.notify();
    }

    /// Move the highlight by `step` items, wrapping around and skipping the
    /// disabled ones.
    fn select_by(&mut self, step: isize, cx: &mut Context<Self>) {
        let len = self.matched.len();
        if len == 0 {
            return;
        }

        let mut next = self.selected_index;
        for _ in 0..len {
            next = (next as isize + step).rem_euclid(len as isize) as usize;
            if !self.matched[next].disabled {
                break;
            }
        }

        self.select(next, cx);
    }

    fn on_action_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_by(-1, cx);
    }

    fn on_action_select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_by(1, cx);
    }

    fn on_action_confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm(self.selected_index, window, cx);
    }

    /// Escape clears a non-empty query first, and only then leaves the palette
    /// — the dialog that hosts it closes on the second press.
    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.query(cx).is_empty() {
            self.set_query("", window, cx);
            return;
        }

        cx.emit(CommandEvent::Cancel);

        cx.propagate();
    }

    fn confirm(&mut self, matched_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.item_at(matched_ix) else {
            return;
        };
        if item.is_disabled() {
            return;
        }

        let value = item.value().clone();
        let handler = item.handler.clone();

        if let Some(handler) = handler {
            handler(window, cx);
        }

        cx.emit(CommandEvent::Confirm(value));
    }

    // MARK: Row sizing

    /// Measure one item row and one heading row.
    ///
    /// The first matching item stands in for all of them — the palette is a
    /// virtual list, so measuring each row would undo the virtualization. A
    /// design built with [`CommandItem::element`] can be as tall as it likes,
    /// as long as every row is the same height.
    fn measure_row_heights(&self, window: &mut Window, cx: &mut Context<Self>) -> RowHeights {
        let available = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
        let sample: SharedString = "A".into();

        let item = if self.matched.is_empty() {
            self.item_row(false, cx)
                .child(
                    h_flex()
                        .flex_1()
                        .gap_2()
                        .items_center()
                        .child(Icon::new(IconName::Search).size_4())
                        .child(sample.clone()),
                )
                .into_any_element()
        } else {
            self.render_item(0, window, cx)
        }
        .layout_as_root(available, window, cx)
        .height;

        let heading = self
            .heading_row(sample, cx)
            .into_any_element()
            .layout_as_root(available, window, cx)
            .height;

        RowHeights { item, heading }
    }

    fn rebuild_row_sizes(&mut self) {
        self.row_sizes = Rc::new(
            self.rows
                .iter()
                .map(|row| {
                    let height = match row {
                        CommandRow::Item(_) => self.row_heights.item,
                        CommandRow::Heading(_) => self.row_heights.heading,
                        CommandRow::Separator => px(SEPARATOR_ROW_HEIGHT),
                    };

                    size(px(0.), height)
                })
                .collect(),
        );
    }

    // MARK: Rendering

    fn sync_placeholder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let placeholder = self
            .options
            .placeholder()
            .cloned()
            .unwrap_or_else(|| t!("Command.placeholder").to_string().into());

        if self.applied_placeholder == placeholder {
            return;
        }

        self.applied_placeholder = placeholder.clone();
        self.query_input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx)
        });
    }

    /// The frame every item row shares, so that the measured height matches the
    /// rendered one.
    fn item_row(&self, selected: bool, cx: &App) -> gpui::Div {
        div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .gap_2()
            .px_2()
            .py_1p5()
            .text_sm()
            .rounded(cx.theme().radius)
            .when(selected, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
    }

    fn heading_row(&self, heading: SharedString, cx: &App) -> gpui::Div {
        div()
            .w_full()
            .px_2()
            .py_1p5()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().muted_foreground)
            .child(heading)
    }

    fn render_row(&self, row_ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match self.rows.get(row_ix) {
            None => div().into_any_element(),
            Some(CommandRow::Separator) => div()
                .w_full()
                .py(px(4.))
                .child(div().h(px(1.)).w_full().bg(cx.theme().border))
                .into_any_element(),
            Some(CommandRow::Heading(heading)) => {
                self.heading_row(heading.clone(), cx).into_any_element()
            }
            Some(CommandRow::Item(matched_ix)) => self.render_item(*matched_ix, window, cx),
        }
    }

    fn render_item(
        &self,
        matched_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.item_at(matched_ix) else {
            return div().into_any_element();
        };

        let disabled = item.is_disabled();
        let selected = self.selected_index == matched_ix && !disabled;
        let muted_foreground = cx.theme().muted_foreground;
        let icon_color = if selected {
            cx.theme().accent_foreground
        } else {
            muted_foreground
        };

        let content = match &item.render {
            Some(render) => render(window, cx),
            None => h_flex()
                .flex_1()
                .gap_2()
                .items_center()
                .when_some(item.icon.clone(), |this, icon| {
                    this.child(icon.size_4().text_color(icon_color))
                })
                .child(item.title().clone())
                .into_any_element(),
        };

        self.item_row(selected, cx)
            .id(("command-item", matched_ix))
            .role(Role::ListBoxOption)
            .aria_selected(selected)
            .when(disabled, |this| this.text_color(muted_foreground))
            .when(!disabled, |this| {
                this.cursor_default()
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered {
                            this.select(matched_ix, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.confirm(matched_ix, window, cx);
                    }))
            })
            .child(content)
            .map(|this| match item.shortcut.clone() {
                Some(shortcut) => this.child(
                    div()
                        .ml_auto()
                        .text_xs()
                        .when(!selected, |this| this.text_color(muted_foreground))
                        .child(shortcut),
                ),
                // The shortcut owns the trailing slot, so only an item without
                // one can show its check there.
                None => this.when(item.checked, |this| {
                    this.child(Icon::new(IconName::Check).ml_auto().size_4())
                }),
            })
            .into_any_element()
    }

    fn render_empty(&self, cx: &App) -> AnyElement {
        let message = self
            .options
            .empty_message()
            .cloned()
            .unwrap_or_else(|| t!("Command.empty").to_string().into());

        div()
            .py_6()
            .w_full()
            .text_center()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(message)
            .into_any_element()
    }
}

impl EventEmitter<CommandEvent> for CommandState {}

impl Focusable for CommandState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.query_input.focus_handle(cx)
    }
}

impl Render for CommandState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_placeholder(window, cx);

        // Matching first: the row heights are measured from a real item.
        if self.needs_update {
            self.needs_update = false;
            self.update_matches(cx);
        }

        if self.needs_measure {
            self.needs_measure = false;
            let row_heights = self.measure_row_heights(window, cx);
            if self.row_heights != row_heights {
                self.row_heights = row_heights;
                self.rebuild_row_sizes();
            }
        }

        if let Some(row_ix) = self.pending_scroll.take() {
            self.scroll_handle
                .scroll_to_item(row_ix, ScrollStrategy::Top);
        }

        let rows_count = self.rows.len();
        let row_sizes = self.row_sizes.clone();

        v_flex()
            .id("command")
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_select_up))
            .on_action(cx.listener(Self::on_action_select_down))
            .on_action(cx.listener(Self::on_action_confirm))
            .on_action(cx.listener(Self::on_action_cancel))
            .w_full()
            .overflow_hidden()
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .when(self.options.is_bordered(), |this| {
                this.rounded(cx.theme().radius_lg)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .refine_style(self.options.style())
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Input::new(&self.query_input)
                            .prefix(
                                Icon::new(IconName::Search).text_color(cx.theme().muted_foreground),
                            )
                            .appearance(false)
                            .p_0(),
                    ),
            )
            .child(
                v_flex()
                    .id("command-list-container")
                    .role(Role::ListBox)
                    .relative()
                    .flex_1()
                    .p_1()
                    .max_h(self.options.max_h())
                    .overflow_hidden()
                    // While a search is in flight the list is empty because the
                    // answer has not arrived, which is not the same as no match.
                    .when(rows_count == 0 && !self.loading, |this| {
                        this.child(self.render_empty(cx))
                    })
                    .when(rows_count > 0, |this| {
                        this.child(
                            v_virtual_list(
                                cx.entity(),
                                "command-list",
                                row_sizes,
                                move |this, visible_range, window, cx| {
                                    visible_range
                                        .map(|row_ix| this.render_row(row_ix, window, cx))
                                        .collect::<Vec<_>>()
                                },
                            )
                            .with_sizing_behavior(ListSizingBehavior::Infer)
                            .track_scroll(&self.scroll_handle),
                        )
                        .child(Scrollbar::vertical(&self.scroll_handle))
                    }),
            )
    }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Entity, IntoElement, ParentElement as _, Render, SharedString,
        Styled as _, TestAppContext, Window, div, px,
    };

    use super::{CommandRow, CommandState};
    use crate::{
        Disableable as _,
        command::{Command, CommandGroup, CommandItem},
    };

    fn suggestions(state: CommandState) -> CommandState {
        state
            .group(
                CommandGroup::new("Suggestions")
                    .item(CommandItem::new("Calendar"))
                    .item(CommandItem::new("Search Emoji"))
                    .item(CommandItem::new("Calculator").disabled(true)),
            )
            .separator()
            .group(
                CommandGroup::new("Settings")
                    .item(CommandItem::new("profile").label("Profile"))
                    .item(CommandItem::new("billing").label("Billing")),
            )
    }

    #[gpui::test]
    fn query_hides_the_groups_that_have_no_match(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let state = cx.new(|cx| suggestions(CommandState::new(window, cx)));

            state.update(cx, |state, cx| {
                state.update_matches(cx);
                assert_eq!(state.matched_count(), 5);
                assert_eq!(
                    state
                        .rows
                        .iter()
                        .filter(|row| matches!(row, CommandRow::Heading(_)))
                        .count(),
                    2,
                );
                assert_eq!(
                    state
                        .rows
                        .iter()
                        .filter(|row| matches!(row, CommandRow::Separator))
                        .count(),
                    1,
                );

                // "Bil" only matches an item of the second group, so the first
                // group's heading and the separator between them both go.
                state.set_query("Bil", window, cx);
                state.update_matches(cx);

                assert_eq!(state.matched_count(), 1);
                assert_eq!(state.selected_value(), Some("billing".into()));
                assert_eq!(
                    state
                        .rows
                        .iter()
                        .filter(|row| matches!(row, CommandRow::Separator))
                        .count(),
                    0,
                );
                assert!(matches!(state.rows.first(), Some(CommandRow::Heading(_))));
            });
        });
    }

    #[gpui::test]
    fn a_query_that_matches_nothing_leaves_no_rows(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let state = cx.new(|cx| suggestions(CommandState::new(window, cx)));

            state.update(cx, |state, cx| {
                state.set_query("zzz", window, cx);
                state.update_matches(cx);

                assert_eq!(state.matched_count(), 0);
                assert!(state.rows.is_empty());
                assert_eq!(state.selected_value(), None);
            });
        });
    }

    #[gpui::test]
    fn keywords_match_when_the_label_does_not(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let state = cx.new(|cx| {
                CommandState::new(window, cx).item(
                    CommandItem::new("profile")
                        .label("Profile")
                        .keywords(["account"]),
                )
            });

            state.update(cx, |state, cx| {
                state.set_query("account", window, cx);
                state.update_matches(cx);

                assert_eq!(state.matched_count(), 1);
            });
        });
    }

    #[gpui::test]
    fn moving_the_highlight_skips_disabled_items_and_wraps(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let state = cx.new(|cx| suggestions(CommandState::new(window, cx)));

            state.update(cx, |state, cx| {
                state.update_matches(cx);
                state.reset_selection();
                assert_eq!(state.selected_value(), Some("Calendar".into()));

                state.select_by(1, cx);
                assert_eq!(state.selected_value(), Some("Search Emoji".into()));

                // "Calculator" is disabled, so it is stepped over.
                state.select_by(1, cx);
                assert_eq!(state.selected_value(), Some("profile".into()));

                state.select_by(-1, cx);
                assert_eq!(state.selected_value(), Some("Search Emoji".into()));

                // Wraps around the end, skipping the disabled item again.
                state.select_by(-1, cx);
                assert_eq!(state.selected_value(), Some("Calendar".into()));
                state.select_by(-1, cx);
                assert_eq!(state.selected_value(), Some("billing".into()));
            });
        });
    }

    #[gpui::test]
    fn confirming_a_disabled_item_does_nothing(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();

        cx.update(|window, cx| {
            let state = cx.new(|cx| {
                CommandState::new(window, cx)
                    .item(CommandItem::new("enabled"))
                    .item(CommandItem::new("disabled").disabled(true))
            });

            state.update(cx, |state, cx| {
                state.update_matches(cx);

                assert_eq!(state.matched_count(), 2);
                // Reaching the disabled row is only possible with the mouse or
                // an explicit index; confirming it must be a no-op.
                state.confirm(1, window, cx);
                assert_eq!(state.selected_index(), 0);
            });
        });
    }

    struct Harness {
        state: Entity<CommandState>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(Command::new(&self.state).max_h(px(200.)))
        }
    }

    /// A custom row design decides the row height, so a two-line result row
    /// gets two lines rather than being squeezed into the standard one.
    #[gpui::test]
    fn a_custom_item_element_sets_the_row_height(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| {
                CommandState::new(window, cx).item(CommandItem::new("standard").label("Standard"))
            }),
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let standard = cx.update(|_, cx| harness.read(cx).state.read(cx).row_heights.item);

        cx.update(|_, cx| {
            let state = harness.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.set_entries(
                    [CommandItem::new("two-line")
                        .element(|_, _| {
                            crate::v_flex()
                                .child(SharedString::from("Symbol"))
                                .child(SharedString::from("Name"))
                        })
                        .into()],
                    cx,
                )
            });
        });
        cx.update(|window, cx| _ = window.draw(cx));

        let custom = cx.update(|_, cx| harness.read(cx).state.read(cx).row_heights.item);
        assert!(
            custom > standard,
            "a two-line row should measure taller than the standard one ({custom:?} vs {standard:?})",
        );
    }

    #[gpui::test]
    fn an_unchanged_custom_row_is_not_remeasured_on_every_frame(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let renders = Rc::new(Cell::new(0));
        let count = renders.clone();

        let (_, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| {
                CommandState::new(window, cx).item(CommandItem::new("custom").element(
                    move |_, _| {
                        count.set(count.get() + 1);
                        div().child("Custom")
                    },
                ))
            }),
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let after_first_draw = renders.get();
        cx.update(|window, cx| _ = window.draw(cx));

        assert_eq!(renders.get() - after_first_draw, 2);
    }

    #[gpui::test]
    fn moving_past_the_visible_rows_scrolls_the_list(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| {
                (0..50).fold(CommandState::new(window, cx), |state, ix| {
                    state.item(CommandItem::new(format!("Item {ix}")))
                })
            }),
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));

        let state = cx.update(|_, cx| harness.read(cx).state.clone());
        assert_eq!(
            state.read_with(cx, |state, _| state.scroll_handle.base_handle().offset().y),
            px(0.),
        );

        // The list is capped well below 50 rows, so walking to the last one has
        // to bring the viewport with it.
        cx.update(|_, cx| {
            state.update(cx, |state, cx| {
                for _ in 0..49 {
                    state.select_by(1, cx);
                }
            })
        });
        cx.update(|window, cx| _ = window.draw(cx));

        assert_eq!(state.read_with(cx, |state, _| state.selected_index()), 49);
        assert!(
            state.read_with(cx, |state, _| state.scroll_handle.base_handle().offset().y) < px(0.),
            "selecting the last row should have scrolled the list",
        );
    }
}

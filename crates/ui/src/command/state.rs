use std::rc::Rc;

use gpui::{
    AbsoluteLength, AnyElement, App, AppContext as _, AvailableSpace, Context, Entity,
    EventEmitter, FocusHandle, Focusable, FontFallbacks, FontFeatures, FontStyle, FontWeight,
    InteractiveElement, IntoElement, KeyBinding, ListSizingBehavior, ParentElement, Pixels, Render,
    Role, ScrollStrategy, SharedString, Size, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Subscription, TextOverflow, WhiteSpace, Window, div, prelude::FluentBuilder as _, px,
    size,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, ElementExt as _, Icon, IconName, StyledExt as _, VirtualListScrollHandle,
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

type CommandFilter = dyn Fn(&CommandItem, &str) -> bool;

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

#[derive(Clone, PartialEq)]
struct TextShapeKey {
    font_family: SharedString,
    font_features: FontFeatures,
    font_fallbacks: Option<FontFallbacks>,
    font_size: AbsoluteLength,
    font_weight: FontWeight,
    font_style: FontStyle,
    white_space: WhiteSpace,
    text_overflow: Option<TextOverflow>,
    line_clamp: Option<usize>,
}

#[derive(Clone, PartialEq)]
struct ListMeasurementKey {
    content_width: Pixels,
    rem_size: Pixels,
    line_height: Pixels,
    text_shape: TextShapeKey,
}

/// An item that survived the current query, and where it landed.
#[derive(Clone)]
struct MatchedItem {
    entry_ix: usize,
    item_ix: usize,
    row_ix: usize,
    disabled: bool,
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
    list_measurement_key: Option<ListMeasurementKey>,
    needs_measure: bool,
    matched: Vec<MatchedItem>,
    selected_index: usize,
    /// Set by the builders, which run before the entity exists and so cannot
    /// filter yet; consumed by the first render.
    needs_update: bool,
    searchable: bool,
    filter: Option<Rc<CommandFilter>>,
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
            list_measurement_key: None,
            needs_measure: true,
            matched: Vec::new(),
            selected_index: 0,
            needs_update: true,
            searchable: true,
            filter: None,
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

    /// Enable or disable local filtering of items as the query changes.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self.needs_update = true;
        self
    }

    /// Set the predicate used to decide whether an item matches the query.
    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&CommandItem, &str) -> bool + 'static,
    {
        self.filter = Some(Rc::new(filter));
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

    /// Move focus to the palette's active control.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if self.searchable {
            self.query_input.focus_handle(cx).focus(window, cx);
        } else {
            self.focus_handle.focus(window, cx);
        }
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

    fn item_matches(&self, item: &CommandItem, query: &str) -> bool {
        if !self.searchable || query.is_empty() {
            true
        } else if let Some(filter) = &self.filter {
            filter(item, query)
        } else {
            item.matches(query)
        }
    }

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
                    if !self.item_matches(item, query) {
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
                        .filter(|(_, item)| self.item_matches(item, query))
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

    fn set_list_measurement_key(
        &mut self,
        measurement_key: ListMeasurementKey,
        cx: &mut Context<Self>,
    ) {
        if self.list_measurement_key.as_ref() == Some(&measurement_key) {
            return;
        }

        self.list_measurement_key = Some(measurement_key);
        self.needs_measure = true;
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

    /// Measure each row before passing the sizes to the virtual list. Custom
    /// item elements can have independent intrinsic heights.
    fn measure_row_sizes(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<Size<Pixels>> {
        let available = size(
            self.list_measurement_key
                .as_ref()
                .map_or(AvailableSpace::MinContent, |key| {
                    AvailableSpace::Definite(key.content_width)
                }),
            AvailableSpace::MinContent,
        );
        let mut text_style = StyleRefinement::default();
        text_style.text = self.options.style().text.clone();

        self.rows
            .iter()
            .enumerate()
            .map(|(row_ix, row)| match row {
                CommandRow::Separator => size(px(0.), px(SEPARATOR_ROW_HEIGHT)),
                CommandRow::Heading(_) | CommandRow::Item(_) => {
                    let row_size = div()
                        .refine_style(&text_style)
                        .child(self.render_row(row_ix, window, cx))
                        .into_any_element()
                        .layout_as_root(available, window, cx);
                    size(px(0.), row_size.height)
                }
            })
            .collect()
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
                    this.child(crate::Sizable::xsmall(Icon::new(IconName::Check).ml_auto()))
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
        if self.searchable {
            self.query_input.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
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
            self.row_sizes = Rc::new(self.measure_row_sizes(window, cx));
        }

        if let Some(row_ix) = self.pending_scroll.take() {
            self.scroll_handle
                .scroll_to_item(row_ix, ScrollStrategy::Top);
        }

        let rows_count = self.rows.len();
        let row_sizes = self.row_sizes.clone();
        let command_state = cx.entity();

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
            .when_some(self.options.header(), |this, header| {
                this.child(header(self, window, cx))
            })
            .when(self.searchable, |this| {
                this.child(
                    div()
                        .flex_none()
                        .px_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            Input::new(&self.query_input)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .appearance(false)
                                .p_0(),
                        ),
                )
            })
            .child(
                v_flex()
                    .id("command-list-container")
                    .role(Role::ListBox)
                    .relative()
                    .flex_1()
                    .p_1()
                    .on_prepaint({
                        let measure_state = command_state.clone();
                        move |bounds, window, cx| {
                            measure_state.update(cx, |state, cx| {
                                // `p_1` is one quarter rem on each side. Its
                                // rem-dependent padding and inherited
                                // layout-relevant text style participate in
                                // the row-size cache key.
                                let text_style = window.text_style();
                                state.set_list_measurement_key(
                                    ListMeasurementKey {
                                        content_width: (bounds.size.width
                                            - window.rem_size() * 0.5)
                                            .max(px(0.)),
                                        rem_size: window.rem_size(),
                                        line_height: window.line_height(),
                                        text_shape: TextShapeKey {
                                            font_family: text_style.font_family,
                                            font_features: text_style.font_features,
                                            font_fallbacks: text_style.font_fallbacks,
                                            font_size: text_style.font_size,
                                            font_weight: text_style.font_weight,
                                            font_style: text_style.font_style,
                                            white_space: text_style.white_space,
                                            text_overflow: text_style.text_overflow,
                                            line_clamp: text_style.line_clamp,
                                        },
                                    },
                                    cx,
                                )
                            })
                        }
                    })
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
                                command_state.clone(),
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
            .when_some(self.options.footer(), |this, footer| {
                this.child(footer(self, window, cx))
            })
    }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use gpui::{
        AppContext as _, Entity, IntoElement, ParentElement as _, Pixels, Render, Styled as _,
        TestAppContext, Window, div, prelude::FluentBuilder as _, px,
    };

    use super::{CommandRow, CommandState, SEPARATOR_ROW_HEIGHT};
    use crate::{
        Disableable as _,
        actions::{Confirm, SelectDown},
        command::{Command, CommandEvent, CommandGroup, CommandItem},
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
    fn custom_filter_controls_visible_items(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| {
                CommandState::new(window, cx)
                    .filter(|item, query| item.value().starts_with(query))
                    .item(CommandItem::new("alpha"))
                    .item(CommandItem::new("beta-alpha"))
            });
            state.update(cx, |state, cx| {
                state.set_query("alpha", window, cx);
                assert_eq!(state.matched_count(), 1);
                assert_eq!(state.selected_value(), Some("alpha".into()));
            });
        });
    }

    #[gpui::test]
    fn non_searchable_command_keeps_every_item(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| {
                CommandState::new(window, cx)
                    .searchable(false)
                    .item(CommandItem::new("alpha"))
                    .item(CommandItem::new("beta"))
            });
            state.update(cx, |state, cx| {
                state.set_query("missing", window, cx);
                assert_eq!(state.matched_count(), 2);
            });
        });
    }

    #[gpui::test]
    fn non_searchable_command_uses_frame_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (harness, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| {
                CommandState::new(window, cx)
                    .searchable(false)
                    .item(CommandItem::new("alpha"))
                    .item(CommandItem::new("beta"))
            }),
        });
        let state = cx.update(|_, cx| harness.read(cx).state.clone());
        let confirmed = Rc::new(RefCell::new(None));
        let confirmed_value = confirmed.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&state, move |_, event: &CommandEvent, _| {
                if let CommandEvent::Confirm(value) = event {
                    *confirmed_value.borrow_mut() = Some(value.clone());
                }
            })
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.focus(window, cx));
            assert!(state.read(cx).focus_handle.is_focused(window));
            window.dispatch_action(Box::new(SelectDown), cx);
            window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
        });

        assert_eq!(*confirmed.borrow(), Some("beta".into()));
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

    #[gpui::test]
    fn a_checked_item_uses_an_xsmall_trailing_check_icon(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let unchecked_width = Rc::new(Cell::new(None));
        let checked_width = Rc::new(Cell::new(None));
        let (unchecked, checked) = cx.update(|window, cx| {
            let unchecked_state =
                cx.new(|cx| CommandState::new(window, cx).item(CommandItem::new("theme")));
            let checked_state = cx.new(|cx| {
                CommandState::new(window, cx).item(CommandItem::new("theme").checked(true))
            });
            let unchecked_width = unchecked_width.clone();
            let checked_width = checked_width.clone();
            (
                cx.new(|_| CheckIconWidthHarness {
                    state: unchecked_state,
                    width: unchecked_width,
                }),
                cx.new(|_| CheckIconWidthHarness {
                    state: checked_state,
                    width: checked_width,
                }),
            )
        });

        cx.draw(
            gpui::point(px(0.), px(0.)),
            gpui::AvailableSpace::min_size(),
            move |_, _| unchecked.into_any_element(),
        );

        cx.draw(
            gpui::point(px(0.), px(0.)),
            gpui::AvailableSpace::min_size(),
            move |_, _| checked.into_any_element(),
        );

        assert_eq!(
            checked_width.get().unwrap() - unchecked_width.get().unwrap(),
            px(20.)
        );
    }

    struct CheckIconWidthHarness {
        state: Entity<CommandState>,
        width: Rc<Cell<Option<gpui::Pixels>>>,
    }

    impl Render for CheckIconWidthHarness {
        fn render(
            &mut self,
            window: &mut Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            let width = self.width.clone();
            let item = self.state.update(cx, |state, cx| {
                state.update_matches(cx);
                state.render_item(0, window, cx)
            });

            div()
                .on_children_prepainted(move |bounds, _, _| width.set(Some(bounds[0].size.width)))
                .child(item)
        }
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

    #[gpui::test]
    fn header_and_footer_render_with_current_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let header_calls = Rc::new(Cell::new(0));
        let footer_calls = Rc::new(Cell::new(0));
        let header_matched_count = Rc::new(Cell::new(None));
        let footer_matched_count = Rc::new(Cell::new(None));

        let (harness, cx) = cx.add_window_view(|window, cx| HeaderFooterHarness {
            state: cx.new(|cx| {
                CommandState::new(window, cx)
                    .item(CommandItem::new("Calendar"))
                    .item(CommandItem::new("Calculator"))
            }),
            header_calls,
            footer_calls,
            header_matched_count,
            footer_matched_count,
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));

        let (header_calls, footer_calls, header_matched_count, footer_matched_count) =
            cx.update(|_, cx| {
                let harness = harness.read(cx);
                (
                    harness.header_calls.get(),
                    harness.footer_calls.get(),
                    harness.header_matched_count.get(),
                    harness.footer_matched_count.get(),
                )
            });
        assert!(header_calls > 0);
        assert!(footer_calls > 0);
        assert_eq!(header_matched_count, Some(2));
        assert_eq!(footer_matched_count, Some(2));
    }

    struct HeaderFooterHarness {
        state: Entity<CommandState>,
        header_calls: Rc<Cell<usize>>,
        footer_calls: Rc<Cell<usize>>,
        header_matched_count: Rc<Cell<Option<usize>>>,
        footer_matched_count: Rc<Cell<Option<usize>>>,
    }

    impl Render for HeaderFooterHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let header_calls = self.header_calls.clone();
            let header_matched_count = self.header_matched_count.clone();
            let footer_calls = self.footer_calls.clone();
            let footer_matched_count = self.footer_matched_count.clone();

            div().size_full().child(
                Command::new(&self.state)
                    .max_h(px(200.))
                    .header(move |state, _, _| {
                        header_calls.set(header_calls.get() + 1);
                        header_matched_count.set(Some(state.matched_count()));
                        div()
                    })
                    .footer(move |state, _, _| {
                        footer_calls.set(footer_calls.get() + 1);
                        footer_matched_count.set(Some(state.matched_count()));
                        div()
                    }),
            )
        }
    }

    struct PaddedHarness {
        state: Entity<CommandState>,
    }

    impl Render for PaddedHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(Command::new(&self.state).max_h(px(200.)).p_4())
        }
    }

    struct WrappingHarness {
        state: Entity<CommandState>,
        width: Pixels,
        no_wrap: bool,
    }

    impl Render for WrappingHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().size_full().child(
                div().w(self.width).child(
                    Command::new(&self.state)
                        .max_h(px(200.))
                        .when(self.no_wrap, |this| this.whitespace_nowrap()),
                ),
            )
        }
    }

    fn wrapping_state(window: &mut Window, cx: &mut gpui::Context<CommandState>) -> CommandState {
        CommandState::new(window, cx).item(CommandItem::new("wrapped").element(|_, _| {
            div()
                .w_full()
                .child("A command row whose content wraps at narrow list widths")
        }))
    }

    #[gpui::test]
    fn wrapping_rows_remeasure_for_the_list_content_width(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| WrappingHarness {
            state: cx.new(|cx| wrapping_state(window, cx)),
            width: px(360.),
            no_wrap: false,
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));

        let wide = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);

        cx.update(|_, cx| {
            harness.update(cx, |harness, cx| {
                harness.width = px(120.);
                cx.notify();
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let narrow = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);

        assert!(
            narrow > wide,
            "the narrow list should cache a taller wrapped row ({narrow:?} vs {wide:?})",
        );
    }

    #[gpui::test]
    fn wrapping_rows_remeasure_when_rem_size_changes(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| {
            window.set_rem_size(px(20.));
            WrappingHarness {
                state: cx.new(|cx| wrapping_state(window, cx)),
                width: px(160.),
                no_wrap: false,
            }
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let smaller_rem = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);

        cx.update(|window, cx| {
            window.set_rem_size(px(28.));
            _ = window.draw(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let larger_rem = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);

        assert!(
            larger_rem > smaller_rem,
            "a larger rem should remeasure the fixed-width wrapped row ({larger_rem:?} vs {smaller_rem:?})",
        );
    }

    #[gpui::test]
    fn wrapping_rows_remeasure_when_inherited_typography_changes(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| WrappingHarness {
            state: cx.new(|cx| wrapping_state(window, cx)),
            width: px(160.),
            no_wrap: false,
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let wrapped_height = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);

        cx.update(|window, cx| {
            harness.update(cx, |harness, cx| {
                harness.no_wrap = true;
                cx.notify();
            });
            _ = window.draw(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let no_wrap_height = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);
        assert!(
            no_wrap_height < wrapped_height,
            "a changed inherited typography should remeasure the fixed-width row ({no_wrap_height:?} vs {wrapped_height:?})",
        );
    }

    #[gpui::test]
    fn outer_command_padding_does_not_inflate_measured_row_heights(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| PaddedHarness {
            state: cx.new(|cx| {
                CommandState::new(window, cx)
                    .item(CommandItem::new("fixed").element(|_, _| div().h(px(32.))))
            }),
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let height = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes[0].height);

        assert_eq!(height, px(44.));
    }

    #[gpui::test]
    fn custom_rows_keep_independent_heights(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (harness, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| {
                CommandState::new(window, cx)
                    .group(
                        CommandGroup::new("Short")
                            .item(CommandItem::new("short").element(|_, _| div().h(px(32.)))),
                    )
                    .separator()
                    .group(
                        CommandGroup::new("Tall")
                            .item(CommandItem::new("tall").element(|_, _| div().h(px(72.)))),
                    )
            }),
        });

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let row_sizes = cx.update(|_, cx| harness.read(cx).state.read(cx).row_sizes.clone());

        assert_eq!(row_sizes.len(), 5);
        assert!(row_sizes[0].height > px(0.));
        assert_eq!(row_sizes[1].height, px(44.));
        assert_eq!(row_sizes[2].height, px(SEPARATOR_ROW_HEIGHT));
        assert!(row_sizes[3].height > px(0.));
        assert_eq!(row_sizes[4].height, px(84.));
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

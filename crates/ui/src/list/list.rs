use std::ops::Range;
use std::time::Duration;

use crate::actions::{Cancel, Confirm, SelectNext, SelectPrev};
use crate::input::InputState;
use crate::list::cache::{MeasuredEntrySize, RowEntry, RowsCache};
use crate::list::ListDelegate;
use crate::{
    input::{InputEvent, TextInput},
    scroll::{Scrollbar, ScrollbarState},
    v_flex, ActiveTheme, IconName, Size,
};
use crate::{
    v_virtual_list, Icon, IndexPath, Selectable, Sizable as _, StyledExt, VirtualListScrollHandle,
};
use gpui::{
    div, prelude::FluentBuilder, AppContext, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, Length, MouseButton, ParentElement, Render, Styled, Task, Window,
};
use gpui::{
    px, size, App, AvailableSpace, Context, Edges, EventEmitter, ListSizingBehavior,
    MouseDownEvent, Pixels, ScrollStrategy, Subscription,
};
use rust_i18n::t;
use smol::Timer;

pub(crate) fn init(cx: &mut App) {
    let context: Option<&str> = Some("List");
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, context),
        KeyBinding::new("enter", Confirm { secondary: false }, context),
        KeyBinding::new("secondary-enter", Confirm { secondary: true }, context),
        KeyBinding::new("up", SelectPrev, context),
        KeyBinding::new("down", SelectNext, context),
    ]);
}

#[derive(Clone)]
pub enum ListEvent {
    /// Move to select item.
    Select(IndexPath),
    /// Click on item or pressed Enter.
    Confirm(IndexPath),
    /// Pressed ESC to deselect the item.
    Cancel,
}

pub struct List<D: ListDelegate> {
    focus_handle: FocusHandle,
    delegate: D,
    max_height: Option<Length>,
    paddings: Edges<Pixels>,
    query_input: Option<Entity<InputState>>,
    last_query: Option<String>,
    selectable: bool,
    querying: bool,
    scrollbar_visible: bool,
    scroll_handle: VirtualListScrollHandle,
    scroll_state: ScrollbarState,
    pub(crate) size: Size,
    rows_cache: RowsCache,
    selected_index: Option<IndexPath>,
    item_to_measure_index: IndexPath,
    deferred_scroll_to_index: Option<(IndexPath, ScrollStrategy)>,
    mouse_right_clicked_index: Option<IndexPath>,
    reset_on_cancel: bool,
    _search_task: Task<()>,
    _load_more_task: Task<()>,
    _query_input_subscription: Subscription,
}

impl<D> List<D>
where
    D: ListDelegate,
{
    pub fn new(delegate: D, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("List.search_placeholder")));

        let _query_input_subscription =
            cx.subscribe_in(&query_input, window, Self::on_query_input_event);

        Self {
            focus_handle: cx.focus_handle(),
            delegate,
            rows_cache: RowsCache::default(),
            query_input: Some(query_input),
            last_query: None,
            selected_index: None,
            item_to_measure_index: IndexPath::default(),
            deferred_scroll_to_index: None,
            mouse_right_clicked_index: None,
            scroll_handle: VirtualListScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
            max_height: None,
            scrollbar_visible: true,
            selectable: true,
            querying: false,
            size: Size::default(),
            reset_on_cancel: true,
            paddings: Edges::default(),
            _search_task: Task::ready(()),
            _load_more_task: Task::ready(()),
            _query_input_subscription,
        }
    }

    /// Set the size
    pub fn set_size(&mut self, size: Size, _: &mut Window, _: &mut Context<Self>) {
        self.size = size;
    }

    pub fn max_h(mut self, height: impl Into<Length>) -> Self {
        self.max_height = Some(height.into());
        self
    }

    /// Set the visibility of the scrollbar, default is true.
    pub fn scrollbar_visible(mut self, visible: bool) -> Self {
        self.scrollbar_visible = visible;
        self
    }

    pub fn no_query(mut self) -> Self {
        self.query_input = None;
        self
    }

    /// Sets whether the list is selectable, default is true.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub fn set_query_input(
        &mut self,
        query_input: Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._query_input_subscription =
            cx.subscribe_in(&query_input, window, Self::on_query_input_event);
        self.query_input = Some(query_input);
    }

    /// Get the query input entity.
    pub fn query_input(&self) -> Option<&Entity<InputState>> {
        self.query_input.as_ref()
    }

    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut D {
        &mut self.delegate
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_handle(cx).focus(window);
    }

    /// Set the selected index of the list,
    /// this will also scroll to the selected item.
    pub(crate) fn _set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_index = ix;
        self.delegate.set_selected_index(ix, window, cx);
        self.scroll_to_selected_item(window, cx);
    }

    /// Set the selected index of the list,
    /// this method will not scroll to the selected item.
    pub fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_index = ix;
        self.delegate.set_selected_index(ix, window, cx);
    }

    pub fn selected_index(&self) -> Option<IndexPath> {
        self.selected_index
    }

    /// Set a specific list item for measurement.
    pub fn set_item_to_measure_index(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.item_to_measure_index = ix;
        cx.notify();
    }

    fn render_scrollbar(&self, _: &mut Window, _: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.scrollbar_visible {
            return None;
        }

        Some(Scrollbar::uniform_scroll(
            &self.scroll_state,
            &self.scroll_handle,
        ))
    }

    /// Scroll to the item at the given index.
    pub fn scroll_to_item(
        &mut self,
        ix: IndexPath,
        strategy: ScrollStrategy,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ix.section == 0 && ix.row == 0 {
            // If the item is the first item, scroll to the top.
            let mut offset = self.scroll_handle.base_handle().offset();
            offset.y = px(0.);
            self.scroll_handle.base_handle().set_offset(offset);
            cx.notify();
            return;
        }
        self.deferred_scroll_to_index = Some((ix, strategy));
        cx.notify();
    }

    /// Get scroll handle
    pub fn scroll_handle(&self) -> &VirtualListScrollHandle {
        &self.scroll_handle
    }

    pub fn scroll_to_selected_item(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected_index {
            self.deferred_scroll_to_index = Some((ix, ScrollStrategy::Top));
            cx.notify();
        }
    }

    /// Set paddings for the list.
    pub fn paddings(mut self, paddings: Edges<Pixels>) -> Self {
        self.paddings = paddings;
        self
    }

    fn on_query_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                let text = text.trim().to_string();
                if Some(&text) == self.last_query.as_ref() {
                    return;
                }

                self.set_querying(true, window, cx);
                let search = self.delegate.perform_search(&text, window, cx);

                if self.rows_cache.len() > 0 {
                    self._set_selected_index(Some(IndexPath::default()), window, cx);
                } else {
                    self._set_selected_index(None, window, cx);
                }

                self._search_task = cx.spawn_in(window, async move |this, window| {
                    search.await;

                    _ = this.update_in(window, |this, _, _| {
                        this.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
                        this.last_query = Some(text);
                    });

                    // Always wait 100ms to avoid flicker
                    Timer::after(Duration::from_millis(100)).await;
                    _ = this.update_in(window, |this, window, cx| {
                        this.set_querying(false, window, cx);
                    });
                });
            }
            InputEvent::PressEnter { secondary } => self.on_action_confirm(
                &Confirm {
                    secondary: *secondary,
                },
                window,
                cx,
            ),
            _ => {}
        }
    }

    fn set_querying(&mut self, querying: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.querying = querying;
        if let Some(input) = &self.query_input {
            input.update(cx, |input, cx| input.set_loading(querying, window, cx))
        }
        cx.notify();
    }

    /// Dispatch delegate's `load_more` method when the
    /// visible range is near the end.
    fn load_more_if_need(
        &mut self,
        entities_count: usize,
        visible_end: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // FIXME: Here need void sections items count.

        let threshold = self.delegate.load_more_threshold();
        // Securely handle subtract logic to prevent attempt
        // to subtract with overflow
        if visible_end >= entities_count.saturating_sub(threshold) {
            if !self.delegate.is_eof(cx) {
                return;
            }

            self._load_more_task = cx.spawn_in(window, async move |view, cx| {
                _ = view.update_in(cx, |view, window, cx| {
                    view.delegate.load_more(window, cx);
                });
            });
        }
    }

    pub(crate) fn reset_on_cancel(mut self, reset: bool) -> Self {
        self.reset_on_cancel = reset;
        self
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        cx.propagate();
        if self.reset_on_cancel {
            self._set_selected_index(None, window, cx);
        }

        self.delegate.cancel(window, cx);
        cx.emit(ListEvent::Cancel);
        cx.notify();
    }

    fn on_action_confirm(
        &mut self,
        confirm: &Confirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.rows_cache.len() == 0 {
            return;
        }

        let Some(ix) = self.selected_index else {
            return;
        };

        self.delegate
            .set_selected_index(self.selected_index, window, cx);
        self.delegate.confirm(confirm.secondary, window, cx);
        cx.emit(ListEvent::Confirm(ix));
        cx.notify();
    }

    fn select_item(&mut self, ix: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_index = Some(ix);
        self.delegate.set_selected_index(Some(ix), window, cx);
        self.scroll_to_selected_item(window, cx);
        cx.emit(ListEvent::Select(ix));
        cx.notify();
    }

    pub(crate) fn on_action_select_prev(
        &mut self,
        _: &SelectPrev,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.rows_cache.len() == 0 {
            return;
        }

        let prev_ix = self
            .rows_cache
            .prev(self.selected_index.unwrap_or(IndexPath::default()));
        self.select_item(prev_ix, window, cx);
    }

    pub(crate) fn on_action_select_next(
        &mut self,
        _: &SelectNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.rows_cache.len() == 0 {
            return;
        }

        let next_ix = self
            .rows_cache
            .next(self.selected_index.unwrap_or_default());
        self.select_item(next_ix, window, cx);
    }

    fn render_list_item(
        &self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_index.map(|s| s.eq_row(ix)).unwrap_or(false);
        let mouse_right_clicked = self
            .mouse_right_clicked_index
            .map(|s| s.eq_row(ix))
            .unwrap_or(false);

        div()
            .id("list-item")
            .w_full()
            .relative()
            .children(self.delegate.render_item(ix, window, cx).map(|item| {
                item.selected(selected)
                    .secondary_selected(mouse_right_clicked)
            }))
            .when(self.selectable, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                        this.mouse_right_clicked_index = None;
                        this.selected_index = Some(ix);
                        this.on_action_confirm(
                            &Confirm {
                                secondary: ev.modifiers.secondary(),
                            },
                            window,
                            cx,
                        );
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, _, _, cx| {
                        this.mouse_right_clicked_index = Some(ix);
                        cx.notify();
                    }),
                )
            })
    }

    fn render_items(
        &mut self,
        items_count: usize,
        entities_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let measured_size = self.rows_cache.measured_size();

        v_flex()
            .flex_grow()
            .relative()
            .h_full()
            .min_w(measured_size.item_size.width)
            .when_some(self.max_height, |this, h| this.max_h(h))
            .overflow_hidden()
            .when(items_count == 0, |this| {
                this.child(self.delegate().render_empty(window, cx))
            })
            .when(items_count > 0, {
                let rows_cache = self.rows_cache.clone();
                |this| {
                    this.child(
                        v_virtual_list(
                            cx.entity().clone(),
                            "virtual-list",
                            rows_cache.entries_sizes.clone(),
                            move |list, visible_range: Range<usize>, window, cx| {
                                list.load_more_if_need(
                                    entities_count,
                                    visible_range.end,
                                    window,
                                    cx,
                                );

                                // NOTE: Here the v_virtual_list would not able to have gap_y,
                                // because the section header, footer is always have rendered as a empty child item,
                                // even the delegate give a None result.

                                visible_range
                                    .map(|ix| {
                                        let Some(entry) = rows_cache.get(ix) else {
                                            return div();
                                        };

                                        div().children(match entry {
                                            RowEntry::Entry(index) => Some(
                                                list.render_list_item(index, window, cx)
                                                    .into_any_element(),
                                            ),
                                            RowEntry::SectionHeader(section_ix) => list
                                                .delegate()
                                                .render_section_header(section_ix, window, cx)
                                                .map(|r| r.into_any_element()),
                                            RowEntry::SectionFooter(section_ix) => list
                                                .delegate()
                                                .render_section_footer(section_ix, window, cx)
                                                .map(|r| r.into_any_element()),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .paddings(self.paddings)
                        .when(self.max_height.is_some(), |this| {
                            this.with_sizing_behavior(ListSizingBehavior::Infer)
                        })
                        .track_scroll(&self.scroll_handle)
                        .into_any_element(),
                    )
                }
            })
            .children(self.render_scrollbar(window, cx))
    }

    fn prepare_items_if_needed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sections_count = self.delegate.sections_count(cx);

        let mut measured_size = MeasuredEntrySize::default();

        // Measure the item_height and section header/footer height.
        let available_space = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
        measured_size.item_size = self
            .render_list_item(self.item_to_measure_index, window, cx)
            .into_any_element()
            .layout_as_root(available_space, window, cx);

        if let Some(mut el) = self
            .delegate
            .render_section_header(0, window, cx)
            .map(|r| r.into_any_element())
        {
            measured_size.section_header_size = el.layout_as_root(available_space, window, cx);
        }
        if let Some(mut el) = self
            .delegate
            .render_section_footer(0, window, cx)
            .map(|r| r.into_any_element())
        {
            measured_size.section_footer_size = el.layout_as_root(available_space, window, cx);
        }

        self.rows_cache
            .prepare_if_needed(sections_count, measured_size, cx, |section_ix, cx| {
                self.delegate.items_count(section_ix, cx)
            });
    }
}

impl<D> Focusable for List<D>
where
    D: ListDelegate,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if let Some(query_input) = &self.query_input {
            query_input.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl<D> EventEmitter<ListEvent> for List<D> where D: ListDelegate {}
impl<D> Render for List<D>
where
    D: ListDelegate,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare_items_if_needed(window, cx);

        // Scroll to the selected item if it is set.
        if let Some((ix, strategy)) = self.deferred_scroll_to_index.take() {
            if let Some(item_ix) = self.rows_cache.position_of(&ix) {
                self.scroll_handle.scroll_to_item(item_ix, strategy);
            }
        }

        let items_count = self.rows_cache.items_count();
        let entities_count = self.rows_cache.len();
        let loading = self.delegate.loading(cx);

        let initial_view = if let Some(input) = &self.query_input {
            if input.read(cx).value().is_empty() {
                self.delegate().render_initial(window, cx)
            } else {
                None
            }
        } else {
            None
        };

        v_flex()
            .key_context("List")
            .id("list")
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .overflow_hidden()
            .when_some(self.query_input.clone(), |this, input| {
                this.child(
                    div()
                        .map(|this| match self.size {
                            Size::Small => this.px_1p5(),
                            _ => this.px_2(),
                        })
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            TextInput::new(&input)
                                .with_size(self.size)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .cleanable()
                                .p_0()
                                .appearance(false),
                        ),
                )
            })
            .when(loading, |this| {
                this.child(self.delegate().render_loading(window, cx))
            })
            .when(!loading, |this| {
                this.on_action(cx.listener(Self::on_action_cancel))
                    .on_action(cx.listener(Self::on_action_confirm))
                    .on_action(cx.listener(Self::on_action_select_next))
                    .on_action(cx.listener(Self::on_action_select_prev))
                    .map(|this| {
                        if let Some(view) = initial_view {
                            this.child(view)
                        } else {
                            this.child(self.render_items(items_count, entities_count, window, cx))
                        }
                    })
                    // Click out to cancel right clicked row
                    .when(self.mouse_right_clicked_index.is_some(), |this| {
                        this.on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.mouse_right_clicked_index = None;
                            cx.notify();
                        }))
                    })
            })
    }
}

use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use crate::actions::{Cancel, Confirm, SelectDown, SelectUp};
use crate::input::InputState;
use crate::list::cache::{MeasuredEntrySize, RowEntry, RowsCache};
use crate::list::loading::Loading;
use crate::{
    h_flex, v_virtual_list, Icon, IndexPath, ListItem, Selectable, Sizable as _, StyledExt,
    VirtualListScrollHandle,
};
use crate::{
    input::{InputEvent, TextInput},
    scroll::{Scrollbar, ScrollbarState},
    v_flex, ActiveTheme, IconName, Size,
};
use gpui::{
    div, prelude::FluentBuilder, AppContext, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, ParentElement, Render, Styled, Task, Window,
};
use gpui::{
    px, size, AnyElement, App, AvailableSpace, Context, Edges, EventEmitter, ListSizingBehavior,
    MouseDownEvent, Pixels, RenderOnce, ScrollStrategy, StyleRefinement, Subscription,
};
use rust_i18n::t;
use smol::Timer;

pub(crate) fn init(cx: &mut App) {
    let context: Option<&str> = Some("List");
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, context),
        KeyBinding::new("enter", Confirm { secondary: false }, context),
        KeyBinding::new("secondary-enter", Confirm { secondary: true }, context),
        KeyBinding::new("up", SelectUp, context),
        KeyBinding::new("down", SelectDown, context),
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

#[derive(Clone)]
struct ListRenderContext {
    render_item: Rc<dyn Fn(IndexPath, &mut Window, &mut App) -> Option<ListItem>>,
    render_section_header: Rc<dyn Fn(usize, &mut Window, &mut App) -> Option<AnyElement>>,
    render_section_footer: Rc<dyn Fn(usize, &mut Window, &mut App) -> Option<AnyElement>>,
    render_empty: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    render_initial: Rc<dyn Fn(&mut Window, &mut App) -> Option<AnyElement>>,
    render_loading: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    on_select: Rc<dyn Fn(Option<IndexPath>, &mut Window, &mut App)>,
    on_confirm: Rc<dyn Fn(bool, &mut Window, &mut App)>,
    on_cancel: Rc<dyn Fn(&mut Window, &mut App)>,
    on_load_more: Rc<dyn Fn(&mut Window, &mut App)>,
    on_search: Rc<dyn Fn(&str, &mut Window, &mut App) -> Task<()>>,
}

impl Default for ListRenderContext {
    fn default() -> Self {
        Self {
            render_item: Rc::new(|_, _, _| None),
            render_section_header: Rc::new(|_, _, _| None),
            render_section_footer: Rc::new(|_, _, _| None),
            render_empty: Rc::new(|_, cx| {
                h_flex()
                    .size_full()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child(Icon::new(IconName::Inbox).size_12())
                    .into_any_element()
            }),
            render_initial: Rc::new(|_, _| None),
            render_loading: Rc::new(|_, _| Loading.into_any_element()),
            on_select: Rc::new(|_, _, _| {}),
            on_confirm: Rc::new(|_, _, _| {}),
            on_cancel: Rc::new(|_, _| {}),
            on_load_more: Rc::new(|_, _| {}),
            on_search: Rc::new(|_, _, _| Task::ready(())),
        }
    }
}

pub struct ListState {
    focus_handle: FocusHandle,
    section_items_count: Vec<usize>,
    render_context: ListRenderContext,
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
    load_more_threshold: usize,

    _search_task: Task<()>,
    _load_more_task: Task<()>,
    _query_input_subscription: Subscription,
}

impl ListState {
    /// Create a new List view with a single section.
    ////
    /// The `items_count` is the number of items in that section.
    pub fn new(items_count: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::sections(vec![items_count], window, cx)
    }

    /// Create a new List view with the given section items.
    ///
    /// The `section_items_count` is a vector of usize, each child is the number of items in that section.
    pub fn sections(
        section_items_count: Vec<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("List.search_placeholder")));

        let _query_input_subscription =
            cx.subscribe_in(&query_input, window, Self::on_query_input_event);

        Self {
            focus_handle: cx.focus_handle(),
            render_context: ListRenderContext::default(),
            section_items_count,
            load_more_threshold: 20,
            rows_cache: RowsCache::default(),
            query_input: Some(query_input),
            last_query: None,
            selected_index: None,
            item_to_measure_index: IndexPath::default(),
            deferred_scroll_to_index: None,
            mouse_right_clicked_index: None,
            scroll_handle: VirtualListScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
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

    /// Reset the list state items count and clear the selection and query state.
    pub fn reset(
        &mut self,
        section_items_count: Vec<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.section_items_count = section_items_count;
        self.rows_cache = RowsCache::default();
        self.selected_index = None;
        self.item_to_measure_index = IndexPath::default();
        self.deferred_scroll_to_index = None;
        self.mouse_right_clicked_index = None;
        self.last_query = None;
        self.querying = false;
        if let Some(input) = &self.query_input {
            input.update(cx, |input, cx| input.set_value(String::new(), window, cx))
        }
        cx.notify();
    }

    /// Set a threshold value (n entities), of course,
    /// when scrolling to the bottom, the remaining number of rows
    /// triggers `load_more`.
    ///
    /// This should smaller than the total number of first load rows.
    ///
    /// Default: 20 entities (section header, footer and row)
    pub fn load_more_threshold(mut self, threshold: usize) -> Self {
        self.load_more_threshold = threshold;
        self
    }

    /// Set the size
    pub fn set_size(&mut self, size: Size, _: &mut Window, _: &mut Context<Self>) {
        self.size = size;
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
        (self.render_context.on_select)(ix, window, cx);
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
        (self.render_context.on_select)(ix, window, cx);
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
                let search = (self.render_context.on_search)(&text, window, cx);

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

        let threshold = self.load_more_threshold;
        // Securely handle subtract logic to prevent attempt
        // to subtract with overflow
        if visible_end >= entities_count.saturating_sub(threshold) {
            self._load_more_task = cx.spawn_in(window, async move |state, cx| {
                _ = state.update_in(cx, |state, window, cx| {
                    (state.render_context.on_load_more)(window, cx);
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

        (self.render_context.on_cancel)(window, cx);
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

        (self.render_context.on_select)(self.selected_index, window, cx);
        (self.render_context.on_confirm)(confirm.secondary, window, cx);
        cx.emit(ListEvent::Confirm(ix));
        cx.notify();
    }

    fn select_item(&mut self, ix: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_index = Some(ix);
        (self.render_context.on_select)(self.selected_index, window, cx);
        self.scroll_to_selected_item(window, cx);
        cx.emit(ListEvent::Select(ix));
        cx.notify();
    }

    pub(crate) fn on_action_select_prev(
        &mut self,
        _: &SelectUp,
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
        _: &SelectDown,
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
        render_item: Rc<dyn Fn(IndexPath, &mut Window, &mut App) -> Option<ListItem>>,
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
            .children(render_item(ix, window, cx).map(|item| {
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

    fn prepare_items_if_needed(
        &mut self,
        render_context: &ListRenderContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sections_count = self.section_items_count.len();

        let mut measured_size = MeasuredEntrySize::default();

        // Measure the item_height and section header/footer height.
        let available_space = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
        measured_size.item_size = self
            .render_list_item(
                self.item_to_measure_index,
                render_context.render_item.clone(),
                window,
                cx,
            )
            .into_any_element()
            .layout_as_root(available_space, window, cx);

        if let Some(mut el) =
            (render_context.render_section_header)(0, window, cx).map(|r| r.into_any_element())
        {
            measured_size.section_header_size = el.layout_as_root(available_space, window, cx);
        }
        if let Some(mut el) =
            (render_context.render_section_footer)(0, window, cx).map(|r| r.into_any_element())
        {
            measured_size.section_footer_size = el.layout_as_root(available_space, window, cx);
        }

        self.rows_cache
            .prepare_if_needed(sections_count, measured_size, cx, |section_ix, _| {
                self.section_items_count[section_ix]
            });
    }
}

impl Focusable for ListState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if let Some(query_input) = &self.query_input {
            query_input.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl EventEmitter<ListEvent> for ListState {}
impl Render for ListState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // Scroll to the selected item if it is set.
        if let Some((ix, strategy)) = self.deferred_scroll_to_index.take() {
            if let Some(item_ix) = self.rows_cache.position_of(&ix) {
                self.scroll_handle.scroll_to_item(item_ix, strategy);
            }
        }

        div()
    }
}

#[derive(IntoElement)]
pub struct List {
    state: Entity<ListState>,
    style: StyleRefinement,
    render_context: ListRenderContext,
    size: Size,
    scrollbar_visible: bool,
    loading: bool,
}

impl List {
    pub fn new(state: &Entity<ListState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            size: Size::default(),
            scrollbar_visible: true,
            loading: false,
            render_context: ListRenderContext::default(),
        }
    }

    pub fn scrollbar_visible(mut self, visible: bool) -> Self {
        self.scrollbar_visible = visible;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set the callback for render section header at the given index, default is None.
    ///
    /// NOTE: Every header should have same height.
    pub fn section_header<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) -> Option<AnyElement> + 'static,
    {
        self.render_context.render_section_header = Rc::new(f);
        self
    }

    /// Set the callback for render section footer at the given index, default is None.
    ///
    /// NOTE: Every footer should have same height.
    pub fn section_footer<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) -> Option<AnyElement> + 'static,
    {
        self.render_context.render_section_footer = Rc::new(f);
        self
    }

    /// Set the callback for render item at the given index.
    ///
    /// Return None will skip the item.
    ///
    /// NOTE: Every item should have same height.
    pub fn item<F>(mut self, f: F) -> Self
    where
        F: Fn(IndexPath, &mut Window, &mut App) -> Option<ListItem> + 'static,
    {
        self.render_context.render_item = Rc::new(f);
        self
    }

    /// Set the callback for render empty view, default is a inbox icon.
    pub fn empty<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> AnyElement + 'static,
    {
        self.render_context.render_empty = Rc::new(f);
        self
    }

    /// Set the callback for render loading view, default is None.
    ///
    /// This can be used to show a view for the list before the user has
    /// interacted with it.
    ///
    /// For example: The last search results, or the last selected item.
    ///
    /// Default is None, that means no initial state.
    pub fn initial<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> Option<AnyElement> + 'static,
    {
        self.render_context.render_initial = Rc::new(f);
        self
    }

    /// Set the callback when select an item.
    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(Option<IndexPath>, &mut Window, &mut App) + 'static,
    {
        self.render_context.on_select = Rc::new(f);
        self
    }

    /// Set the callback when confirm an item.
    ///
    /// Callback arguments:
    ///
    /// - `secondary`: whether the confirm action is secondary action.
    /// - `&mut Window`: the window reference.
    /// - `&mut App`: the app context reference.
    pub fn on_confirm<F>(mut self, f: F) -> Self
    where
        F: Fn(bool, &mut Window, &mut App) + 'static,
    {
        self.render_context.on_confirm = Rc::new(f);
        self
    }

    /// Set the callback when cancel the selection.
    pub fn on_cancel<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.render_context.on_cancel = Rc::new(f);
        self
    }

    /// Set the callback to load more data when scrolling to the bottom.
    pub fn on_load_more<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.render_context.on_load_more = Rc::new(f);
        self
    }

    /// Set the callback to perform search when the query input changed.
    pub fn on_search<F>(mut self, f: F) -> Self
    where
        F: Fn(&str, &mut Window, &mut App) -> Task<()> + 'static,
    {
        self.render_context.on_search = Rc::new(f);
        self
    }

    fn render_items(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rows_cache = self.state.read(cx).rows_cache.clone();

        let items_count = rows_cache.items_count();
        let entities_count = rows_cache.len();

        let measured_size = rows_cache.measured_size();
        let scroll_state = self.state.read(cx).scroll_state.clone();
        let scroll_handle = self.state.read(cx).scroll_handle.clone();
        let max_height = self.style.max_size.height;
        let paddings = self.style.padding.clone();

        v_flex()
            .flex_grow()
            .relative()
            .h_full()
            .min_w(measured_size.item_size.width)
            .when_some(max_height, |this, h| this.max_h(h))
            .overflow_hidden()
            .when(items_count == 0, |this| {
                this.child((self.render_context.render_empty)(window, cx))
            })
            .when(items_count > 0, {
                let rows_cache = rows_cache.clone();
                let render_section_header = self.render_context.render_section_header.clone();
                let render_section_footer = self.render_context.render_section_footer.clone();
                let render_item = self.render_context.render_item.clone();
                |this| {
                    this.child(
                        v_virtual_list(
                            self.state.clone(),
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
                                                list.render_list_item(
                                                    index,
                                                    render_item.clone(),
                                                    window,
                                                    cx,
                                                )
                                                .into_any_element(),
                                            ),
                                            RowEntry::SectionHeader(section_ix) => {
                                                render_section_header(section_ix, window, cx)
                                                    .map(|r| r.into_any_element())
                                            }
                                            RowEntry::SectionFooter(section_ix) => {
                                                render_section_footer(section_ix, window, cx)
                                                    .map(|r| r.into_any_element())
                                            }
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .paddings(paddings)
                        .when(max_height.is_some(), |this| {
                            this.with_sizing_behavior(ListSizingBehavior::Infer)
                        })
                        .track_scroll(&scroll_handle)
                        .into_any_element(),
                    )
                }
            })
            .when(self.scrollbar_visible, |this| {
                this.child(Scrollbar::uniform_scroll(&scroll_state, &scroll_handle))
            })
    }
}

impl Styled for List {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for List {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let query_input = state.query_input.clone();
        let focus_handle = state.focus_handle.clone();
        let mouse_right_clicked_index = state.mouse_right_clicked_index.clone();

        let initial_view = if let Some(input) = &query_input {
            if input.read(cx).value().is_empty() {
                (self.render_context.render_initial)(window, cx)
            } else {
                None
            }
        } else {
            None
        };

        self.state.update(cx, |state, cx| {
            state.render_context = self.render_context.clone();
            state.prepare_items_if_needed(&self.render_context, window, cx);
        });

        v_flex()
            .key_context("List")
            .id("list")
            .track_focus(&focus_handle)
            .size_full()
            .relative()
            .overflow_hidden()
            .when_some(query_input, |this, input| {
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
            .when(self.loading, |this| {
                this.child((self.render_context.render_loading)(window, cx))
            })
            .when(!self.loading, |this| {
                this.on_action(window.listener_for(&self.state, ListState::on_action_cancel))
                    .on_action(window.listener_for(&self.state, ListState::on_action_confirm))
                    .on_action(window.listener_for(&self.state, ListState::on_action_select_next))
                    .on_action(window.listener_for(&self.state, ListState::on_action_select_prev))
                    .map(|this| {
                        if let Some(view) = initial_view {
                            this.child(view)
                        } else {
                            this.child(self.render_items(window, cx))
                        }
                    })
                    // Click out to cancel right clicked row
                    .when(mouse_right_clicked_index.is_some(), |this| {
                        this.on_mouse_down_out(window.listener_for(
                            &self.state,
                            |this, _, _, cx| {
                                this.mouse_right_clicked_index = None;
                                cx.notify();
                            },
                        ))
                    })
            })
    }
}

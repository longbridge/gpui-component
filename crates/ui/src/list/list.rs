use std::time::Duration;
use std::{cell::Cell, rc::Rc};

use crate::actions::{Cancel, Confirm, SelectNext, SelectPrev};
use crate::Icon;
use crate::{
    input::{InputEvent, TextInput},
    scroll::{Scrollbar, ScrollbarState},
    v_flex, ActiveTheme, IconName, Size,
};
use gpui::{
    div, prelude::FluentBuilder, uniform_list, AnyElement, AppContext, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, Length, ListSizingBehavior,
    MouseButton, ParentElement, Render, Styled, Task, UniformListScrollHandle, Window,
};
use gpui::{px, App, Context, EventEmitter, MouseDownEvent, ScrollStrategy, Subscription};
use rust_i18n::t;
use smol::Timer;

use super::loading::Loading;

pub fn init(cx: &mut App) {
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
    Select(usize),
    /// Click on item or pressed Enter.
    Confirm(usize),
    /// Pressed ESC to deselect the item.
    Cancel,
}

/// A delegate for the List.
#[allow(unused)]
pub trait ListDelegate: Sized + 'static {
    type Item: IntoElement;

    /// When Query Input change, this method will be called.
    /// You can perform search here.
    fn perform_search(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Task<()> {
        Task::ready(())
    }

    /// Return the number of items in the list.
    fn items_count(&self, cx: &App) -> usize;

    /// Render the item at the given index.
    ///
    /// Return None will skip the item.
    fn render_item(
        &self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item>;

    /// Return a Element to show when list is empty.
    fn render_empty(&self, window: &mut Window, cx: &mut Context<List<Self>>) -> impl IntoElement {
        div()
    }

    /// Returns Some(AnyElement) to render the initial state of the list.
    ///
    /// This can be used to show a view for the list before the user has interacted with it.
    ///
    /// For example: The last search results, or the last selected item.
    ///
    /// Default is None, that means no initial state.
    fn render_initial(
        &self,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<AnyElement> {
        None
    }

    /// Returns the loading state to show the loading view.
    fn loading(&self, cx: &App) -> bool {
        false
    }

    /// Returns a Element to show when loading, default is built-in Skeleton loading view.
    fn render_loading(
        &self,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> impl IntoElement {
        Loading
    }

    /// Set the selected index, just store the ix, don't confirm.
    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    );

    /// Set the confirm and give the selected index, this is means user have clicked the item or pressed Enter.
    ///
    /// This will always to `set_selected_index` before confirm.
    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {}

    /// Cancel the selection, e.g.: Pressed ESC.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {}

    /// Return true to enable load more data when scrolling to the bottom.
    ///
    /// Default: true
    fn can_load_more(&self, cx: &App) -> bool {
        true
    }

    /// Returns a threshold value (n rows), of course, when scrolling to the bottom,
    /// the remaining number of rows triggers `load_more`.
    /// This should smaller than the total number of first load rows.
    ///
    /// Default: 20 rows
    fn load_more_threshold(&self) -> usize {
        20
    }

    /// Load more data when the table is scrolled to the bottom.
    ///
    /// This will performed in a background task.
    ///
    /// This is always called when the table is near the bottom,
    /// so you must check if there is more data to load or lock the loading state.
    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {}
}

pub struct List<D: ListDelegate> {
    focus_handle: FocusHandle,
    delegate: D,
    max_height: Option<Length>,
    query_input: Option<Entity<TextInput>>,
    last_query: Option<String>,
    selectable: bool,
    querying: bool,
    scrollbar_visible: bool,
    vertical_scroll_handle: UniformListScrollHandle,
    scrollbar_state: Rc<Cell<ScrollbarState>>,
    pub(crate) size: Size,
    selected_index: Option<usize>,
    right_clicked_index: Option<usize>,
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
        let query_input = cx.new(|cx| {
            TextInput::new(window, cx)
                .appearance(false)
                .prefix(|_, cx| Icon::new(IconName::Search).text_color(cx.theme().muted_foreground))
                .placeholder(t!("List.search_placeholder"))
                .cleanable()
        });

        let _query_input_subscription =
            cx.subscribe_in(&query_input, window, Self::on_query_input_event);

        Self {
            focus_handle: cx.focus_handle(),
            delegate,
            query_input: Some(query_input),
            last_query: None,
            selected_index: None,
            right_clicked_index: None,
            vertical_scroll_handle: UniformListScrollHandle::new(),
            scrollbar_state: Rc::new(Cell::new(ScrollbarState::new())),
            max_height: None,
            scrollbar_visible: true,
            selectable: true,
            querying: false,
            size: Size::default(),
            reset_on_cancel: true,
            _search_task: Task::ready(()),
            _load_more_task: Task::ready(()),
            _query_input_subscription,
        }
    }

    /// Set the size
    pub fn set_size(&mut self, size: Size, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = &self.query_input {
            input.update(cx, |input, cx| {
                input.set_size(size, window, cx);
            })
        }
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
        query_input: Entity<TextInput>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._query_input_subscription =
            cx.subscribe_in(&query_input, window, Self::on_query_input_event);
        self.query_input = Some(query_input);
    }

    /// Get the query input entity.
    pub fn query_input(&self) -> Option<&Entity<TextInput>> {
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

    /// Set the selected index of the list, this will also scroll to the selected item.
    pub fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_index = ix;
        self.delegate.set_selected_index(ix, window, cx);
        self.scroll_to_selected_item(window, cx);
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    fn render_scrollbar(&self, _: &mut Window, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.scrollbar_visible {
            return None;
        }

        Some(Scrollbar::uniform_scroll(
            cx.entity().entity_id(),
            self.scrollbar_state.clone(),
            self.vertical_scroll_handle.clone(),
        ))
    }

    /// Scroll to the item at the given index.
    pub fn scroll_to_item(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical_scroll_handle
            .scroll_to_item(ix, ScrollStrategy::Top);
        cx.notify();
    }

    /// Get scroll handle
    pub fn scroll_handle(&self) -> &UniformListScrollHandle {
        &self.vertical_scroll_handle
    }

    fn scroll_to_selected_item(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(ix) = self.selected_index {
            self.vertical_scroll_handle
                .scroll_to_item(ix, ScrollStrategy::Top);
        }
    }

    fn on_query_input_event(
        &mut self,
        _: &Entity<TextInput>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change(text) => {
                let text = text.trim().to_string();
                if Some(&text) == self.last_query.as_ref() {
                    return;
                }

                self.set_querying(true, window, cx);
                let search = self.delegate.perform_search(&text, window, cx);

                if self.delegate.items_count(cx) > 0 {
                    self.set_selected_index(Some(0), window, cx);
                } else {
                    self.set_selected_index(None, window, cx);
                }

                self._search_task = cx.spawn_in(window, async move |this, window| {
                    search.await;

                    _ = this.update_in(window, |this, _, _| {
                        this.vertical_scroll_handle
                            .scroll_to_item(0, ScrollStrategy::Top);
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

    /// Dispatch delegate's `load_more` method when the visible range is near the end.
    fn load_more_if_need(
        &mut self,
        items_count: usize,
        visible_end: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let threshold = self.delegate.load_more_threshold();
        // Securely handle subtract logic to prevent attempt to subtract with overflow
        if visible_end >= items_count.saturating_sub(threshold) {
            if !self.delegate.can_load_more(cx) {
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
        if self.selected_index.is_none() {
            cx.propagate();
        }

        if self.reset_on_cancel {
            self.set_selected_index(None, window, cx);
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
        if self.delegate.items_count(cx) == 0 {
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

    fn select_item(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_index = Some(ix);
        self.delegate.set_selected_index(Some(ix), window, cx);
        self.scroll_to_selected_item(window, cx);
        cx.emit(ListEvent::Select(ix));
        cx.notify();
    }

    fn on_action_select_prev(
        &mut self,
        _: &SelectPrev,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items_count = self.delegate.items_count(cx);
        if items_count == 0 {
            return;
        }

        let mut selected_index = self.selected_index.unwrap_or(0);
        if selected_index > 0 {
            selected_index = selected_index - 1;
        } else {
            selected_index = items_count - 1;
        }
        self.select_item(selected_index, window, cx);
    }

    fn on_action_select_next(
        &mut self,
        _: &SelectNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items_count = self.delegate.items_count(cx);
        if items_count == 0 {
            return;
        }

        let selected_index;
        if let Some(ix) = self.selected_index {
            if ix < items_count - 1 {
                selected_index = ix + 1;
            } else {
                // When the last item is selected, select the first item.
                selected_index = 0;
            }
        } else {
            // When no selected index, select the first item.
            selected_index = 0;
        }

        self.select_item(selected_index, window, cx);
    }

    fn render_list_item(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_index == Some(ix);
        let right_clicked = self.right_clicked_index == Some(ix);

        div()
            .id("list-item")
            .w_full()
            .relative()
            .children(self.delegate.render_item(ix, window, cx))
            .when(self.selectable, |this| {
                this.when(selected || right_clicked, |this| {
                    this.child(
                        div()
                            .absolute()
                            .top(px(0.))
                            .left(px(0.))
                            .right(px(0.))
                            .bottom(px(0.))
                            .when(selected, |this| this.bg(cx.theme().list_active))
                            .border_1()
                            .border_color(cx.theme().list_active_border),
                    )
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                        this.right_clicked_index = None;
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
                        this.right_clicked_index = Some(ix);
                        cx.notify();
                    }),
                )
            })
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
        let view = cx.entity().clone();
        let vertical_scroll_handle = self.vertical_scroll_handle.clone();
        let items_count = self.delegate.items_count(cx);
        let loading = self.delegate.loading(cx);
        let sizing_behavior = if self.max_height.is_some() {
            ListSizingBehavior::Infer
        } else {
            ListSizingBehavior::Auto
        };

        let initial_view = if let Some(input) = &self.query_input {
            if input.read(cx).text().is_empty() {
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
                            Size::Small => this.py_0().px_1p5(),
                            _ => this.py_1().px_2(),
                        })
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(input),
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
                            this.child(
                                v_flex()
                                    .flex_grow()
                                    .relative()
                                    .when_some(self.max_height, |this, h| this.max_h(h))
                                    .overflow_hidden()
                                    .when(items_count == 0, |this| {
                                        this.child(self.delegate().render_empty(window, cx))
                                    })
                                    .when(items_count > 0, |this| {
                                        this.child(
                                            uniform_list(view, "uniform-list", items_count, {
                                                move |list, visible_range, window, cx| {
                                                    list.load_more_if_need(
                                                        items_count,
                                                        visible_range.end,
                                                        window,
                                                        cx,
                                                    );

                                                    visible_range
                                                        .map(|ix| {
                                                            list.render_list_item(ix, window, cx)
                                                        })
                                                        .collect::<Vec<_>>()
                                                }
                                            })
                                            .flex_grow()
                                            .with_sizing_behavior(sizing_behavior)
                                            .track_scroll(vertical_scroll_handle)
                                            .into_any_element(),
                                        )
                                    })
                                    .children(self.render_scrollbar(window, cx)),
                            )
                        }
                    })
                    // Click out to cancel right clicked row
                    .when(self.right_clicked_index.is_some(), |this| {
                        this.on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.right_clicked_index = None;
                            cx.notify();
                        }))
                    })
            })
    }
}

use gpui::{
    canvas, deferred, div, prelude::FluentBuilder, px, relative, Action, AnyElement, App,
    AppContext, Bounds, Context, DismissEvent, Empty, Entity, EntityInputHandler, EventEmitter,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, Point, Render, RenderOnce,
    SharedString, Styled, Subscription, Window,
};
use lsp_types::CompletionItem;

const MAX_MENU_WIDTH: Pixels = px(320.);
const MAX_MENU_HEIGHT: Pixels = px(480.);

use crate::{
    actions, h_flex,
    input::{self, InputState},
    label::Label,
    list::{List, ListDelegate, ListEvent},
    ActiveTheme, IndexPath, Selectable,
};

struct ContextMenuDelegate {
    query: SharedString,
    menu: Entity<CompletionMenu>,
    items: Vec<CompletionItem>,
    selected_ix: usize,
}

impl ContextMenuDelegate {
    fn set_items(&mut self, items: Vec<CompletionItem>) {
        self.items = items;
        self.selected_ix = 0;
    }

    fn selected_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected_ix)
    }
}

#[derive(IntoElement)]
struct CompletionMenuItem {
    children: Vec<AnyElement>,
    selected: bool,
}

impl CompletionMenuItem {
    pub fn new() -> Self {
        Self {
            children: vec![],
            selected: false,
        }
    }
}
impl Selectable for CompletionMenuItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl ParentElement for CompletionMenuItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl RenderOnce for CompletionMenuItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .py(px(2.))
            .px_2()
            .text_xs()
            .line_height(relative(1.))
            .rounded_sm()
            .children(self.children)
            .when(self.selected, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
    }
}

impl EventEmitter<DismissEvent> for ContextMenuDelegate {}

impl ListDelegate for ContextMenuDelegate {
    type Item = CompletionMenuItem;

    fn items_count(&self, _: usize, _: &gpui::App) -> usize {
        self.items.len()
    }

    fn render_item(
        &self,
        ix: crate::IndexPath,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;
        let deprecated = item.deprecated.unwrap_or(false);

        Some(
            CompletionMenuItem::new()
                .child(
                    Label::new(item.label.clone())
                        .when(deprecated, |this| this.line_through())
                        .highlights(self.query.clone()),
                )
                .when(item.detail.is_some(), |this| {
                    this.child(
                        Label::new(item.detail.as_deref().unwrap_or("").to_string())
                            .text_color(cx.theme().muted_foreground)
                            .when(deprecated, |this| this.line_through())
                            .italic(),
                    )
                }),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<crate::IndexPath>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_ix = ix.map(|i| i.row).unwrap_or(0);

        cx.notify();
    }

    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        let Some(item) = self.selected_item() else {
            return;
        };

        self.menu.update(cx, |this, cx| {
            this.select_item(&item, window, cx);
        });
    }
}

/// A context menu for code completions and code actions.
pub struct CompletionMenu {
    offset: usize,
    state: Entity<InputState>,
    list: Entity<List<ContextMenuDelegate>>,
    open: bool,
    bounds: Bounds<Pixels>,

    /// The offset of the first character that triggered the completion.
    pub(super) trigger_start_offset: Option<usize>,
    query: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl CompletionMenu {
    /// Creates a new `CompletionMenu` with the given offset and completion items.
    ///
    /// NOTE: This element should not call from InputState::new, unless that will stack overflow.
    pub(super) fn new(
        state: Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let view = cx.entity();
            let menu = ContextMenuDelegate {
                query: SharedString::default(),
                menu: view,
                items: vec![],
                selected_ix: 0,
            };

            let list = cx.new(|cx| {
                List::new(menu, window, cx)
                    .no_query()
                    .max_h(MAX_MENU_HEIGHT)
            });

            let _subscriptions =
                vec![
                    cx.subscribe(&list, |this: &mut Self, _, ev: &ListEvent, cx| {
                        match ev {
                            ListEvent::Confirm(_) => {
                                this.hide(cx);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }),
                ];

            Self {
                offset: 0,
                state,
                list,
                open: false,
                trigger_start_offset: None,
                query: SharedString::default(),
                bounds: Bounds::default(),
                _subscriptions,
            }
        })
    }

    fn select_item(&mut self, item: &CompletionItem, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.trigger_start_offset.unwrap_or(self.offset)..self.offset;
        let insert_text = item
            .insert_text
            .as_deref()
            .unwrap_or(&item.label)
            .to_string();
        let state = self.state.clone();

        cx.spawn_in(window, async move |_, cx| {
            state.update_in(cx, |state, window, cx| {
                state.completion_inserting = true;
                state.replace_text_in_range(
                    Some(state.range_to_utf16(&range)),
                    &insert_text,
                    window,
                    cx,
                );
                state.completion_inserting = false;
                // FIXME: Input not get the focus
                state.focus(window, cx);
            })
        })
        .detach();

        self.hide(cx);
    }

    pub(super) fn handle_action(
        &mut self,
        action: Box<dyn Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.open {
            return false;
        }

        cx.propagate();
        if action.partial_eq(&input::Enter { secondary: false }) {
            self.on_action_enter(window, cx);
        } else if action.partial_eq(&input::Escape) {
            self.on_action_escape(window, cx);
        } else if action.partial_eq(&input::MoveUp) {
            self.on_action_up(window, cx);
        } else if action.partial_eq(&input::MoveDown) {
            self.on_action_down(window, cx);
        } else {
            return false;
        }

        true
    }

    fn on_action_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.list.read(cx).delegate().selected_item().cloned() else {
            return;
        };
        self.select_item(&item, window, cx);
    }

    fn on_action_escape(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.hide(cx);
    }

    fn on_action_up(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |this, cx| {
            this.on_action_select_prev(&actions::SelectPrev, window, cx)
        });
    }

    fn on_action_down(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |this, cx| {
            this.on_action_select_next(&actions::SelectNext, window, cx)
        });
    }

    pub(super) fn is_open(&self) -> bool {
        self.open
    }

    /// Hide the completion menu and reset the trigger start offset.
    pub(super) fn hide(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.trigger_start_offset = None;
        cx.notify();
    }

    /// Sets the trigger start offset if it is not already set.
    pub(super) fn update_query(&mut self, start_offset: usize, query: impl Into<SharedString>) {
        if self.trigger_start_offset.is_none() {
            self.trigger_start_offset = Some(start_offset);
        }
        self.query = query.into();
    }

    pub(super) fn show(
        &mut self,
        offset: usize,
        items: impl Into<Vec<CompletionItem>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = items.into();
        self.offset = offset;
        self.open = true;
        self.list.update(cx, |this, cx| {
            this.delegate_mut().query = self.query.clone();
            this.delegate_mut().set_items(items);
            this.set_selected_index(Some(IndexPath::new(0)), window, cx);
        });

        cx.notify();
    }

    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let state = self.state.read(cx);
        let Some(last_layout) = state.last_layout.as_ref() else {
            return None;
        };

        let line_number_width = last_layout.line_number_width;
        let (_, _, start_pos) = state.line_and_position_for_offset(self.offset.saturating_sub(1));
        start_pos.map(|pos| {
            pos + Point::new(line_number_width, last_layout.line_height)
                + Point::new(px(0.), px(4.))
        })
    }
}

impl Render for CompletionMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        if self.list.read(cx).delegate().items.is_empty() {
            self.open = false;
            return Empty.into_any_element();
        }

        let view = cx.entity();

        let Some(pos) = self.origin(cx) else {
            return Empty.into_any_element();
        };

        let scroll_origin = self.state.read(cx).scroll_handle.offset();
        let max_width = MAX_MENU_WIDTH.min(window.bounds().size.width - pos.x);
        let pos = scroll_origin + pos;

        deferred(
            div()
                .id("completion-menu")
                .absolute()
                .occlude()
                .left(pos.x)
                .top(pos.y)
                .p_1()
                .text_xs()
                .max_w(max_width)
                .min_w(px(120.))
                .text_color(cx.theme().popover_foreground)
                .bg(cx.theme().popover)
                .border_1()
                .border_color(cx.theme().border)
                .rounded(cx.theme().radius)
                .shadow_md()
                .child(self.list.clone())
                .child(
                    canvas(
                        move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                        |_, _, _, _| {},
                    )
                    .top_0()
                    .left_0()
                    .absolute()
                    .size_full(),
                )
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.hide(cx);
                })),
        )
        .into_any_element()
    }
}

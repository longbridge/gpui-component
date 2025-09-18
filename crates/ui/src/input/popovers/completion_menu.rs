use std::rc::Rc;

use gpui::{
    canvas, deferred, div, prelude::FluentBuilder, px, relative, Action, AnyElement, App,
    AppContext, Bounds, Context, DismissEvent, Empty, Entity, EntityInputHandler, EventEmitter,
    HighlightStyle, InteractiveElement as _, IntoElement, ParentElement, Pixels, Point, Render,
    RenderOnce, SharedString, Styled, StyledText, Subscription, Window,
};
use lsp_types::CompletionItem;

const MAX_MENU_WIDTH: Pixels = px(320.);
const MAX_MENU_HEIGHT: Pixels = px(480.);
const POPOVER_GAP: Pixels = px(4.);

use crate::{
    actions, h_flex,
    input::{
        self,
        popovers::{popover, render_markdown},
        InputState,
    },
    label::Label,
    list::{List, ListDelegate, ListEvent},
    ActiveTheme, IndexPath, Selectable,
};

struct ContextMenuDelegate {
    query: SharedString,
    menu: Entity<CompletionMenu>,
    items: Vec<Rc<CompletionItem>>,
    selected_ix: usize,
}

impl ContextMenuDelegate {
    fn set_items(&mut self, items: Vec<CompletionItem>) {
        self.items = items.into_iter().map(Rc::new).collect();
        self.selected_ix = 0;
    }

    fn selected_item(&self) -> Option<&Rc<CompletionItem>> {
        self.items.get(self.selected_ix)
    }
}

#[derive(IntoElement)]
struct CompletionMenuItem {
    ix: usize,
    item: Rc<CompletionItem>,
    children: Vec<AnyElement>,
    selected: bool,
    highlight_prefix_len: usize,
}

impl CompletionMenuItem {
    fn new(ix: usize, item: Rc<CompletionItem>) -> Self {
        Self {
            ix,
            item,
            children: vec![],
            selected: false,
            highlight_prefix_len: 0,
        }
    }

    fn highlight_prefix(mut self, len: usize) -> Self {
        self.highlight_prefix_len = len;
        self
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
        let item = self.item;

        let deprecated = item.deprecated.unwrap_or(false);
        let matched_len = self.highlight_prefix_len;
        let highlights = vec![(
            0..matched_len,
            HighlightStyle {
                color: Some(cx.theme().blue),
                ..Default::default()
            },
        )];

        h_flex()
            .id(self.ix)
            .gap_2()
            .p_1()
            .text_xs()
            .line_height(relative(1.))
            .rounded_sm()
            .when(item.deprecated.unwrap_or(false), |this| this.line_through())
            .hover(|this| this.bg(cx.theme().accent.opacity(0.8)))
            .when(self.selected, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .child(div().child(StyledText::new(item.label.clone()).with_highlights(highlights)))
            .when(item.detail.is_some(), |this| {
                this.child(
                    Label::new(item.detail.as_deref().unwrap_or("").to_string())
                        .text_color(cx.theme().muted_foreground)
                        .when(deprecated, |this| this.line_through())
                        .italic(),
                )
            })
            .children(self.children)
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
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;
        let matched_len = self.query.len();
        Some(CompletionMenuItem::new(ix.row, item.clone()).highlight_prefix(matched_len))
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
    pub(crate) trigger_start_offset: Option<usize>,
    query: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl CompletionMenu {
    /// Creates a new `CompletionMenu` with the given offset and completion items.
    ///
    /// NOTE: This element should not call from InputState::new, unless that will stack overflow.
    pub(crate) fn new(
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

    pub(crate) fn handle_action(
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

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Hide the completion menu and reset the trigger start offset.
    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.trigger_start_offset = None;
        cx.notify();
    }

    /// Sets the trigger start offset if it is not already set.
    pub(crate) fn update_query(&mut self, start_offset: usize, query: impl Into<SharedString>) {
        if self.trigger_start_offset.is_none() {
            self.trigger_start_offset = Some(start_offset);
        }
        self.query = query.into();
    }

    pub(crate) fn show(
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
        let Some(cursor_origin) = last_layout.cursor_bounds.map(|b| b.origin) else {
            return None;
        };

        let scroll_origin = self.state.read(cx).scroll_handle.offset();

        Some(
            scroll_origin + cursor_origin - state.input_bounds.origin
                + Point::new(-px(4.), last_layout.line_height + px(4.)),
        )
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

        let selected_documentation = self
            .list
            .read(cx)
            .delegate()
            .selected_item()
            .and_then(|item| item.documentation.clone());

        let max_width = MAX_MENU_WIDTH.min(window.bounds().size.width - pos.x);
        let vertical_layout = pos.x + MAX_MENU_WIDTH + POPOVER_GAP + MAX_MENU_WIDTH + POPOVER_GAP
            > window.bounds().size.width;

        deferred(
            div()
                .absolute()
                .left(pos.x)
                .top(pos.y)
                .flex()
                .flex_row()
                .gap(POPOVER_GAP)
                .items_start()
                .when(vertical_layout, |this| this.flex_col())
                .child(
                    popover("completion-menu", cx)
                        .max_w(max_width)
                        .min_w(px(120.))
                        .child(self.list.clone())
                        .child(
                            canvas(
                                move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full(),
                        ),
                )
                .when_some(selected_documentation, |this, documentation| {
                    let mut doc = match documentation {
                        lsp_types::Documentation::String(s) => s.clone(),
                        lsp_types::Documentation::MarkupContent(mc) => mc.value.clone(),
                    };
                    if vertical_layout {
                        doc = doc.split("\n").next().unwrap_or_default().to_string();
                    }

                    this.child(
                        div().child(
                            popover("completion-menu", cx)
                                .w(MAX_MENU_WIDTH)
                                .px_2()
                                .child(render_markdown("doc", doc, window, cx)),
                        ),
                    )
                })
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.hide(cx);
                })),
        )
        .into_any_element()
    }
}

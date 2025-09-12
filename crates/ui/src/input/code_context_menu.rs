use gpui::{
    canvas, deferred, div, px, App, AppContext, Bounds, Context, DismissEvent, ElementId, Empty,
    Entity, EventEmitter, InteractiveElement as _, IntoElement, ParentElement, Pixels, Point,
    Render, Styled as _, Window,
};
use lsp_types::CompletionItem;

const MAX_MENU_WIDTH: Pixels = px(380.);
const MAX_MENU_HEIGHT: Pixels = px(480.);

use crate::{
    input::InputState,
    list::{List, ListDelegate, ListItem},
    ActiveTheme,
};

struct ContextMenuDelegate {
    items: Vec<CompletionItem>,
    selected_ix: Option<usize>,
}

impl EventEmitter<DismissEvent> for ContextMenuDelegate {}

impl ListDelegate for ContextMenuDelegate {
    type Item = ListItem;

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
        let is_selected = Some(ix.row) == self.selected_ix;

        Some(
            ListItem::new(ix)
                .py(px(2.))
                .px_1()
                .text_xs()
                .child(item.label.clone())
                .selected(is_selected),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<crate::IndexPath>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_ix = ix.map(|i| i.row);
        cx.notify();
    }
}

/// A context menu for code completions and code actions.
pub struct CompletionMenu {
    offset: usize,
    state: Entity<InputState>,
    list: Entity<List<ContextMenuDelegate>>,
    open: bool,
    bounds: Bounds<Pixels>,
}

impl CompletionMenu {
    /// Creates a new `CompletionMenu` with the given offset and completion items.
    pub fn new(
        state: Entity<InputState>,
        offset: usize,
        items: impl Into<Vec<CompletionItem>>,
        open: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let menu = ContextMenuDelegate {
            items: items.into(),
            selected_ix: None,
        };

        let list = cx.new(|cx| {
            List::new(menu, window, cx)
                .no_query()
                .max_h(MAX_MENU_HEIGHT)
        });

        cx.new(|_| Self {
            offset,
            state,
            list,
            open,
            bounds: Bounds::default(),
        })
    }

    pub fn show(
        &mut self,
        offset: usize,
        items: impl Into<Vec<CompletionItem>>,
        cx: &mut Context<Self>,
    ) {
        let items = items.into();
        self.offset = offset;
        self.open = true;
        self.list.update(cx, |this, _| {
            this.delegate_mut().items = items;
        });

        cx.notify();
    }

    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let state = self.state.read(cx);
        let Some(last_layout) = state.last_layout.as_ref() else {
            return None;
        };

        let line_number_width = last_layout.line_number_width;
        let (_, _, start_pos) = state.line_and_position_for_offset(self.offset);
        start_pos.map(|pos| {
            pos + Point::new(line_number_width, last_layout.line_height)
                + Point::new(px(6.), px(6.))
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
                .px_1()
                .py_0p5()
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
                    this.open = false;
                    cx.notify();
                })),
        )
        .into_any_element()
    }
}

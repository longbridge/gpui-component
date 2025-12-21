use std::{ops::Range, rc::Rc};

use gpui::{
    Action, AnyElement, App, AppContext, Context, DismissEvent, Empty, Entity, EventEmitter,
    HighlightStyle, InteractiveElement as _, IntoElement, ParentElement, Pixels, Point, Render,
    RenderOnce, SharedString, Styled, StyledText, Subscription, Window, deferred, div,
    prelude::FluentBuilder, px,
};
use lsp_types::{CompletionItem, CompletionTextEdit};

const MAX_MENU_WIDTH: Pixels = px(320.);
const MAX_MENU_HEIGHT: Pixels = px(240.);
const POPOVER_GAP: Pixels = px(4.);

use crate::{
    ActiveTheme, IndexPath, Selectable, actions, h_flex,
    input::{
        self, InputState, RopeExt,
        popovers::{editor_popover, render_markdown},
    },
    list::{List, ListDelegate, ListEvent, ListState},
};

struct ContextMenuDelegate {
    query: SharedString,
    menu: Entity<CompletionMenu>,
    items: Vec<Rc<CompletionItem>>,
    selected_ix: usize,
    max_width: Pixels,
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
    highlight_prefix: SharedString,
    max_width: Pixels,
}

impl CompletionMenuItem {
    fn new(ix: usize, item: Rc<CompletionItem>) -> Self {
        Self {
            ix,
            item,
            children: vec![],
            selected: false,
            highlight_prefix: "".into(),
            max_width: MAX_MENU_WIDTH,
        }
    }

    fn highlight_prefix(mut self, s: impl Into<SharedString>) -> Self {
        self.highlight_prefix = s.into();
        self
    }

    fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = width;
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
        let display_label = &self.item.label;
        let query = &self.highlight_prefix;

        let max_chars = (self.max_width.to_f64() / 10.0).floor() as usize;
        let final_text = if display_label.chars().count() > max_chars {
            let truncated: String = display_label.chars().take(max_chars - 3).collect();
            format!("{}...", truncated)
        } else {
            display_label.to_string()
        };

        let indices = compute_match_indices(query, &final_text);

        let highlight_style = HighlightStyle {
            color: Some(cx.theme().blue),
            ..Default::default()
        };
        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
        if !indices.is_empty() {
            let mut start = indices[0];
            let mut end = start + 1;
            for &idx in indices.iter().skip(1) {
                if idx == end {
                    end += 1;
                } else {
                    highlights.push((start..end, highlight_style));
                    start = idx;
                    end = start + 1;
                }
            }
            highlights.push((start..end, highlight_style));
        }

        h_flex()
            .id(self.ix)
            .overflow_hidden()
            .gap_2()
            .p_1()
            .text_xs()
            .rounded_sm()
            .hover(|this| this.bg(cx.theme().accent.opacity(0.8)))
            .when(self.selected, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .child(
                div()
                    .flex_1()
                    .child(StyledText::new(final_text).with_highlights(highlights)),
            )
    }
}

impl EventEmitter<DismissEvent> for ContextMenuDelegate {}

impl ListDelegate for ContextMenuDelegate {
    type Item = CompletionMenuItem;

    fn items_count(&self, _: usize, _: &gpui::App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: crate::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;
        Some(
            CompletionMenuItem::new(ix.row, item.clone())
                .highlight_prefix(self.query.clone())
                .max_width(self.max_width), // Use the field here!
        )
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        div()
            .p_2()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child("Nothing Found")
    }

    fn set_selected_index(
        &mut self,
        ix: Option<crate::IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_ix = ix.map(|i| i.row).unwrap_or(0);
        cx.notify();
    }

    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
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
    editor: Entity<InputState>,
    list: Entity<ListState<ContextMenuDelegate>>,
    open: bool,

    /// The offset of the first character that triggered the completion.
    pub(crate) trigger_start_offset: Option<usize>,
    query: SharedString,
    _subscriptions: Vec<Subscription>,
    max_width: Pixels,
}

impl CompletionMenu {
    /// Creates a new `CompletionMenu` with the given offset and completion items.
    ///
    /// NOTE: This element should not call from InputState::new, unless that will stack overflow.
    pub(crate) fn new(
        editor: Entity<InputState>,
        width: Pixels,

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
                max_width: width,
            };

            let list = cx.new(|cx| ListState::new(menu, window, cx));

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
                editor,
                list,
                open: false,
                trigger_start_offset: None,
                query: SharedString::default(),
                _subscriptions,
                max_width: width,
            }
        })
    }

    fn select_item(&mut self, item: &CompletionItem, window: &mut Window, cx: &mut Context<Self>) {
        let offset = self.offset;
        let item = item.clone();
        let mut range = self.trigger_start_offset.unwrap_or(self.offset)..self.offset;

        let editor = self.editor.clone();

        cx.spawn_in(window, async move |_, cx| {
            editor.update_in(cx, |editor, window, cx| {
                editor.completion_inserting = true;

                let mut new_text = item.label.clone();
                if let Some(text_edit) = item.text_edit.as_ref() {
                    match text_edit {
                        CompletionTextEdit::Edit(edit) => {
                            new_text = edit.new_text.clone();
                            range.start = editor.text.position_to_offset(&edit.range.start);
                            range.end = editor.text.position_to_offset(&edit.range.end);
                        }
                        CompletionTextEdit::InsertAndReplace(edit) => {
                            new_text = edit.new_text.clone();
                            range.start = editor.text.position_to_offset(&edit.replace.start);
                            range.end = editor.text.position_to_offset(&edit.replace.end);
                        }
                    }
                } else if let Some(insert_text) = item.insert_text.clone() {
                    new_text = insert_text;
                    range = offset..offset;
                }

                editor.replace_text_in_range_silent(
                    Some(editor.range_to_utf16(&range)),
                    &new_text,
                    window,
                    cx,
                );
                editor.completion_inserting = false;
                // FIXME: Input not get the focus
                editor.focus(window, cx);
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
        if action.partial_eq(&input::IndentInline) {
            self.on_action_enter(window, cx);
            return true;
        }

        if action.partial_eq(&input::Enter { secondary: false }) {
            self.hide(cx);
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

    pub fn set_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        self.max_width = width;
        self.list.update(cx, |list, _| {
            list.delegate_mut().max_width = width;
        });
        cx.notify();
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
            this.on_action_select_prev(&actions::SelectUp, window, cx)
        });
    }

    fn on_action_down(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |this, cx| {
            this.on_action_select_next(&actions::SelectDown, window, cx)
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
            let longest_ix = items
                .iter()
                .enumerate()
                .max_by_key(|(_, item)| {
                    item.label.len() + item.detail.as_ref().map(|d| d.len()).unwrap_or(0)
                })
                .map(|(ix, _)| ix)
                .unwrap_or(0);

            this.delegate_mut().max_width = self.max_width; // Pass it to delegate
            this.delegate_mut().query = self.query.clone();
            this.delegate_mut().set_items(items);
            this.set_selected_index(Some(IndexPath::new(0)), window, cx);
            this.set_item_to_measure_index(IndexPath::new(longest_ix), window, cx);
        });

        cx.notify();
    }

    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let editor = self.editor.read(cx);
        let Some(last_layout) = editor.last_layout.as_ref() else {
            return None;
        };
        let Some(cursor_origin) = last_layout.cursor_bounds.map(|b| b.origin) else {
            return None;
        };

        let scroll_origin = self.editor.read(cx).scroll_handle.offset();

        Some(
            scroll_origin + cursor_origin - editor.input_bounds.origin
                + Point::new(-px(4.), last_layout.line_height + px(4.)),
        )
    }
}

impl Render for CompletionMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        let Some(pos) = self.origin(cx) else {
            return Empty.into_any_element();
        };

        let editor_origin = self.editor.read(cx).input_bounds.origin;
        let abs_pos = editor_origin + pos;
        let window_size = window.bounds().size;

        let available_space_right = window_size.width - abs_pos.x - POPOVER_GAP;
        let menu_width = self.max_width.min(available_space_right);

        let selected_documentation = self
            .list
            .read(cx)
            .delegate()
            .selected_item()
            .and_then(|item| item.documentation.clone());

        let vertical_layout =
            abs_pos.x + self.max_width + POPOVER_GAP + self.max_width + POPOVER_GAP
                > window_size.width;

        deferred(
            div()
                .absolute()
                .left(pos.x)
                .top(pos.y)
                .flex()
                .gap(POPOVER_GAP)
                .items_start()
                .when(vertical_layout, |this: gpui::Div| this.flex_col())
                .child(
                    editor_popover("completion-menu", cx)
                        .w(menu_width)
                        .max_h(MAX_MENU_HEIGHT)
                        .overflow_hidden()
                        .child(
                            List::new(&self.list)
                                .scrollbar_show(crate::scroll::ScrollbarShow::Always)
                                .max_h(MAX_MENU_HEIGHT)
                                .size_full()
                                .p_1(),
                        ),
                )
                .when_some(selected_documentation, |this, documentation| {
                    let mut doc = match documentation {
                        lsp_types::Documentation::String(s) => s.clone(),
                        lsp_types::Documentation::MarkupContent(mc) => mc.value.clone(),
                    };
                    if vertical_layout {
                        doc = doc.lines().next().unwrap_or_default().to_string();
                    }

                    this.child(
                        editor_popover("completion-menu-doc", cx)
                            .w(self.max_width.min(available_space_right))
                            .max_h(MAX_MENU_HEIGHT)
                            .overflow_hidden()
                            .px_2()
                            .child(render_markdown("doc", doc, window, cx)),
                    )
                })
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.hide(cx);
                })),
        )
        .into_any_element()
    }
}

fn compute_match_indices(query: &str, text: &str) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut text_chars = text.char_indices().peekable();
    for q_char in query.chars() {
        let q_lower = q_char.to_lowercase().next().unwrap_or(q_char);
        while let Some((idx, t_char)) = text_chars.next() {
            if t_char.to_lowercase().next().unwrap_or(t_char) == q_lower {
                indices.push(idx);
                break;
            }
        }
    }
    indices
}

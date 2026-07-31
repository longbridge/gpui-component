use gpui::{
    AnyElement, App, ClickEvent, Context, DismissEvent, Edges, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement, KeyBinding, Length,
    ParentElement, Render, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, anchored, deferred, div, prelude::FluentBuilder, px, rems,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Disableable, ElementExt as _, Icon, IconName, IndexPath, Sizable, Size,
    StyleSized, StyledExt,
    actions::{Cancel, Confirm, SelectDown, SelectUp},
    global_state::GlobalState,
    h_flex,
    input::{clear_button, input_style},
    list::List,
    searchable_list::{
        SearchableListChange, SearchableListDelegate, SearchableListItem, SearchableListState,
    },
    v_flex,
};

// MARK: Public re-exports for back-compat

/// Re-exported for backward compatibility. New code should prefer [`SearchableGroup`].
pub use crate::searchable_list::SearchableGroup as SelectGroup;
/// Re-exported for backward compatibility. New code should prefer [`SearchableListDelegate`].
pub use crate::searchable_list::SearchableListDelegate as SelectDelegate;
/// Re-exported for backward compatibility. New code should prefer [`SearchableListItem`].
pub use crate::searchable_list::SearchableListItem as SelectItem;
/// Re-exported for backward compatibility. New code should prefer [`SearchableListItemElement`].
pub use crate::searchable_list::SearchableListItemElement as SelectListItem;
/// Re-exported for backward compatibility.
pub use crate::searchable_list::SearchableVec;

#[derive(IntoElement)]
pub struct Caret {
    size: Size,
    color: Option<Hsla>,
}

impl Caret {
    /// Create a select caret sized for its trigger.
    pub fn new(size: Size) -> Self {
        Self { size, color: None }
    }

    /// Set the caret color.
    pub fn text_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Caret {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(IconName::ChevronDown)
            .with_size(match self.size {
                Size::XSmall => Size::XSmall,
                Size::Small => Size::Small,
                _ => Size::Medium,
            })
            .when_some(self.color, |this, color| this.text_color(color))
    }
}

const CONTEXT: &str = "Select";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new(
            "secondary-enter",
            Confirm { secondary: true },
            Some(CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ])
}

/// Events emitted by [`SelectState`].
pub enum SelectEvent<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    Confirm(Option<<D::Item as SearchableListItem>::Value>),
}

// MARK: SelectOptions (builder only — applied to SearchableListState during render)

struct SelectOptions {
    style: StyleRefinement,
    size: Size,
    icon: Option<Icon>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    title_prefix: Option<SharedString>,
    search_placeholder: Option<SharedString>,
    menu_width: Length,
    menu_max_h: Length,
    disabled: bool,
    appearance: bool,
}

impl Default for SelectOptions {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: Size::default(),
            icon: None,
            cleanable: false,
            placeholder: None,
            title_prefix: None,
            menu_width: Length::Auto,
            menu_max_h: rems(20.).into(),
            disabled: false,
            appearance: true,
            search_placeholder: None,
        }
    }
}

// MARK: SelectState

/// State of the [`Select`] component.
pub struct SelectState<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    pub(crate) state: SearchableListState<D>,

    // Select-specific fields
    searchable: bool,
    icon: Option<Icon>,
    title_prefix: Option<SharedString>,
}

/// A Select element.
#[derive(IntoElement)]
pub struct Select<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    id: ElementId,
    state: Entity<SelectState<D>>,
    options: SelectOptions,
    empty: Option<Box<dyn Fn(&mut Window, &App) -> AnyElement + 'static>>,
}

impl<D> SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    /// Create a new Select state.
    pub fn new(
        delegate: D,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let weak = cx.entity().downgrade();
        let weak_confirm = weak.clone();
        let weak_cancel = weak.clone();
        let weak_empty = weak;

        let selected_indices = selected_index.into_iter().collect::<Vec<_>>();

        let state = SearchableListState::new(
            delegate,
            selected_indices,
            // on_confirm — commit the selection
            move |selected_index, _secondary, window, cx| {
                cx.defer_in(window, {
                    let weak_confirm = weak_confirm.clone();
                    move |list_state, window, cx| {
                        let mut selection = weak_confirm
                            .upgrade()
                            .map(|e| e.read(cx).state.selection.clone())
                            .unwrap_or_default();

                        let changes = {
                            let mut changes: Vec<SearchableListChange> = selection
                                .iter()
                                .map(|(ix, _)| SearchableListChange::Deselect { index: *ix })
                                .collect();

                            if let Some(ix) = selected_index {
                                changes.push(SearchableListChange::Select { index: ix });
                            }

                            changes
                        };

                        // on_will_change is called directly — entity-handle access would
                        // re-enter the ListState lock that defer_in holds for this callback.
                        list_state
                            .delegate_mut()
                            .delegate
                            .on_will_change(&mut selection, &changes);

                        let new_selection = weak_confirm.update(cx, |this, cx| {
                            this.state.selection = selection;

                            let final_value =
                                this.state.selection.first().map(|(_, i)| i.value().clone());

                            cx.emit(SelectEvent::Confirm(final_value));
                            cx.notify();
                            this.set_open(false, cx);
                            this.focus(window, cx);

                            this.state.selection.clone()
                        });

                        // Sync snapshot and fire on_confirm directly — same re-entrancy guard.
                        if let Ok(new_selection) = new_selection {
                            list_state
                                .delegate_mut()
                                .update_selection_snapshot(new_selection.clone());
                            list_state
                                .delegate_mut()
                                .delegate
                                .on_confirm(&new_selection);
                        }
                    }
                });
            },
            // on_cancel — restore cursor to committed index, close
            move |_final_selected_index, window, cx| {
                cx.defer_in(window, {
                    let weak_cancel = weak_cancel.clone();
                    move |list_state, window, cx| {
                        let committed_ix = weak_cancel
                            .upgrade()
                            .and_then(|e| e.read(cx).state.selection.first().map(|(ix, _)| *ix));

                        list_state.set_selected_index(committed_ix, window, cx);

                        _ = weak_cancel.update(cx, |this, cx| {
                            this.set_open(false, cx);
                            this.focus(window, cx);
                        });
                    }
                });
            },
            // on_render_empty
            move |window, cx| {
                if let Some(empty) = weak_empty
                    .upgrade()
                    .and_then(|e| e.read(cx).state.empty.as_ref().map(|f| f(window, cx)))
                {
                    empty
                } else {
                    h_flex()
                        .justify_center()
                        .py_6()
                        .text_color(cx.theme().muted_foreground.opacity(0.6))
                        .child(Icon::new(IconName::Inbox).size(px(28.)))
                        .into_any_element()
                }
            },
            Self::on_blur,
            window,
            cx,
        );

        Self {
            state,
            searchable: false,
            icon: None,
            title_prefix: None,
        }
    }

    /// Sets whether the dropdown menu is searchable, default is `false`.
    ///
    /// When `true`, a search input appears at the top of the dropdown menu.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set the selected index for the select.
    pub fn set_selected_index(
        &mut self,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.list.update(cx, |list, cx| {
            list._set_selected_index(selected_index, window, cx);
        });

        let item = selected_index
            .and_then(|ix| self.state.list.read(cx).delegate().delegate.item(ix))
            .map(|i| i.clone());

        self.state.selection = match (selected_index, item) {
            (Some(ix), Some(item)) => vec![(ix, item)],
            _ => vec![],
        };
        self.state.sync_snapshot(cx);
    }

    /// Set selected value for the select.
    ///
    /// Clears an active search query before looking up the value, so the lookup uses the
    /// delegate's unfiltered items and their restored index paths. For delegates with
    /// asynchronous search, the selection is updated after the reset search completes.
    /// Clears the current selection when the value is not found.
    pub fn set_selected_value(
        &mut self,
        selected_value: &<D::Item as SearchableListItem>::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (selected_index, can_select_immediately) = {
            let list = self.state.list.read(cx);
            (
                list.delegate().delegate.position(selected_value),
                list.query_input.read(cx).value().is_empty() && !list.is_search_pending(),
            )
        };

        if can_select_immediately {
            self.set_selected_index(selected_index, window, cx);
            return;
        }

        let selected_value = selected_value.clone();
        let weak = cx.entity().downgrade();
        self.state.list.update(cx, |list, cx| {
            list.set_query_with_completion(
                "",
                move |list, window, cx| {
                    let selected_index = list.delegate().delegate.position(&selected_value);
                    let selection = selected_index
                        .and_then(|ix| {
                            list.delegate()
                                .delegate
                                .item(ix)
                                .cloned()
                                .map(|item| (ix, item))
                        })
                        .into_iter()
                        .collect::<Vec<_>>();

                    list._set_selected_index(selected_index, window, cx);
                    // The list entity is already mutably borrowed by the search-completion
                    // callback, so update its adapter snapshot directly instead of calling
                    // `SelectState::set_selected_index`, which would re-enter the list entity.
                    list.delegate_mut()
                        .update_selection_snapshot(selection.clone());

                    _ = weak.update(cx, |select, cx| {
                        select.state.selection = selection;
                        cx.notify();
                    });
                },
                window,
                cx,
            );
        });
    }

    /// Replace the delegate (item data) for the select state.
    pub fn set_items(&mut self, items: D, _: &mut Window, cx: &mut Context<Self>)
    where
        D: SearchableListDelegate + 'static,
    {
        self.state.list.update(cx, |list, _| {
            list.delegate_mut().delegate = items;
        });
    }

    /// Get the current selected index.
    pub fn selected_index(&self, cx: &App) -> Option<IndexPath> {
        self.state.list.read(cx).selected_index()
    }

    /// Get the current selected value.
    pub fn selected_value(&self) -> Option<&<D::Item as SearchableListItem>::Value> {
        self.state.selection.first().map(|(_, i)| i.value())
    }

    /// Focus the select trigger input.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.focus_handle.focus(window, cx);
    }

    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.list.read(cx).is_focused(window, cx)
            || self.state.focus_handle.is_focused(window)
        {
            return;
        }

        let committed_ix = self.state.selection.first().map(|(ix, _)| *ix);
        if self.selected_index(cx) != committed_ix {
            self.state.list.update(cx, |list, cx| {
                list.set_selected_index(committed_ix, window, cx);
            });
        }

        self.set_open(false, cx);
        cx.notify();
    }

    fn up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.open {
            self.set_open(true, cx);
        }

        self.state.list.focus_handle(cx).focus(window, cx);
        cx.propagate();
    }

    fn down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.open {
            self.set_open(true, cx);
        }

        self.state.list.focus_handle(cx).focus(window, cx);
        cx.propagate();
    }

    fn enter(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        cx.propagate();

        if !self.state.open {
            self.set_open(true, cx);
            cx.notify();
        }

        self.state.list.focus_handle(cx).focus(window, cx);
    }

    fn toggle_menu(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        self.set_open(!self.state.open, cx);

        if self.state.open {
            self.state.list.focus_handle(cx).focus(window, cx);
        }

        cx.notify();
    }

    fn escape(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.open {
            cx.propagate();
            return;
        }

        cx.stop_propagation();
        self.set_open(false, cx);
        self.focus(window, cx);
        cx.notify();
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.state.open = open;

        if self.state.open {
            GlobalState::global_mut(cx).register_deferred_popover(&self.state.focus_handle)
        } else {
            GlobalState::global_mut(cx).unregister_deferred_popover(&self.state.focus_handle)
        }

        cx.notify();
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.set_selected_index(None, window, cx);
        cx.emit(SelectEvent::Confirm(None));
    }

    fn display_title(&mut self, _: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let default_title = div().text_color(cx.theme().muted_foreground).child(
            self.state
                .placeholder
                .clone()
                .unwrap_or_else(|| t!("Select.placeholder").into()),
        );

        let Some(selected_index) = self.selected_index(cx) else {
            return default_title;
        };

        let Some(title) = self
            .state
            .list
            .read(cx)
            .delegate()
            .delegate
            .item(selected_index)
            .map(|item| {
                if let Some(el) = item.display_title() {
                    el
                } else if let Some(prefix) = self.title_prefix.as_ref() {
                    format!("{}{}", prefix, item.title()).into_any_element()
                } else {
                    item.title().into_any_element()
                }
            })
        else {
            return default_title;
        };

        div()
            .when(self.state.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .child(title)
    }
}

impl<D> Render for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let searchable = self.searchable;
        let is_focused = self.state.focus_handle.is_focused(window);
        let show_clean = self.state.cleanable && self.selected_index(cx).is_some();
        let bounds = self.state.bounds;
        let allow_open = !(self.state.open || self.state.disabled);
        let outline_visible = self.state.open || (is_focused && !self.state.disabled);
        let popup_radius = cx.theme().radius.min(px(8.));

        let (bg, fg) = input_style(self.state.disabled, cx);

        self.state.list.update(cx, |list, cx| {
            list.set_searchable(searchable, cx);
            list.delegate_mut().size = self.state.size;
        });

        div()
            .size_full()
            .relative()
            .child(
                div()
                    .id("input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_1()
                    .border_color(cx.theme().transparent)
                    .when(self.state.appearance, |this| {
                        this.bg(bg)
                            .text_color(fg)
                            .when(self.state.disabled, |this| this.opacity(0.5))
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius)
                    })
                    .overflow_hidden()
                    .input_size(self.state.size)
                    .input_text_size(self.state.size)
                    .refine_style(&self.state.style)
                    .when(outline_visible, |this| this.focused_border(cx))
                    .when(allow_open, |this| {
                        this.on_click(cx.listener(Self::toggle_menu))
                    })
                    .child(
                        h_flex()
                            .id("inner")
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .child(
                                div()
                                    .id("title")
                                    .w_full()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .truncate()
                                    .child(self.display_title(window, cx)),
                            )
                            .when(show_clean, |this| {
                                this.child(clear_button(cx).map(|this| {
                                    if self.state.disabled {
                                        this.disabled(true)
                                    } else {
                                        this.on_click(cx.listener(Self::clean))
                                    }
                                }))
                            })
                            .when(!show_clean, |this| {
                                let icon = match self.icon.clone() {
                                    Some(icon) => icon
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground)
                                        .into_any_element(),
                                    None => Caret::new(self.state.size)
                                        .text_color(cx.theme().muted_foreground)
                                        .into_any_element(),
                                };

                                this.child(icon)
                            }),
                    )
                    .on_prepaint({
                        let state = cx.entity();
                        move |bounds, _, cx| state.update(cx, |r, _| r.state.bounds = bounds)
                    }),
            )
            .when(self.state.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .map(|this| match self.state.menu_width {
                                    Length::Auto => this.w(bounds.size.width + px(2.)),
                                    Length::Definite(w) => this.w(w),
                                })
                                .child(
                                    v_flex()
                                        .occlude()
                                        .mt_1p5()
                                        .bg(cx.theme().tokens.popover)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .rounded(popup_radius)
                                        .shadow_md()
                                        .child(
                                            List::new(&self.state.list)
                                                .when_some(
                                                    self.state.search_placeholder.clone(),
                                                    |this, placeholder| {
                                                        this.search_placeholder(placeholder)
                                                    },
                                                )
                                                .with_size(self.state.size)
                                                .max_h(self.state.menu_max_h)
                                                .paddings(Edges::all(px(4.))),
                                        ),
                                )
                                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                                    this.escape(&Cancel, window, cx);
                                })),
                        ),
                    )
                    .with_priority(1),
                )
            })
    }
}

impl<D> Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    pub fn new(state: &Entity<SelectState<D>>) -> Self {
        Self {
            id: ("select", state.entity_id()).into(),
            state: state.clone(),
            options: SelectOptions::default(),
            empty: None,
        }
    }

    /// Set the width of the dropdown menu, default: `Length::Auto`.
    pub fn menu_width(mut self, width: impl Into<Length>) -> Self {
        self.options.menu_width = width.into();
        self
    }

    /// Set the max height of the dropdown menu, default: 20rem.
    pub fn menu_max_h(mut self, max_h: impl Into<Length>) -> Self {
        self.options.menu_max_h = max_h.into();
        self
    }

    /// Set the placeholder shown when no value is selected.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    /// Override the trailing icon, replacing the default chevron.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.options.icon = Some(icon.into());
        self
    }

    /// Set a label prefix shown before the selected title in the trigger.
    ///
    /// e.g. `title_prefix("Country: ")` → "Country: United States"
    pub fn title_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.options.title_prefix = Some(prefix.into());
        self
    }

    /// Show a clear button when a value is selected.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.options.cleanable = cleanable;
        self
    }

    /// Set the placeholder text for the search input.
    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.search_placeholder = Some(placeholder.into());
        self
    }

    /// Set the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.options.disabled = disabled;
        self
    }

    /// Set a custom closure that renders the empty-state element.
    pub fn empty<E: IntoElement + 'static>(
        mut self,
        builder: impl Fn(&mut Window, &App) -> E + 'static,
    ) -> Self {
        self.empty = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Control whether the trigger shows a border and background (`true` by default).
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.options.appearance = appearance;
        self
    }
}

impl<D> Sizable for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> EventEmitter<SelectEvent<D>> for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
}

impl<D> EventEmitter<DismissEvent> for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
}

impl<D> Focusable for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.state.open {
            self.state.list.focus_handle(cx)
        } else {
            self.state.focus_handle.clone()
        }
    }
}

impl<D> Styled for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.options.style
    }
}

impl<D> RenderOnce for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.options.disabled;
        let focus_handle = self.state.focus_handle(cx);
        let empty = self.empty;
        let opts = self.options;

        self.state.update(cx, |this, _| {
            this.state.style = opts.style;
            this.state.size = opts.size;
            this.state.cleanable = opts.cleanable;
            this.state.placeholder = opts.placeholder;
            this.state.search_placeholder = opts.search_placeholder;
            this.state.menu_width = opts.menu_width;
            this.state.menu_max_h = opts.menu_max_h;
            this.state.disabled = opts.disabled;
            this.state.appearance = opts.appearance;
            this.icon = opts.icon;
            this.title_prefix = opts.title_prefix;

            if let Some(empty) = empty {
                this.state.empty = Some(empty);
            }
        });

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .when(!disabled, |this| {
                this.track_focus(&focus_handle.tab_stop(true))
            })
            .on_action(window.listener_for(&self.state, SelectState::up))
            .on_action(window.listener_for(&self.state, SelectState::down))
            .on_action(window.listener_for(&self.state, SelectState::enter))
            .on_action(window.listener_for(&self.state, SelectState::escape))
            .size_full()
            .child(self.state)
    }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
        Modifiers, ParentElement as _, Render, SharedString, Styled as _, Task, TestAppContext,
        VisualTestContext, WeakEntity, Window, div, px,
    };

    use crate::{
        IndexPath,
        list::ListState,
        searchable_list::{
            SearchableListAdapter, SearchableListDelegate, SearchableListItem, SearchableVec,
        },
        select::{Select, SelectGroup, SelectState},
    };

    fn filter_select<D>(
        state: &Entity<SelectState<D>>,
        query: &str,
        window: &mut Window,
        cx: &mut App,
    ) where
        D: SearchableListDelegate + 'static,
        <D::Item as SearchableListItem>::Value: PartialEq + Clone,
    {
        state.update(cx, |state, cx| {
            state.state.list.update(cx, |list, cx| {
                list.query_input.update(cx, |input, cx| {
                    input.set_value(query, window, cx);
                });
                let search = list
                    .delegate_mut()
                    .delegate
                    .perform_search(query, window, cx);
                assert!(
                    search.is_ready(),
                    "this helper is only for synchronous test delegates"
                );
            });
        });
    }

    #[gpui::test]
    fn test_select_initial_selection_seeds_cursor(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            let state = cx.new(|cx| SelectState::new(items, Some(IndexPath::new(1)), window, cx));

            assert_eq!(
                state.read(cx).selected_index(cx),
                Some(IndexPath::new(1)),
                "initial cursor should be seeded on ListState so display_title can read it",
            );
            assert_eq!(state.read(cx).selected_value(), Some(&"Go"));
        });
    }

    #[gpui::test]
    fn test_select_initial_grouped_selection_seeds_cursor(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let mut groups: SearchableVec<SelectGroup<&'static str>> = SearchableVec::new(vec![]);
            groups.push(SelectGroup::new("A").items(["Apple", "Avocado"]));
            groups.push(SelectGroup::new("B").items(["Banana", "Blueberry", "Blackberry"]));

            let initial = IndexPath::new(1).section(1);
            let state = cx.new(|cx| SelectState::new(groups, Some(initial), window, cx));

            assert_eq!(state.read(cx).selected_index(cx), Some(initial));
            assert_eq!(state.read(cx).selected_value(), Some(&"Blueberry"));
        });
    }

    #[gpui::test]
    fn test_set_selected_value_resets_search_filter(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| {
            cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Rust", "Go", "C++"]),
                    None,
                    window,
                    cx,
                )
                .searchable(true)
            })
        });

        cx.update(|window, cx| {
            filter_select(&state, "Rust", window, cx);
            state.update(cx, |state, cx| {
                state.set_selected_value(&"Go", window, cx);
            });
        });
        cx.run_until_parked();

        cx.update(|_, cx| {
            let state = state.read(cx);
            assert_eq!(state.selected_value(), Some(&"Go"));
            assert_eq!(state.selected_index(cx), Some(IndexPath::new(1)));
            assert_eq!(state.state.list.read(cx).query_input.read(cx).value(), "");
            assert_eq!(
                state.state.list.read(cx).delegate().delegate.items_count(0),
                3
            );
        });
    }

    #[gpui::test]
    fn test_programmatic_list_query_runs_search_delegate(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| {
            cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Rust", "Go", "C++"]),
                    None,
                    window,
                    cx,
                )
                .searchable(true)
            })
        });

        cx.update(|window, cx| {
            let list = state.read(cx).state.list.clone();
            list.update(cx, |list, cx| {
                list.set_query("Rust", window, cx);
            });
        });
        cx.run_until_parked();

        cx.update(|_, cx| {
            let list = state.read(cx).state.list.read(cx);
            assert_eq!(list.query_input.read(cx).value(), "Rust");
            assert_eq!(list.delegate().delegate.items_count(0), 1);
        });
    }

    #[gpui::test]
    fn test_set_selected_value_without_filter_remains_synchronous(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Rust", "Go", "C++"]),
                    Some(IndexPath::new(0)),
                    window,
                    cx,
                )
            });

            state.update(cx, |state, cx| {
                state.set_selected_value(&"Missing", window, cx);
                assert_eq!(state.selected_value(), None);
                assert_eq!(state.selected_index(cx), None);
                assert!(!state.state.list.read(cx).is_search_pending());
            });
        });
    }

    #[gpui::test]
    fn test_set_selected_value_restores_grouped_index_after_search(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| {
            let mut groups: SearchableVec<SelectGroup<&'static str>> = SearchableVec::new(vec![]);
            groups.push(SelectGroup::new("A").items(["Apple", "Avocado"]));
            groups.push(SelectGroup::new("B").items(["Banana", "Blueberry"]));

            cx.new(|cx| SelectState::new(groups, None, window, cx).searchable(true))
        });

        cx.update(|window, cx| {
            filter_select(&state, "Blue", window, cx);
            state.update(cx, |state, cx| {
                state.set_selected_value(&"Banana", window, cx);
            });
        });
        cx.run_until_parked();

        cx.update(|_, cx| {
            let state = state.read(cx);
            assert_eq!(state.selected_value(), Some(&"Banana"));
            assert_eq!(state.selected_index(cx), Some(IndexPath::new(0).section(1)));
        });
    }

    struct AsyncSearchDelegate {
        items: Vec<&'static str>,
        matched_items: Vec<&'static str>,
        release_search: async_channel::Receiver<()>,
        search_count: Rc<Cell<usize>>,
        list: Option<WeakEntity<ListState<SearchableListAdapter<AsyncSearchDelegate>>>>,
    }

    impl SearchableListDelegate for AsyncSearchDelegate {
        type Item = &'static str;

        fn items_count(&self, _: usize) -> usize {
            self.matched_items.len()
        }

        fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
            self.matched_items.get(ix.row)
        }

        fn position<V>(&self, value: &V) -> Option<IndexPath>
        where
            Self::Item: SearchableListItem<Value = V>,
            V: PartialEq,
        {
            self.matched_items
                .iter()
                .position(|item| item.value() == value)
                .map(IndexPath::new)
        }

        fn perform_search(&mut self, query: &str, _: &mut Window, cx: &mut App) -> Task<()> {
            self.search_count.set(self.search_count.get() + 1);

            let items = self.items.clone();
            let query = query.to_string();
            let release_search = self.release_search.clone();
            let list = self.list.clone().expect("test list handle is initialized");

            cx.spawn(async move |cx| {
                release_search
                    .recv()
                    .await
                    .expect("test search gate should stay open");

                _ = list.update(cx, |list, cx| {
                    list.delegate_mut().delegate.matched_items = items
                        .into_iter()
                        .filter(|item| item.matches(&query))
                        .collect();
                    cx.notify();
                });
            })
        }
    }

    #[gpui::test]
    fn test_latest_set_selected_value_waits_for_async_filter_reset(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let (release_search, wait_for_search) = async_channel::unbounded();
        let search_count = Rc::new(Cell::new(0));
        let state = cx.update(|window, cx| {
            let delegate = AsyncSearchDelegate {
                items: vec!["Rust", "Go", "C++"],
                matched_items: vec!["Rust"],
                release_search: wait_for_search,
                search_count: search_count.clone(),
                list: None,
            };
            let state = cx.new(|cx| SelectState::new(delegate, None, window, cx).searchable(true));
            let list = state.read(cx).state.list.clone();

            list.update(cx, |list_state, cx| {
                list_state.delegate_mut().delegate.list = Some(list.downgrade());
                list_state.query_input.update(cx, |input, cx| {
                    input.set_value("Rust", window, cx);
                });
            });

            state
        });

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_selected_value(&"Go", window, cx);
            });
        });
        cx.run_until_parked();
        assert_eq!(search_count.get(), 1);

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_selected_value(&"C++", window, cx);
            });
        });
        cx.run_until_parked();

        cx.update(|_, cx| {
            assert_eq!(
                state.read(cx).selected_value(),
                None,
                "selection must not use the still-filtered delegate before async reset completes"
            );
            assert_eq!(search_count.get(), 2);
        });

        release_search
            .try_send(())
            .expect("release the latest reset search");
        cx.run_until_parked();

        cx.update(|_, cx| {
            let state = state.read(cx);
            assert_eq!(state.selected_value(), Some(&"C++"));
            assert_eq!(state.selected_index(cx), Some(IndexPath::new(2)));
        });
    }

    #[derive(Clone)]
    struct VisualItem(&'static str);

    impl SearchableListItem for VisualItem {
        type Value = &'static str;

        fn title(&self) -> SharedString {
            self.0.into()
        }

        fn display_title(&self) -> Option<AnyElement> {
            let selector = format!("selected-title-{}", self.0.to_lowercase());
            Some(
                div()
                    .debug_selector(move || selector)
                    .child(self.0)
                    .into_any_element(),
            )
        }

        fn value(&self) -> &Self::Value {
            &self.0
        }
    }

    struct SelectResetVisualTest {
        select: Entity<SelectState<SearchableVec<VisualItem>>>,
    }

    impl Render for SelectResetVisualTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(320.))
                .h(px(36.))
                .debug_selector(|| "select-trigger".to_string())
                .child(Select::new(&self.select))
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
    }

    #[gpui::test]
    fn test_set_selected_value_after_real_search_interaction(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state_slot = Rc::new(std::cell::RefCell::new(None));
        let (_, cx) = cx.add_window_view({
            let state_slot = state_slot.clone();
            move |window, cx| {
                let select = cx.new(|cx| {
                    SelectState::new(
                        SearchableVec::new(vec![
                            VisualItem("Rust"),
                            VisualItem("Go"),
                            VisualItem("C++"),
                        ]),
                        None,
                        window,
                        cx,
                    )
                    .searchable(true)
                });
                *state_slot.borrow_mut() = Some(select.clone());
                let content = cx.new(|_| SelectResetVisualTest { select });
                crate::Root::new(content, window, cx)
            }
        });
        let state = state_slot
            .borrow()
            .clone()
            .expect("select state is created with the test window");
        let cx: &mut VisualTestContext = cx;

        draw(cx);
        let trigger = cx
            .debug_bounds("select-trigger")
            .expect("select trigger is rendered");
        cx.simulate_click(trigger.center(), Modifiers::default());
        draw(cx);
        cx.simulate_input("Rust");
        draw(cx);

        cx.update(|_, cx| {
            let state = state.read(cx);
            let list = state.state.list.read(cx);
            assert_eq!(list.query_input.read(cx).value(), "Rust");
            assert_eq!(list.delegate().delegate.items_count(0), 1);
        });

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_selected_value(&"Go", window, cx);
            });
        });
        draw(cx);

        cx.update(|_, cx| {
            let state = state.read(cx);
            let list = state.state.list.read(cx);
            assert_eq!(state.selected_value(), Some(&"Go"));
            assert_eq!(state.selected_index(cx), Some(IndexPath::new(1)));
            assert_eq!(list.query_input.read(cx).value(), "");
            assert_eq!(list.delegate().delegate.items_count(0), 3);
        });
        assert!(
            cx.debug_bounds("selected-title-go").is_some(),
            "the actual Select trigger should render the programmatically selected item"
        );
    }
}

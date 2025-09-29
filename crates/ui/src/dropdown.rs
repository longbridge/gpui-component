use gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder, px, rems, AnyElement, App, AppContext,
    Bounds, ClickEvent, Context, DismissEvent, Edges, ElementId, Empty, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyBinding, Length, ParentElement,
    Pixels, Render, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Subscription, Task, WeakEntity, Window,
};
use rust_i18n::t;

use crate::{
    actions::{Cancel, Confirm, SelectNext, SelectPrev},
    h_flex,
    input::clear_button,
    list::{List, ListDelegate},
    v_flex, ActiveTheme, Disableable, Icon, IconName, IndexPath, Selectable, Sizable, Size,
    StyleSized, StyledExt,
};

#[derive(Clone)]
pub enum ListEvent {
    /// Single click or move to selected row.
    SelectItem(usize),
    /// Double click on the row.
    ConfirmItem(usize),
    // Cancel the selection.
    Cancel,
}

const CONTEXT: &str = "Dropdown";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectPrev, Some(CONTEXT)),
        KeyBinding::new("down", SelectNext, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new(
            "secondary-enter",
            Confirm { secondary: true },
            Some(CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ])
}

/// A trait for items that can be displayed in a dropdown.
pub trait DropdownItem: Clone {
    type Value: Clone;
    fn title(&self) -> SharedString;
    /// Customize the display title used to selected item in Dropdown Input.
    ///
    /// If return None, the title will be used.
    fn display_title(&self) -> Option<AnyElement> {
        None
    }
    fn value(&self) -> &Self::Value;
    /// Check if the item matches the query for search, default is to match the title.
    fn matches(&self, query: &str) -> bool {
        self.title().to_lowercase().contains(&query.to_lowercase())
    }
}

impl DropdownItem for String {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        &self
    }
}

impl DropdownItem for SharedString {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        &self
    }
}

pub trait DropdownDelegate: Sized {
    type Item: DropdownItem;

    /// Returns the number of sections in the dropdown.
    fn sections_count(&self, _: &App) -> usize {
        1
    }

    /// Returns the section header element for the given section index.
    fn section(&self, _section: usize) -> Option<AnyElement> {
        return None;
    }

    /// Returns the number of items in the given section.
    fn items_count(&self, section: usize) -> usize;

    /// Returns the item at the given index path (Only section, row will be use).
    fn item(&self, ix: IndexPath) -> Option<&Self::Item>;

    /// Returns the index of the item with the given value, or None if not found.
    fn position<V>(&self, _value: &V) -> Option<IndexPath>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq;

    fn searchable(&self) -> bool {
        false
    }

    fn perform_search(&mut self, _query: &str, _window: &mut Window, _: &mut App) -> Task<()> {
        Task::ready(())
    }
}

impl<T: DropdownItem> DropdownDelegate for Vec<T> {
    type Item = T;

    fn items_count(&self, _: usize) -> usize {
        self.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.as_slice().get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        self.iter()
            .position(|v| v.value() == value)
            .map(|ix| IndexPath::default().row(ix))
    }
}

struct DropdownListDelegate<D: DropdownDelegate + 'static> {
    delegate: D,
    dropdown: WeakEntity<DropdownState<D>>,
    selected_index: Option<IndexPath>,
}

impl<D> ListDelegate for DropdownListDelegate<D>
where
    D: DropdownDelegate + 'static,
{
    type Item = DropdownListItem;

    fn sections_count(&self, cx: &App) -> usize {
        self.delegate.sections_count(cx)
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.delegate.items_count(section)
    }

    fn render_section_header(
        &self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<impl IntoElement> {
        let dropdown = self.dropdown.upgrade()?.read(cx);
        let Some(item) = self.delegate.section(section) else {
            return None;
        };

        return Some(
            div()
                .py_0p5()
                .px_2()
                .list_size(dropdown.size)
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(item),
        );
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = self
            .selected_index
            .map_or(false, |selected_index| selected_index == ix);
        let size = self
            .dropdown
            .upgrade()
            .map_or(Size::Medium, |dropdown| dropdown.read(cx).size);

        if let Some(item) = self.delegate.item(ix) {
            let list_item = DropdownListItem::new(ix.row)
                .selected(selected)
                .with_size(size)
                .child(div().whitespace_nowrap().child(item.title().to_string()));
            Some(list_item)
        } else {
            None
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        let dropdown = self.dropdown.clone();
        cx.defer_in(window, move |_, window, cx| {
            _ = dropdown.update(cx, |this, cx| {
                this.open = false;
                this.focus(window, cx);
            });
        });
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        let selected_value = self
            .selected_index
            .and_then(|ix| self.delegate.item(ix))
            .map(|item| item.value().clone());
        let dropdown = self.dropdown.clone();

        cx.defer_in(window, move |_, window, cx| {
            _ = dropdown.update(cx, |this, cx| {
                cx.emit(DropdownEvent::Confirm(selected_value.clone()));
                this.selected_value = selected_value;
                this.open = false;
                this.focus(window, cx);
            });
        });
    }

    fn perform_search(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.dropdown.upgrade().map_or(Task::ready(()), |dropdown| {
            dropdown.update(cx, |_, cx| self.delegate.perform_search(query, window, cx))
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn render_empty(&self, window: &mut Window, cx: &mut Context<List<Self>>) -> impl IntoElement {
        if let Some(empty) = self
            .dropdown
            .upgrade()
            .and_then(|dropdown| dropdown.read(cx).empty.as_ref())
        {
            empty(window, cx).into_any_element()
        } else {
            h_flex()
                .justify_center()
                .py_6()
                .text_color(cx.theme().muted_foreground.opacity(0.6))
                .child(Icon::new(IconName::Inbox).size(px(28.)))
                .into_any_element()
        }
    }
}

pub enum DropdownEvent<D: DropdownDelegate + 'static> {
    Confirm(Option<<D::Item as DropdownItem>::Value>),
}

/// State of the [`Dropdown`].
pub struct DropdownState<D: DropdownDelegate + 'static> {
    focus_handle: FocusHandle,
    list: Entity<List<DropdownListDelegate<D>>>,
    size: Size,
    empty: Option<Box<dyn Fn(&Window, &App) -> AnyElement>>,
    /// Store the bounds of the input
    bounds: Bounds<Pixels>,
    open: bool,
    selected_value: Option<<D::Item as DropdownItem>::Value>,
    _subscriptions: Vec<Subscription>,
}

/// A Dropdown element.
#[derive(IntoElement)]
pub struct Dropdown<D: DropdownDelegate + 'static> {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<DropdownState<D>>,
    size: Size,
    icon: Option<Icon>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    title_prefix: Option<SharedString>,
    empty: Option<AnyElement>,
    menu_width: Length,
    disabled: bool,
    appearance: bool,
}

#[derive(Debug, Clone)]
pub struct SearchableVec<T> {
    items: Vec<T>,
    matched_items: Vec<T>,
}

impl<T: Clone> SearchableVec<T> {
    pub fn push(&mut self, item: T) {
        self.items.push(item.clone());
        self.matched_items.push(item);
    }
}

impl<T: Clone> SearchableVec<T> {
    pub fn new(items: impl Into<Vec<T>>) -> Self {
        let items = items.into();
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<T: DropdownItem> From<Vec<T>> for SearchableVec<T> {
    fn from(items: Vec<T>) -> Self {
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<I: DropdownItem> DropdownDelegate for SearchableVec<I> {
    type Item = I;

    fn items_count(&self, _: usize) -> usize {
        self.matched_items.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.matched_items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        for (ix, item) in self.matched_items.iter().enumerate() {
            if item.value() == value {
                return Some(IndexPath::default().row(ix));
            }
        }

        None
    }

    fn searchable(&self) -> bool {
        true
    }

    fn perform_search(&mut self, query: &str, _window: &mut Window, _: &mut App) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.title().to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();

        Task::ready(())
    }
}

impl<I: DropdownItem> DropdownDelegate for SearchableVec<DropdownItemGroup<I>> {
    type Item = I;

    fn sections_count(&self, _: &App) -> usize {
        self.matched_items.len()
    }

    fn items_count(&self, section: usize) -> usize {
        self.matched_items
            .get(section)
            .map_or(0, |group| group.items.len())
    }

    fn section(&self, section: usize) -> Option<AnyElement> {
        Some(
            self.matched_items
                .get(section)?
                .title
                .clone()
                .into_any_element(),
        )
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        let section = self.matched_items.get(ix.section)?;

        section.items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        for (ix, group) in self.matched_items.iter().enumerate() {
            for (row_ix, item) in group.items.iter().enumerate() {
                if item.value() == value {
                    return Some(IndexPath::default().section(ix).row(row_ix));
                }
            }
        }

        None
    }

    fn searchable(&self) -> bool {
        true
    }

    fn perform_search(&mut self, query: &str, _window: &mut Window, _: &mut App) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.matches(&query))
            .cloned()
            .map(|mut item| {
                item.items.retain(|item| item.matches(&query));
                item
            })
            .collect();

        Task::ready(())
    }
}

/// A group of dropdown items with a title.
#[derive(Debug, Clone)]
pub struct DropdownItemGroup<I: DropdownItem> {
    pub title: SharedString,
    pub items: Vec<I>,
}

// impl<I> DropdownItem for DropdownItemGroup<I>
// where
//     I: DropdownItem,
// {
//     type Value = SharedString;

//     fn title(&self) -> SharedString {
//         self.title.clone()
//     }

//     fn value(&self) -> &Self::Value {
//         &self.title
//     }

//     fn matches(&self, query: &str) -> bool {
//         self.title.to_lowercase().contains(&query.to_lowercase())
//             || self.items.iter().any(|item| item.matches(query))
//     }
// }

impl<I> DropdownItemGroup<I>
where
    I: DropdownItem,
{
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            items: vec![],
        }
    }

    pub fn items(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    fn matches(&self, query: &str) -> bool {
        self.title.to_lowercase().contains(&query.to_lowercase())
            || self.items.iter().any(|item| item.matches(query))
    }
}

impl<D> DropdownState<D>
where
    D: DropdownDelegate + 'static,
{
    pub fn new(
        delegate: D,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let delegate = DropdownListDelegate {
            delegate,
            dropdown: cx.entity().downgrade(),
            selected_index,
        };

        let searchable = delegate.delegate.searchable();

        let list = cx.new(|cx| {
            let mut list = List::new(delegate, window, cx)
                .max_h(rems(20.))
                .paddings(Edges::all(px(4.)))
                .reset_on_cancel(false);
            if !searchable {
                list = list.no_query();
            }
            list
        });

        let _subscriptions = vec![
            cx.on_blur(&list.focus_handle(cx), window, Self::on_blur),
            cx.on_blur(&focus_handle, window, Self::on_blur),
        ];

        let mut this = Self {
            focus_handle,
            list,
            size: Size::Medium,
            selected_value: None,
            open: false,
            bounds: Bounds::default(),
            empty: None,
            _subscriptions,
        };
        this.set_selected_index(selected_index, window, cx);
        this
    }

    pub fn empty<E, F>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&Window, &App) -> E + 'static,
    {
        self.empty = Some(Box::new(move |window, cx| f(window, cx).into_any_element()));
        self
    }

    pub fn set_selected_index(
        &mut self,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            list._set_selected_index(selected_index, window, cx);
        });
        self.update_selected_value(window, cx);
    }

    pub fn set_selected_value(
        &mut self,
        selected_value: &<D::Item as DropdownItem>::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        <<D as DropdownDelegate>::Item as DropdownItem>::Value: PartialEq,
    {
        let delegate = self.list.read(cx).delegate();
        let selected_index = delegate.delegate.position(selected_value);
        self.set_selected_index(selected_index, window, cx);
    }

    pub fn selected_index(&self, cx: &App) -> Option<IndexPath> {
        self.list.read(cx).selected_index()
    }

    fn update_selected_value(&mut self, _: &Window, cx: &App) {
        self.selected_value = self
            .selected_index(cx)
            .and_then(|ix| self.list.read(cx).delegate().delegate.item(ix))
            .map(|item| item.value().clone());
    }

    pub fn selected_value(&self) -> Option<&<D::Item as DropdownItem>::Value> {
        self.selected_value.as_ref()
    }

    pub fn focus(&self, window: &mut Window, _: &mut App) {
        self.focus_handle.focus(window);
    }

    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // When the dropdown and dropdown menu are both not focused, close the dropdown menu.
        if self.list.focus_handle(cx).is_focused(window) || self.focus_handle.is_focused(window) {
            return;
        }

        self.open = false;
        cx.notify();
    }

    fn up(&mut self, _: &SelectPrev, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.open = true;
        }

        self.list.focus_handle(cx).focus(window);
        cx.propagate();
    }

    fn down(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.open = true;
        }

        self.list.focus_handle(cx).focus(window);
        cx.propagate();
    }

    fn enter(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        // Propagate the event to the parent view, for example to the Modal to support ENTER to confirm.
        cx.propagate();

        if !self.open {
            self.open = true;
            cx.notify();
        }

        self.list.focus_handle(cx).focus(window);
    }

    fn toggle_menu(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        self.open = !self.open;
        if self.open {
            self.list.focus_handle(cx).focus(window);
        }
        cx.notify();
    }

    fn escape(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
        }

        self.open = false;
        cx.notify();
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.set_selected_index(None, window, cx);
        cx.emit(DropdownEvent::Confirm(None));
    }

    /// Set the items for the dropdown.
    pub fn set_items(&mut self, items: D, _: &mut Window, cx: &mut Context<Self>)
    where
        D: DropdownDelegate + 'static,
    {
        self.list.update(cx, |list, _| {
            list.delegate_mut().delegate = items;
        });
    }
}

impl<D> Render for DropdownState<D>
where
    D: DropdownDelegate + 'static,
{
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl<D> Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    pub fn new(state: &Entity<DropdownState<D>>) -> Self {
        Self {
            id: ("dropdown", state.entity_id()).into(),
            style: StyleRefinement::default(),
            state: state.clone(),
            placeholder: None,
            size: Size::Medium,
            icon: None,
            cleanable: false,
            title_prefix: None,
            empty: None,
            menu_width: Length::Auto,
            disabled: false,
            appearance: true,
        }
    }

    /// Set the width of the dropdown menu, default: Length::Auto
    pub fn menu_width(mut self, width: impl Into<Length>) -> Self {
        self.menu_width = width.into();
        self
    }

    /// Set the placeholder for display when dropdown value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the right icon for the dropdown input, instead of the default arrow icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set title prefix for the dropdown.
    ///
    /// e.g.: Country: United States
    ///
    /// You should set the label is `Country: `
    pub fn title_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.title_prefix = Some(prefix.into());
        self
    }

    /// Set true to show the clear button when the input field is not empty.
    pub fn cleanable(mut self) -> Self {
        self.cleanable = true;
        self
    }

    /// Set the disable state for the dropdown.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn empty(mut self, el: impl IntoElement) -> Self {
        self.empty = Some(el.into_any_element());
        self
    }

    /// Set the appearance of the dropdown, if false the dropdown input will no border, background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Returns the title element for the dropdown input.
    fn display_title(&self, _: &Window, cx: &App) -> impl IntoElement {
        let default_title = div()
            .text_color(cx.theme().accent_foreground)
            .child(
                self.placeholder
                    .clone()
                    .unwrap_or_else(|| t!("Dropdown.placeholder").into()),
            )
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            });

        let Some(selected_index) = &self.state.read(cx).selected_index(cx) else {
            return default_title;
        };

        let Some(title) = self
            .state
            .read(cx)
            .list
            .read(cx)
            .delegate()
            .delegate
            .item(*selected_index)
            .map(|item| {
                if let Some(el) = item.display_title() {
                    el
                } else {
                    if let Some(prefix) = self.title_prefix.as_ref() {
                        format!("{}{}", prefix, item.title()).into_any_element()
                    } else {
                        item.title().into_any_element()
                    }
                }
            })
        else {
            return default_title;
        };

        div()
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .child(title)
    }
}

impl<D> Sizable for Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl<D> EventEmitter<DropdownEvent<D>> for DropdownState<D> where D: DropdownDelegate + 'static {}
impl<D> EventEmitter<DismissEvent> for DropdownState<D> where D: DropdownDelegate + 'static {}
impl<D> Focusable for DropdownState<D>
where
    D: DropdownDelegate,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.open {
            self.list.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl<D> Focusable for Dropdown<D>
where
    D: DropdownDelegate,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl<D> Styled for Dropdown<D>
where
    D: DropdownDelegate,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<D> RenderOnce for Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_focused = self.focus_handle(cx).is_focused(window);
        // If the size has change, set size to self.list, to change the QueryInput size.
        let old_size = self.state.read(cx).list.read(cx).size;
        if old_size != self.size {
            self.state
                .read(cx)
                .list
                .clone()
                .update(cx, |this, cx| this.set_size(self.size, window, cx));
            self.state.update(cx, |this, _| {
                this.size = self.size;
            });
        }

        let state = self.state.read(cx);
        let show_clean = self.cleanable && state.selected_index(cx).is_some();
        let bounds = state.bounds;
        let allow_open = !(state.open || self.disabled);
        let outline_visible = state.open || is_focused && !self.disabled;
        let popup_radius = cx.theme().radius.min(px(8.));

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .when(!self.disabled, |this| {
                this.track_focus(&self.focus_handle(cx).tab_stop(true))
            })
            .on_action(window.listener_for(&self.state, DropdownState::up))
            .on_action(window.listener_for(&self.state, DropdownState::down))
            .on_action(window.listener_for(&self.state, DropdownState::enter))
            .on_action(window.listener_for(&self.state, DropdownState::escape))
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
                    .when(self.appearance, |this| {
                        this.bg(cx.theme().background)
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius)
                            .when(cx.theme().shadow, |this| this.shadow_xs())
                    })
                    .map(|this| {
                        if self.disabled {
                            this.shadow_none()
                        } else {
                            this
                        }
                    })
                    .overflow_hidden()
                    .input_size(self.size)
                    .input_text_size(self.size)
                    .refine_style(&self.style)
                    .when(outline_visible, |this| this.focused_border(cx))
                    .when(allow_open, |this| {
                        this.on_click(window.listener_for(&self.state, DropdownState::toggle_menu))
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
                                    if self.disabled {
                                        this.disabled(true)
                                    } else {
                                        this.on_click(
                                            window.listener_for(&self.state, DropdownState::clean),
                                        )
                                    }
                                }))
                            })
                            .when(!show_clean, |this| {
                                let icon = match self.icon.clone() {
                                    Some(icon) => icon,
                                    None => {
                                        if state.open {
                                            Icon::new(IconName::ChevronUp)
                                        } else {
                                            Icon::new(IconName::ChevronDown)
                                        }
                                    }
                                };

                                this.child(icon.xsmall().text_color(match self.disabled {
                                    true => cx.theme().muted_foreground.opacity(0.5),
                                    false => cx.theme().muted_foreground,
                                }))
                            }),
                    )
                    .child(
                        canvas(
                            {
                                let state = self.state.clone();
                                move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds)
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            )
            .when(state.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .map(|this| match self.menu_width {
                                    Length::Auto => this.w(bounds.size.width + px(2.)),
                                    Length::Definite(w) => this.w(w),
                                })
                                .child(
                                    v_flex()
                                        .occlude()
                                        .mt_1p5()
                                        .bg(cx.theme().background)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .rounded(popup_radius)
                                        .shadow_md()
                                        .child(state.list.clone()),
                                )
                                .on_mouse_down_out(window.listener_for(
                                    &self.state,
                                    |this, _, window, cx| {
                                        this.escape(&Cancel, window, cx);
                                    },
                                )),
                        ),
                    )
                    .with_priority(1),
                )
            })
    }
}

#[derive(IntoElement)]
struct DropdownListItem {
    id: ElementId,
    size: Size,
    style: StyleRefinement,
    selected: bool,
    disabled: bool,
    children: Vec<AnyElement>,
}

impl DropdownListItem {
    pub fn new(ix: usize) -> Self {
        Self {
            id: ("dropdown-item", ix).into(),
            size: Size::default(),
            style: StyleRefinement::default(),
            selected: false,
            disabled: false,
            children: Vec::new(),
        }
    }
}

impl ParentElement for DropdownListItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for DropdownListItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for DropdownListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for DropdownListItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for DropdownListItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DropdownListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .id(self.id)
            .relative()
            .gap_x_1()
            .py_1()
            .px_2()
            .rounded(cx.theme().radius)
            .text_base()
            .text_color(cx.theme().foreground)
            .relative()
            .items_center()
            .justify_between()
            .input_text_size(self.size)
            .list_size(self.size)
            .refine_style(&self.style)
            .when(!self.disabled, |this| {
                this.when(!self.selected, |this| {
                    this.hover(|this| this.bg(cx.theme().accent.alpha(0.7)))
                })
            })
            .when(self.selected, |this| this.bg(cx.theme().accent))
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_x_1()
                    .child(div().w_full().children(self.children)),
            )
    }
}

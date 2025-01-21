use gpui::{
    actions, anchored, canvas, deferred, div, prelude::FluentBuilder, px, rems, AnyElement,
    AppContext, Bounds, ClickEvent, DismissEvent, ElementId, EventEmitter, FocusHandle,
    FocusableView, InteractiveElement, IntoElement, KeyBinding, Length, ParentElement, Pixels,
    Render, SharedString, StatefulInteractiveElement, Styled, Task, View, ViewContext,
    VisualContext, WeakView, WindowContext,
};
use rust_i18n::t;

use crate::{
    h_flex,
    input::ClearButton,
    list::{self, List, ListDelegate, ListItem},
    v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable, Size, StyleSized, StyledExt,
};

actions!(dropdown, [Up, Down, Enter, Escape]);

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
pub fn init(cx: &mut AppContext) {
    cx.bind_keys([
        KeyBinding::new("up", Up, Some(CONTEXT)),
        KeyBinding::new("down", Down, Some(CONTEXT)),
        KeyBinding::new("enter", Enter, Some(CONTEXT)),
        KeyBinding::new("escape", Escape, Some(CONTEXT)),
    ])
}

/// A trait for items that can be displayed in a dropdown.
pub trait DropdownItem {
    type Value: Clone;
    fn title(&self) -> SharedString;
    fn value(&self) -> &Self::Value;
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

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, ix: usize) -> Option<&Self::Item>;

    fn position<V>(&self, value: &V) -> Option<usize>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        (0..self.len()).find(|&i| self.get(i).map_or(false, |item| item.value() == value))
    }

    fn can_search(&self) -> bool {
        false
    }

    fn perform_search(&mut self, _query: &str, _cx: &mut ViewContext<Dropdown<Self>>) -> Task<()> {
        Task::ready(())
    }
}

impl<T: DropdownItem> DropdownDelegate for Vec<T> {
    type Item = T;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, ix: usize) -> Option<&Self::Item> {
        self.as_slice().get(ix)
    }

    fn position<V>(&self, value: &V) -> Option<usize>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        self.iter().position(|v| v.value() == value)
    }
}

struct DropdownListDelegate<D: DropdownDelegate + 'static> {
    delegate: D,
    dropdown: WeakView<Dropdown<D>>,
    selected_index: Option<usize>,
}

impl<D> ListDelegate for DropdownListDelegate<D>
where
    D: DropdownDelegate + 'static,
{
    type Item = ListItem;

    fn items_count(&self, _: &AppContext) -> usize {
        self.delegate.len()
    }

    fn render_item(&self, ix: usize, cx: &mut gpui::ViewContext<List<Self>>) -> Option<Self::Item> {
        let selected = self
            .selected_index
            .map_or(false, |selected_index| selected_index == ix);
        let size = self
            .dropdown
            .upgrade()
            .map_or(Size::Medium, |dropdown| dropdown.read(cx).size);

        if let Some(item) = self.delegate.get(ix) {
            let list_item = ListItem::new(("list-item", ix))
                .check_icon(IconName::Check)
                .cursor_pointer()
                .selected(selected)
                .input_text_size(size)
                .list_size(size)
                .child(div().whitespace_nowrap().child(item.title().to_string()));
            Some(list_item)
        } else {
            None
        }
    }

    fn cancel(&mut self, cx: &mut ViewContext<List<Self>>) {
        let dropdown = self.dropdown.clone();
        cx.defer(move |_, cx| {
            _ = dropdown.update(cx, |this, cx| {
                this.open = false;
                this.focus(cx);
            });
        });
    }

    fn confirm(&mut self, ix: usize, cx: &mut ViewContext<List<Self>>) {
        self.selected_index = Some(ix);

        let selected_value = self
            .selected_index
            .and_then(|ix| self.delegate.get(ix))
            .map(|item| item.value().clone());
        let dropdown = self.dropdown.clone();

        cx.defer(move |_, cx| {
            _ = dropdown.update(cx, |this, cx| {
                cx.emit(DropdownEvent::Confirm(selected_value.clone()));
                this.selected_value = selected_value;
                this.open = false;
                this.focus(cx);
            });
        });
    }

    fn perform_search(&mut self, query: &str, cx: &mut ViewContext<List<Self>>) -> Task<()> {
        self.dropdown.upgrade().map_or(Task::ready(()), |dropdown| {
            dropdown.update(cx, |_, cx| self.delegate.perform_search(query, cx))
        })
    }

    fn set_selected_index(&mut self, ix: Option<usize>, _: &mut ViewContext<List<Self>>) {
        self.selected_index = ix;
    }

    fn render_empty(&self, cx: &mut ViewContext<List<Self>>) -> impl IntoElement {
        if let Some(empty) = self
            .dropdown
            .upgrade()
            .and_then(|dropdown| dropdown.read(cx).empty.as_ref())
        {
            empty(cx).into_any_element()
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

/// A Dropdown element.
pub struct Dropdown<D: DropdownDelegate + 'static> {
    id: ElementId,
    focus_handle: FocusHandle,
    list: View<List<DropdownListDelegate<D>>>,
    size: Size,
    icon: Option<IconName>,
    open: bool,
    cleanable: bool,
    placeholder: Option<SharedString>,
    title_prefix: Option<SharedString>,
    selected_value: Option<<D::Item as DropdownItem>::Value>,
    empty: Option<Box<dyn Fn(&WindowContext) -> AnyElement + 'static>>,
    width: Length,
    menu_width: Length,
    /// Store the bounds of the input
    bounds: Bounds<Pixels>,
    disabled: bool,
}

pub struct SearchableVec<T> {
    items: Vec<T>,
    matched_items: Vec<T>,
}

impl<T: DropdownItem + Clone> SearchableVec<T> {
    pub fn new(items: impl Into<Vec<T>>) -> Self {
        let items = items.into();
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<T: DropdownItem + Clone> DropdownDelegate for SearchableVec<T> {
    type Item = T;

    fn len(&self) -> usize {
        self.matched_items.len()
    }

    fn get(&self, ix: usize) -> Option<&Self::Item> {
        self.matched_items.get(ix)
    }

    fn position<V>(&self, value: &V) -> Option<usize>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        for (ix, item) in self.matched_items.iter().enumerate() {
            if item.value() == value {
                return Some(ix);
            }
        }

        None
    }

    fn can_search(&self) -> bool {
        true
    }

    fn perform_search(&mut self, query: &str, _cx: &mut ViewContext<Dropdown<Self>>) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.title().to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();

        Task::ready(())
    }
}

impl From<Vec<SharedString>> for SearchableVec<SharedString> {
    fn from(items: Vec<SharedString>) -> Self {
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<D> Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    pub fn new(
        id: impl Into<ElementId>,
        delegate: D,
        selected_index: Option<usize>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let delegate = DropdownListDelegate {
            delegate,
            dropdown: cx.view().downgrade(),
            selected_index,
        };

        let searchable = delegate.delegate.can_search();

        let list = cx.new_view(|cx| {
            let mut list = List::new(delegate, cx).max_h(rems(20.));
            if !searchable {
                list = list.no_query();
            }
            list
        });

        cx.on_blur(&list.focus_handle(cx), Self::on_blur).detach();
        cx.on_blur(&focus_handle, Self::on_blur).detach();

        let mut this = Self {
            id: id.into(),
            focus_handle,
            placeholder: None,
            list,
            size: Size::Medium,
            icon: None,
            selected_value: None,
            open: false,
            cleanable: false,
            title_prefix: None,
            empty: None,
            width: Length::Auto,
            menu_width: Length::Auto,
            bounds: Bounds::default(),
            disabled: false,
        };
        this.set_selected_index(selected_index, cx);
        this
    }

    /// Set the width of the dropdown input, default: Length::Auto
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
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
    pub fn icon(mut self, icon: impl Into<IconName>) -> Self {
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

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    pub fn empty<E, F>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&WindowContext) -> E + 'static,
    {
        self.empty = Some(Box::new(move |cx| f(cx).into_any_element()));
        self
    }

    pub fn set_selected_index(
        &mut self,
        selected_index: Option<usize>,
        cx: &mut ViewContext<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            list.set_selected_index(selected_index, cx);
        });
        self.update_selected_value(cx);
    }

    pub fn set_selected_value(
        &mut self,
        selected_value: &<D::Item as DropdownItem>::Value,
        cx: &mut ViewContext<Self>,
    ) where
        <<D as DropdownDelegate>::Item as DropdownItem>::Value: PartialEq,
    {
        let delegate = self.list.read(cx).delegate();
        let selected_index = delegate.delegate.position(selected_value);
        self.set_selected_index(selected_index, cx);
    }

    pub fn selected_index(&self, cx: &WindowContext) -> Option<usize> {
        self.list.read(cx).selected_index()
    }

    fn update_selected_value(&mut self, cx: &WindowContext) {
        self.selected_value = self
            .selected_index(cx)
            .and_then(|ix| self.list.read(cx).delegate().delegate.get(ix))
            .map(|item| item.value().clone());
    }

    pub fn selected_value(&self) -> Option<&<D::Item as DropdownItem>::Value> {
        self.selected_value.as_ref()
    }

    pub fn focus(&self, cx: &mut WindowContext) {
        self.focus_handle.focus(cx);
    }

    fn on_blur(&mut self, cx: &mut ViewContext<Self>) {
        // When the dropdown and dropdown menu are both not focused, close the dropdown menu.
        if self.list.focus_handle(cx).is_focused(cx) || self.focus_handle.is_focused(cx) {
            return;
        }

        self.open = false;
        cx.notify();
    }

    fn up(&mut self, _: &Up, cx: &mut ViewContext<Self>) {
        if !self.open {
            return;
        }
        self.list.focus_handle(cx).focus(cx);
        cx.dispatch_action(Box::new(list::SelectPrev));
    }

    fn down(&mut self, _: &Down, cx: &mut ViewContext<Self>) {
        if !self.open {
            self.open = true;
        }

        self.list.focus_handle(cx).focus(cx);
        cx.dispatch_action(Box::new(list::SelectNext));
    }

    fn enter(&mut self, _: &Enter, cx: &mut ViewContext<Self>) {
        // Propagate the event to the parent view, for example to the Modal to support ENTER to confirm.
        cx.propagate();

        if !self.open {
            self.open = true;
            cx.notify();
        } else {
            self.list.focus_handle(cx).focus(cx);
            cx.dispatch_action(Box::new(list::Confirm));
        }
    }

    fn toggle_menu(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        cx.stop_propagation();

        self.open = !self.open;
        if self.open {
            self.list.focus_handle(cx).focus(cx);
        }
        cx.notify();
    }

    fn escape(&mut self, _: &Escape, cx: &mut ViewContext<Self>) {
        // Propagate the event to the parent view, for example to the Modal to support ESC to close.
        cx.propagate();

        self.open = false;
        cx.notify();
    }

    fn clean(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.set_selected_index(None, cx);
        cx.emit(DropdownEvent::Confirm(None));
    }

    fn display_title(&self, cx: &WindowContext) -> impl IntoElement {
        let title = if let Some(selected_index) = &self.selected_index(cx) {
            let title = self
                .list
                .read(cx)
                .delegate()
                .delegate
                .get(*selected_index)
                .map(|item| item.title().to_string())
                .unwrap_or_default();

            h_flex()
                .when_some(self.title_prefix.clone(), |this, prefix| this.child(prefix))
                .child(title.clone())
        } else {
            div().text_color(cx.theme().accent_foreground).child(
                self.placeholder
                    .clone()
                    .unwrap_or_else(|| t!("Dropdown.placeholder").into()),
            )
        };

        title.when(self.disabled, |this| {
            this.cursor_not_allowed()
                .text_color(cx.theme().muted_foreground)
        })
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

impl<D> EventEmitter<DropdownEvent<D>> for Dropdown<D> where D: DropdownDelegate + 'static {}
impl<D> EventEmitter<DismissEvent> for Dropdown<D> where D: DropdownDelegate + 'static {}
impl<D> FocusableView for Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    fn focus_handle(&self, cx: &AppContext) -> FocusHandle {
        if self.open {
            self.list.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}

impl<D> Render for Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(cx);
        let show_clean = self.cleanable && self.selected_index(cx).is_some();
        let view = cx.view().clone();
        let bounds = self.bounds;
        let allow_open = !(self.open || self.disabled);
        let outline_visible = self.open || is_focused && !self.disabled;

        // If the size has change, set size to self.list, to change the QueryInput size.
        if self.list.read(cx).size != self.size {
            self.list
                .update(cx, |this, cx| this.set_size(self.size, cx))
        }

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::escape))
            .size_full()
            .relative()
            .input_text_size(self.size)
            .child(
                div()
                    .id("dropdown-input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().input)
                    .rounded(px(cx.theme().radius))
                    .when(cx.theme().shadow, |this| this.shadow_sm())
                    .map(|this| {
                        if self.disabled {
                            this.cursor_not_allowed()
                        } else {
                            this.cursor_pointer()
                        }
                    })
                    .overflow_hidden()
                    .input_text_size(self.size)
                    .map(|this| match self.width {
                        Length::Definite(l) => this.flex_none().w(l),
                        Length::Auto => this.w_full(),
                    })
                    .when(outline_visible, |this| this.outline(cx))
                    .input_size(self.size)
                    .when(allow_open, |this| {
                        this.on_click(cx.listener(Self::toggle_menu))
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .child(
                                div()
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.display_title(cx)),
                            )
                            .when(show_clean, |this| {
                                this.child(ClearButton::new(cx).map(|this| {
                                    if self.disabled {
                                        this.disabled(true)
                                    } else {
                                        this.on_click(cx.listener(Self::clean))
                                    }
                                }))
                            })
                            .when(!show_clean, |this| {
                                let icon = match self.icon.clone() {
                                    Some(icon) => icon,
                                    None => {
                                        if self.open {
                                            IconName::ChevronUp
                                        } else {
                                            IconName::ChevronDown
                                        }
                                    }
                                };

                                this.child(
                                    Icon::new(icon)
                                        .xsmall()
                                        .text_color(match self.disabled {
                                            true => cx.theme().muted_foreground.opacity(0.5),
                                            false => cx.theme().muted_foreground,
                                        })
                                        .when(self.disabled, |this| this.cursor_not_allowed()),
                                )
                            }),
                    )
                    .child(
                        canvas(
                            move |bounds, cx| view.update(cx, |r, _| r.bounds = bounds),
                            |_, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            )
            .when(self.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .map(|this| match self.menu_width {
                                    Length::Auto => this.w(bounds.size.width),
                                    Length::Definite(w) => this.w(w),
                                })
                                .child(
                                    v_flex()
                                        .occlude()
                                        .mt_1p5()
                                        .bg(cx.theme().background)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .rounded(px(cx.theme().radius))
                                        .shadow_md()
                                        .on_mouse_down_out(|_, cx| {
                                            cx.dispatch_action(Box::new(Escape));
                                        })
                                        .child(self.list.clone()),
                                )
                                .on_mouse_down_out(cx.listener(|this, _, cx| {
                                    this.escape(&Escape, cx);
                                })),
                        ),
                    )
                    .with_priority(1),
                )
            })
    }
}

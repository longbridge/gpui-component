use gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder, px, rems, AnyElement, App, AppContext,
    Bounds, ClickEvent, Context, DismissEvent, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, Length, ParentElement, Pixels, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Task, WeakEntity, Window,
};
use rust_i18n::t;

use crate::{
    actions::{Cancel, Confirm, SelectNext, SelectPrev},
    h_flex,
    input::clear_button,
    list::{List, ListDelegate, ListItem},
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable, Size, StyleSized, StyledExt,
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
pub fn init(cx: &mut App) {
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
pub trait DropdownItem {
    type Value: Clone;
    fn title(&self) -> SharedString;
    /// Customize the display title used to selected item in Dropdown Input.
    ///
    /// If return None, the title will be used.
    fn display_title(&self) -> Option<AnyElement> {
        None
    }
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

    fn perform_search(
        &mut self,
        _query: &str,
        _window: &mut Window,
        _: &mut Context<Dropdown<Self>>,
    ) -> Task<()> {
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
    dropdown: WeakEntity<Dropdown<D>>,
    selected_index: Option<usize>,
}

impl<D> ListDelegate for DropdownListDelegate<D>
where
    D: DropdownDelegate + 'static,
{
    type Item = ListItem;

    fn items_count(&self, _: &App) -> usize {
        self.delegate.len()
    }

    fn render_item(
        &self,
        ix: usize,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<List<Self>>,
    ) -> Option<Self::Item> {
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
                .selected(selected)
                .input_text_size(size)
                .list_size(size)
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
            .and_then(|ix| self.delegate.get(ix))
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
        ix: Option<usize>,
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

/// A Dropdown element.
pub struct Dropdown<D: DropdownDelegate + 'static> {
    id: ElementId,
    focus_handle: FocusHandle,
    list: Entity<List<DropdownListDelegate<D>>>,
    size: Size,
    icon: Option<IconName>,
    open: bool,
    cleanable: bool,
    placeholder: Option<SharedString>,
    title_prefix: Option<SharedString>,
    selected_value: Option<<D::Item as DropdownItem>::Value>,
    empty: Option<Box<dyn Fn(&Window, &App) -> AnyElement + 'static>>,
    width: Length,
    menu_width: Length,
    /// Store the bounds of the input
    bounds: Bounds<Pixels>,
    disabled: bool,
    _subscriptions: Vec<Subscription>,
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

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _: &mut Context<Dropdown<Self>>,
    ) -> Task<()> {
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let delegate = DropdownListDelegate {
            delegate,
            dropdown: cx.entity().downgrade(),
            selected_index,
        };

        let searchable = delegate.delegate.can_search();

        let list = cx.new(|cx| {
            let mut list = List::new(delegate, window, cx)
                .max_h(rems(20.))
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
            _subscriptions,
        };
        this.set_selected_index(selected_index, window, cx);
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
        F: Fn(&Window, &App) -> E + 'static,
    {
        self.empty = Some(Box::new(move |window, cx| f(window, cx).into_any_element()));
        self
    }

    pub fn set_selected_index(
        &mut self,
        selected_index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            list.set_selected_index(selected_index, window, cx);
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

    pub fn selected_index(&self, cx: &App) -> Option<usize> {
        self.list.read(cx).selected_index()
    }

    fn update_selected_value(&mut self, _: &Window, cx: &App) {
        self.selected_value = self
            .selected_index(cx)
            .and_then(|ix| self.list.read(cx).delegate().delegate.get(ix))
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
            return;
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
        } else {
            self.list.focus_handle(cx).focus(window);
        }
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

        let Some(selected_index) = &self.selected_index(cx) else {
            return default_title;
        };

        let Some(title) = self
            .list
            .read(cx)
            .delegate()
            .delegate
            .get(*selected_index)
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
impl<D> Focusable for Dropdown<D>
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

impl<D> Render for Dropdown<D>
where
    D: DropdownDelegate + 'static,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let show_clean = self.cleanable && self.selected_index(cx).is_some();
        let view = cx.entity().clone();
        let bounds = self.bounds;
        let allow_open = !(self.open || self.disabled);
        let outline_visible = self.open || is_focused && !self.disabled;
        let popup_radius = cx.theme().radius.min(px(8.));

        // If the size has change, set size to self.list, to change the QueryInput size.
        if self.list.read(cx).size != self.size {
            self.list
                .update(cx, |this, cx| this.set_size(self.size, window, cx))
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
                    .id(ElementId::Name(format!("{}-input", self.id).into()))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().input)
                    .rounded(cx.theme().radius)
                    .when(cx.theme().shadow, |this| this.shadow_sm())
                    .map(|this| if self.disabled { this } else { this })
                    .overflow_hidden()
                    .input_text_size(self.size)
                    .map(|this| match self.width {
                        Length::Definite(l) => this.flex_none().w(l),
                        Length::Auto => this.w_full(),
                    })
                    .when(outline_visible, |this| this.focused_border(cx))
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
                                    .whitespace_nowrap()
                                    .truncate()
                                    .child(self.display_title(window, cx)),
                            )
                            .when(show_clean, |this| {
                                this.child(clear_button(cx).map(|this| {
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

                                this.child(Icon::new(icon).xsmall().text_color(
                                    match self.disabled {
                                        true => cx.theme().muted_foreground.opacity(0.5),
                                        false => cx.theme().muted_foreground,
                                    },
                                ))
                            }),
                    )
                    .child(
                        canvas(
                            move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                            |_, _, _, _| {},
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
                                        .rounded(popup_radius)
                                        .shadow_md()
                                        .child(self.list.clone()),
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

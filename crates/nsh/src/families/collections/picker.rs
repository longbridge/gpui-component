use super::schema::{ComboboxEvent as BindingEvent, ComboboxProps};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, SharedString,
    Subscription, Task, Window,
    component::{
        IndexPath,
        combobox::{Combobox, ComboboxEvent, ComboboxState},
        searchable_list::{
            SearchableGroup, SearchableListDelegate, SearchableListItem, SearchableVec,
        },
    },
    prelude::FluentBuilder,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
type ComponentContext = TypedComponentContext<BindingEvent>;
use serde_json::Value;
use std::collections::HashSet;
#[derive(Clone)]
struct Item {
    value: SharedString,
    label: SharedString,
    disabled: bool,
    context: Option<ComponentContext>,
    group: Option<SharedString>,
}
impl SearchableListItem for Item {
    type Value = SharedString;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &SharedString {
        &self.value
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.context
            .as_ref()
            .and_then(|context| crate::bridge::slot(context, &format!("item:{}", self.value)))
            .unwrap_or_else(|| self.label.clone().into_any_element())
    }
    fn display_title(&self) -> Option<AnyElement> {
        self.context
            .as_ref()
            .and_then(|context| crate::bridge::slot(context, &format!("value:{}", self.value)))
    }
}
#[derive(Clone)]
struct Group {
    label: SharedString,
    items: Vec<Item>,
}
enum Items {
    Plain(SearchableVec<Item>),
    Groups(SearchableVec<SearchableGroup<Item>>),
}
impl SearchableListDelegate for Items {
    type Item = Item;
    fn sections_count(&self, cx: &App) -> usize {
        match self {
            Self::Plain(items) => items.sections_count(cx),
            Self::Groups(groups) => groups.sections_count(cx),
        }
    }
    fn items_count(&self, section: usize) -> usize {
        match self {
            Self::Plain(items) => items.items_count(section),
            Self::Groups(groups) => groups.items_count(section),
        }
    }
    fn item(&self, index: IndexPath) -> Option<&Item> {
        match self {
            Self::Plain(items) => items.item(index),
            Self::Groups(groups) => groups.item(index),
        }
    }
    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        match self {
            Self::Plain(items) => items.position(value),
            Self::Groups(groups) => groups.position(value),
        }
    }
    fn perform_search(&mut self, query: &str, window: &mut Window, cx: &mut App) -> Task<()> {
        match self {
            Self::Plain(items) => items.perform_search(query, window, cx),
            Self::Groups(groups) => groups.perform_search(query, window, cx),
        }
    }
    #[allow(deprecated)]
    fn section(&self, section: usize) -> Option<AnyElement> {
        match self {
            Self::Plain(_) => None,
            Self::Groups(groups) => groups.section(section),
        }
    }
    fn render_section_header(
        &self,
        section: usize,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        let item = self.item(IndexPath::new(0).section(section))?;
        let group = item.group.as_ref()?;
        item.context
            .as_ref()
            .and_then(|context| crate::bridge::slot(context, &format!("section:{group}")))
    }
}
type State = ComboboxState<Items>;
pub(super) struct NativeCombobox {
    context: ComponentContext,
    state: Option<Entity<State>>,
    events: Option<Subscription>,
    focus: Option<FocusHandle>,
    items: Option<Vec<Item>>,
    groups: Option<Option<Vec<Group>>>,
    source_items: Vec<Item>,
    source_groups: Option<Vec<Group>>,
    values: Option<Vec<SharedString>>,
    query: Option<String>,
    multiple: bool,
    searchable: bool,
    disabled: bool,
    cleanable: bool,
    placeholder: Option<SharedString>,
    search_placeholder: Option<SharedString>,
}
impl NativeCombobox {
    pub(super) fn new(context: ComponentContext) -> Self {
        Self {
            context,
            state: None,
            events: None,
            focus: None,
            items: None,
            groups: None,
            source_items: Vec::new(),
            source_groups: None,
            values: None,
            query: None,
            multiple: false,
            searchable: true,
            disabled: false,
            cleanable: false,
            placeholder: None,
            search_placeholder: None,
        }
    }
    fn sync(&mut self, window: &mut Window, cx: &mut App) -> Entity<State> {
        let state = self
            .state
            .get_or_insert_with(|| {
                let state = cx.new(|cx| {
                    ComboboxState::new(
                        Items::Plain(SearchableVec::new(Vec::<Item>::new())),
                        Vec::new(),
                        window,
                        cx,
                    )
                });
                let context = self.context.clone();
                self.events = Some(window.subscribe(&state, cx, move |_, event, _, _| {
                    let values = match event {
                        ComboboxEvent::Change(values) | ComboboxEvent::Confirm(values) => {
                            values.iter().map(ToString::to_string).collect()
                        }
                    };
                    context.emit(match event {
                        ComboboxEvent::Change(_) => BindingEvent::Change(values),
                        ComboboxEvent::Confirm(_) => BindingEvent::Confirm(values),
                    });
                }));
                state
            })
            .clone();
        state.update(cx, |state, cx| {
            state.set_multiple(self.multiple, cx);
            state.set_searchable(self.searchable, cx);
            let old = state.selected_values();
            let query = state.query(cx);
            let changed = self.items.is_some() || self.groups.is_some();
            if let Some(items) = self.items.take() {
                self.source_items = items;
            }
            if let Some(groups) = self.groups.take() {
                self.source_groups = groups;
            }
            if changed {
                let items = match &self.source_groups {
                    Some(groups) => Items::Groups(SearchableVec::new(
                        groups
                            .iter()
                            .map(|group| {
                                SearchableGroup::new(group.label.clone()).items(group.items.clone())
                            })
                            .collect::<Vec<_>>(),
                    )),
                    None => Items::Plain(SearchableVec::new(self.source_items.clone())),
                };
                state.set_items(items, window, cx);
            }
            if let Some(values) = self.values.take().or_else(|| changed.then_some(old)) {
                if changed || state.selected_values() != values {
                    state.set_selected_values(&values, window, cx);
                    if !query.is_empty() {
                        state.set_query(query, window, cx);
                    }
                }
            }
            if let Some(query) = self.query.take() {
                if state.query(cx).as_ref() != query {
                    state.set_query(query, window, cx);
                }
            }
        });
        let focus = state.focus_handle(cx);
        if self
            .focus
            .as_ref()
            .is_some_and(|previous| previous != &focus && previous.is_focused(window))
        {
            window.focus(&focus, cx);
        }
        self.focus = Some(focus);
        state
    }
}
impl TypedNativeComponent for NativeCombobox {
    type Props = ComboboxProps;
    type Event = BindingEvent;
    fn render(
        &mut self,
        _: &ComboboxProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self.sync(window, cx);
        let empty = self.context.clone();
        let footer = self.context.clone();
        let trigger = self.context.clone();
        Some(
            Combobox::new(&state)
                .disabled(self.disabled)
                .cleanable(self.cleanable)
                .when_some(self.placeholder.clone(), |this, value| {
                    this.placeholder(value)
                })
                .when_some(self.search_placeholder.clone(), |this, value| {
                    this.search_placeholder(value)
                })
                .when(
                    crate::bridge::slot(&self.context, "empty").is_some(),
                    |this| {
                        this.empty(move |_, _| {
                            crate::bridge::slot(&empty, "empty")
                                .unwrap_or_else(|| gpui_kit::div().into_any_element())
                        })
                    },
                )
                .when(
                    crate::bridge::slot(&self.context, "footer").is_some(),
                    |this| {
                        this.footer(move |_, _| {
                            crate::bridge::slot(&footer, "footer")
                                .unwrap_or_else(|| gpui_kit::div().into_any_element())
                        })
                    },
                )
                .when(
                    crate::bridge::slot(&self.context, "trigger").is_some(),
                    |this| {
                        this.render_trigger(move |_, _, _| {
                            crate::bridge::slot(&trigger, "trigger")
                                .unwrap_or_else(|| gpui_kit::div().into_any_element())
                        })
                    },
                )
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| !self.disabled)
    }
    fn validate_props(&self, p: &ComboboxProps) -> Result<(), nsh_core::ui::PropError> {
        let mut ids = HashSet::new();
        let mut headings = HashSet::new();
        if p.groups.as_ref().is_some_and(|groups| {
            groups.iter().any(|group| {
                !headings.insert(&group.label)
                    || group.items.iter().any(|item| !ids.insert(&item.value))
            })
        }) {
            return Err(nsh_core::ui::PropError(
                "Combobox group labels and item values must be unique".into(),
            ));
        }
        ids.clear();
        if p.items.iter().any(|item| !ids.insert(&item.value)) {
            return Err(nsh_core::ui::PropError(
                "Combobox item values must be unique".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(
        &mut self,
        p: &ComboboxProps,
        changed: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        let convert = |item: &super::schema::PickerItem, group: Option<SharedString>| Item {
            value: item.value.clone().into(),
            label: item.label.clone().into(),
            disabled: item.disabled,
            context: Some(self.context.clone()),
            group,
        };
        if changed.iter().any(|f| f == "items" || f == "groups") {
            self.items = Some(p.items.iter().map(|item| convert(item, None)).collect());
            self.groups = Some(p.groups.as_ref().map(|groups| {
                groups
                    .iter()
                    .map(|group| Group {
                        label: group.label.clone().into(),
                        items: group
                            .items
                            .iter()
                            .map(|item| convert(item, Some(group.label.clone().into())))
                            .collect(),
                    })
                    .collect()
            }));
        }
        if changed.iter().any(|f| f == "values") {
            self.values = Some(
                p.values
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            );
        }
        if changed.iter().any(|f| f == "query") {
            self.query = Some(p.query.clone().unwrap_or_default());
        }
        self.multiple = p.multiple;
        self.searchable = p.searchable;
        self.disabled = p.disabled;
        self.cleanable = p.cleanable;
        self.placeholder = p.placeholder.clone().map(Into::into);
        self.search_placeholder = p.search_placeholder.clone().map(Into::into);
    }
    fn command(
        &mut self,
        _: &ComboboxProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" && !self.disabled {
            let state = self.sync(window, cx);
            window.focus(&state.focus_handle(cx), cx);
        }
    }
}

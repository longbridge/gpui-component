use super::schema::{CommandEvent as BindingEvent, CommandProps};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, SharedString, Styled, Subscription, Window,
    component::{
        Disableable, IndexPath,
        command::{Command, CommandGroup, CommandItem, CommandState},
    },
    div,
    prelude::FluentBuilder,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
type ComponentContext = TypedComponentContext<BindingEvent>;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Clone)]
struct Item {
    id: String,
    label: SharedString,
    group: SharedString,
    disabled: bool,
    checked: bool,
    keywords: Vec<SharedString>,
}
#[derive(Default)]
struct Model {
    items: Vec<Item>,
    paths: Vec<(IndexPath, String)>,
    built: HashMap<String, CommandItem>,
    reported_query: String,
    reported_selection: Option<String>,
}
pub(super) struct NativeCommand {
    context: ComponentContext,
    state: Option<Entity<CommandState>>,
    focus: Option<FocusHandle>,
    focus_subscription: Option<Subscription>,
    active_searchable: bool,
    model: Rc<RefCell<Model>>,
    items: Option<Vec<Item>>,
    query: Option<String>,
    selected: Option<Option<String>>,
    searchable: bool,
    filterable: bool,
    loading: bool,
    placeholder: Option<SharedString>,
}
impl NativeCommand {
    pub(super) fn new(context: ComponentContext) -> Self {
        Self {
            context,
            state: None,
            focus: None,
            focus_subscription: None,
            active_searchable: true,
            model: Rc::default(),
            items: None,
            query: None,
            selected: None,
            searchable: true,
            filterable: true,
            loading: false,
            placeholder: None,
        }
    }
    fn sync(&mut self, window: &mut Window, cx: &mut App) -> Entity<CommandState> {
        let state = self
            .state
            .get_or_insert_with(|| cx.new(|cx| CommandState::new(window, cx)))
            .clone();
        if let Some(items) = self.items.take() {
            let old = state.read(cx).selected_index().and_then(|index| {
                self.model
                    .borrow()
                    .paths
                    .iter()
                    .find(|(path, _)| *path == index)
                    .map(|(_, id)| id.clone())
            });
            if self.selected.is_none() {
                self.selected = Some(old);
            }
            let built = items
                .iter()
                .map(|item| (item.id.clone(), build_item(&self.context, item)))
                .collect();
            self.model.borrow_mut().items = items;
            self.model.borrow_mut().built = built;
        }
        state.update(cx, |state, cx| {
            if let Some(query) = self.query.take() {
                if state.query(cx).as_ref() != query {
                    self.model.borrow_mut().reported_query = query.clone();
                    state.set_query(query, window, cx);
                }
            }
            if state.is_loading() != self.loading {
                state.set_loading(self.loading, window, cx);
            }
        });
        if self.focus.is_none() {
            let focus = cx.focus_handle();
            let observed = focus.clone();
            let state = state.clone();
            self.focus_subscription = Some(window.on_focus_in(&focus, cx, move |window, cx| {
                if observed.is_focused(window) {
                    state.update(cx, |state, cx| state.focus(window, cx));
                }
            }));
            self.focus = Some(focus);
        }
        if self.active_searchable != self.searchable {
            let was_focused = self
                .focus
                .as_ref()
                .is_some_and(|focus| focus.contains_focused(window, cx));
            self.active_searchable = self.searchable;
            if was_focused {
                let state = state.clone();
                window.defer(cx, move |window, cx| {
                    state.update(cx, |state, cx| state.focus(window, cx))
                });
            }
        }
        state
    }
    fn item(&self, item: &Item) -> CommandItem {
        self.model
            .borrow()
            .built
            .get(&item.id)
            .expect("command data is installed together")
            .clone()
    }
}
fn build_item(context: &ComponentContext, item: &Item) -> CommandItem {
    let context = context.clone();
    let id = item.id.clone();
    let label = item.label.clone();
    CommandItem::new()
        .label(item.label.clone())
        .disabled(item.disabled)
        .checked(item.checked)
        .keywords(item.keywords.clone())
        .child(move |_, _| {
            crate::bridge::slot(&context, &format!("item:{id}"))
                .unwrap_or_else(|| label.clone().into_any_element())
        })
}

impl TypedNativeComponent for NativeCommand {
    type Props = CommandProps;
    type Event = BindingEvent;
    fn render(
        &mut self,
        _: &CommandProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self.sync(window, cx);
        let model = self.model.borrow();
        let mut paths = Vec::new();
        let ungrouped: Vec<_> = model
            .items
            .iter()
            .filter(|item| item.group.is_empty())
            .collect();
        let mut groups: Vec<SharedString> = Vec::new();
        for item in &model.items {
            if !item.group.is_empty() && !groups.contains(&item.group) {
                groups.push(item.group.clone());
            }
        }
        let mut command = Command::new(&state)
            .searchable(self.searchable)
            .filterable(self.filterable)
            .items(ungrouped.iter().enumerate().map(|(row, item)| {
                paths.push((IndexPath::new(row), item.id.clone()));
                self.item(item)
            }));
        for (section, group) in groups.iter().enumerate() {
            command = command.group(
                CommandGroup::new().label(group.clone()).items(
                    model
                        .items
                        .iter()
                        .filter(|item| &item.group == group)
                        .enumerate()
                        .map(|(row, item)| {
                            paths.push((
                                IndexPath::new(row)
                                    .section(section + usize::from(!ungrouped.is_empty())),
                                item.id.clone(),
                            ));
                            self.item(item)
                        }),
                ),
            );
        }
        drop(model);
        self.model.borrow_mut().paths = paths;
        let query = self.context.clone();
        let selected = self.context.clone();
        let confirm = self.context.clone();
        let cancel = self.context.clone();
        let query_model = self.model.clone();
        let selected_model = self.model.clone();
        let confirm_model = self.model.clone();
        let header = self.context.clone();
        let footer = self.context.clone();
        let empty = self.context.clone();
        let command = command
            .when_some(self.placeholder.clone(), |this, value| {
                this.placeholder(value)
            })
            .on_query(move |value, _, _| {
                let mut model = query_model.borrow_mut();
                if model.reported_query != value {
                    model.reported_query = value.to_owned();
                    query.emit(BindingEvent::QueryChange(value.to_string()));
                }
            })
            .on_select(move |index, _, _| {
                let mut model = selected_model.borrow_mut();
                let id = model
                    .paths
                    .iter()
                    .find(|(path, _)| *path == index)
                    .map(|(_, id)| id.clone());
                if model.reported_selection != id {
                    model.reported_selection = id.clone();
                    selected.emit(BindingEvent::SelectionChange(id));
                }
            })
            .on_confirm(move |index, _, _| {
                let model = confirm_model.borrow();
                confirm.emit(BindingEvent::Confirm(
                    model
                        .paths
                        .iter()
                        .find(|(path, _)| *path == index)
                        .map(|(_, id)| id.clone()),
                ));
            })
            .on_cancel(move |_, _| cancel.emit(BindingEvent::Cancel))
            .when(
                crate::bridge::slot(&self.context, "header").is_some(),
                |this| {
                    this.header(move |_, _, _| {
                        crate::bridge::slot(&header, "header")
                            .unwrap_or_else(|| div().into_any_element())
                    })
                },
            )
            .when(
                crate::bridge::slot(&self.context, "footer").is_some(),
                |this| {
                    this.footer(move |_, _, _| {
                        crate::bridge::slot(&footer, "footer")
                            .unwrap_or_else(|| div().into_any_element())
                    })
                },
            )
            .when(
                crate::bridge::slot(&self.context, "empty").is_some(),
                |this| {
                    this.empty(move |_, _, _| {
                        crate::bridge::slot(&empty, "empty")
                            .unwrap_or_else(|| div().into_any_element())
                    })
                },
            );
        if let Some(selected) = self.selected.take() {
            self.model.borrow_mut().reported_selection = selected.clone();
            let index = selected.and_then(|id| {
                self.model
                    .borrow()
                    .paths
                    .iter()
                    .find(|(_, value)| *value == id)
                    .map(|(path, _)| *path)
            });
            let state = state.clone();
            window.defer(cx, move |window, cx| {
                state.update(cx, |state, cx| state.set_selected_index(index, window, cx))
            });
        }
        Some(
            div()
                .id("nsh-command")
                .track_focus(self.focus.as_ref().expect("command focus initialized"))
                .size_full()
                .child(command)
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn validate_props(&self, p: &CommandProps) -> Result<(), nsh_core::ui::PropError> {
        let mut ids = HashSet::new();
        if p.items.iter().any(|item| !ids.insert(&item.id)) {
            return Err(nsh_core::ui::PropError(
                "Command item ids must be unique".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(&mut self, p: &CommandProps, changed: &[String], _: &mut Window, _: &mut App) {
        if changed.iter().any(|f| f == "items") {
            self.items = Some(
                p.items
                    .iter()
                    .map(|item| Item {
                        id: item.id.clone(),
                        label: item.label.clone().into(),
                        group: item.group.clone().into(),
                        disabled: item.disabled,
                        checked: item.checked,
                        keywords: item.keywords.iter().cloned().map(Into::into).collect(),
                    })
                    .collect(),
            );
        }
        if changed.iter().any(|f| f == "selectedId") {
            self.selected = Some(p.selected_id.clone());
        }
        if changed.iter().any(|f| f == "query") {
            self.query = Some(p.query.clone().unwrap_or_default());
        }
        self.searchable = p.searchable;
        self.filterable = p.filterable;
        self.loading = p.loading;
        self.placeholder = p.placeholder.clone().map(Into::into);
    }
    fn command(
        &mut self,
        _: &CommandProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" {
            let state = self.sync(window, cx);
            window.focus(&state.focus_handle(cx), cx);
        }
    }
}

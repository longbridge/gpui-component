use super::schema::{ListEvent as BindingEvent, ListProps};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, SharedString, Styled, Task, Window,
    component::{
        IndexPath,
        list::{List, ListDelegate, ListItem, ListState},
    },
    div,
    prelude::FluentBuilder,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
type ComponentContext = TypedComponentContext<BindingEvent>;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone)]
struct Item {
    id: String,
    label: SharedString,
    disabled: bool,
    section: SharedString,
}
struct Delegate {
    context: ComponentContext,
    items: Vec<Item>,
    sections: Vec<SharedString>,
    matched: Vec<Vec<usize>>,
    query: String,
    selected: Option<String>,
    silent: bool,
    loading: bool,
    more: bool,
}
impl Delegate {
    fn rebuild(&mut self) {
        self.sections.clear();
        self.matched.clear();
        let query = self.query.to_lowercase();
        for (ix, item) in self.items.iter().enumerate() {
            if !item.label.to_lowercase().contains(&query) {
                continue;
            }
            let section = self
                .sections
                .iter()
                .position(|section| section == &item.section)
                .unwrap_or_else(|| {
                    self.sections.push(item.section.clone());
                    self.matched.push(Vec::new());
                    self.sections.len() - 1
                });
            self.matched[section].push(ix);
        }
    }
    fn item(&self, ix: IndexPath) -> Option<&Item> {
        self.items.get(*self.matched.get(ix.section)?.get(ix.row)?)
    }
    fn position(&self, id: &str) -> Option<IndexPath> {
        self.matched.iter().enumerate().find_map(|(section, rows)| {
            rows.iter()
                .position(|ix| self.items[*ix].id == id)
                .map(|row| IndexPath::new(row).section(section))
        })
    }
}
impl ListDelegate for Delegate {
    type Item = ListItem;
    fn sections_count(&self, _: &App) -> usize {
        self.sections.len().max(1)
    }
    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched.get(section).map_or(0, Vec::len)
    }
    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<ListItem> {
        let item = self.item(ix)?;
        Some(
            ListItem::new(gpui_kit::SharedString::from(item.id.clone()))
                .accessibility_label(item.label.clone())
                .disabled(item.disabled)
                .child(
                    crate::bridge::slot(&self.context, &format!("item:{}", item.id))
                        .unwrap_or_else(|| item.label.clone().into_any_element()),
                ),
        )
    }
    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let title = self.sections.get(section)?;
        crate::bridge::slot(&self.context, &format!("section:{title}"))
            .or_else(|| (!title.is_empty()).then(|| title.clone().into_any_element()))
    }
    fn render_empty(
        &mut self,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        crate::bridge::slot(&self.context, "empty").unwrap_or_else(|| div().into_any_element())
    }
    fn render_loading(
        &mut self,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        crate::bridge::slot(&self.context, "loading")
            .unwrap_or_else(|| gpui_kit::component::list::Loading.into_any_element())
    }
    fn loading(&self, _: &App) -> bool {
        self.loading
    }
    fn has_more(&self, _: &App) -> bool {
        self.more && !self.loading
    }
    fn load_more(&mut self, _: &mut Window, _: &mut Context<ListState<Self>>) {
        self.context.emit(BindingEvent::LoadMore);
    }
    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_owned();
        self.rebuild();
        if !self.silent {
            self.context
                .emit(BindingEvent::QueryChange(query.to_owned()));
        }
        cx.notify();
        Task::ready(())
    }
    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        if !self.silent
            && ix
                .and_then(|ix| self.item(ix))
                .is_some_and(|item| item.disabled)
        {
            let paths: Vec<IndexPath> = self
                .matched
                .iter()
                .enumerate()
                .flat_map(|(section, rows)| {
                    (0..rows.len()).map(move |row| IndexPath::new(row).section(section))
                })
                .collect();
            let target = ix
                .and_then(|ix| paths.iter().position(|path| *path == ix))
                .unwrap_or_default();
            let old = self
                .selected
                .as_deref()
                .and_then(|id| self.position(id))
                .and_then(|ix| paths.iter().position(|path| *path == ix));
            let backwards = old.is_some_and(|old| (target + 1) % paths.len() == old);
            let next = (1..=paths.len())
                .map(|step| {
                    if backwards {
                        (target + paths.len() - step) % paths.len()
                    } else {
                        (target + step) % paths.len()
                    }
                })
                .map(|index| paths[index])
                .find(|ix| self.item(*ix).is_some_and(|item| !item.disabled));
            cx.defer_in(window, move |state, window, cx| {
                state.set_selected_index(next, window, cx);
                cx.notify();
            });
            return;
        }
        let selected = ix
            .and_then(|ix| self.item(ix))
            .filter(|item| !item.disabled)
            .map(|item| item.id.clone());
        if self.selected != selected {
            self.selected = selected;
            if !self.silent {
                self.context
                    .emit(BindingEvent::SelectionChange(self.selected.clone()));
            }
        }
    }
    fn confirm(&mut self, secondary: bool, _: &mut Window, _: &mut Context<ListState<Self>>) {
        if let Some(id) = &self.selected {
            self.context.emit(BindingEvent::Confirm {
                id: id.clone(),
                secondary,
            });
        }
    }
    fn cancel(&mut self, _: &mut Window, _: &mut Context<ListState<Self>>) {
        self.context.emit(BindingEvent::Cancel);
    }
}

pub(super) struct NativeList {
    context: ComponentContext,
    state: Option<Entity<ListState<Delegate>>>,
    focus: Option<FocusHandle>,
    items: Option<Vec<Item>>,
    selected: Option<Option<String>>,
    query: Option<String>,
    searchable: bool,
    selectable: bool,
    options_dirty: bool,
    loading: bool,
    more: bool,
    placeholder: Option<SharedString>,
}
impl NativeList {
    pub(super) fn new(context: ComponentContext) -> Self {
        Self {
            context,
            state: None,
            focus: None,
            items: None,
            selected: None,
            query: None,
            searchable: false,
            selectable: true,
            options_dirty: true,
            loading: false,
            more: false,
            placeholder: None,
        }
    }
    fn sync(&mut self, window: &mut Window, cx: &mut App) -> Entity<ListState<Delegate>> {
        let state = self
            .state
            .get_or_insert_with(|| {
                cx.new(|cx| {
                    ListState::new(
                        Delegate {
                            context: self.context.clone(),
                            items: Vec::new(),
                            sections: Vec::new(),
                            matched: Vec::new(),
                            query: String::new(),
                            selected: None,
                            silent: false,
                            loading: false,
                            more: false,
                        },
                        window,
                        cx,
                    )
                })
            })
            .clone();
        state.update(cx, |state, cx| {
            state.set_searchable(self.searchable, cx);
            if self.options_dirty {
                state.set_selectable(self.selectable, cx);
            }
            let changed = self.items.is_some() || self.selected.is_some() || self.query.is_some();
            state.delegate_mut().silent = true;
            if let Some(items) = self.items.take() {
                state.delegate_mut().items = items;
                state.delegate_mut().rebuild();
            }
            if let Some(query) = self.query.take() {
                if state.delegate().query != query {
                    state.set_query(&query, window, cx);
                }
            }
            if changed {
                let selected = self
                    .selected
                    .take()
                    .unwrap_or_else(|| state.delegate().selected.clone());
                let index = selected
                    .as_deref()
                    .and_then(|id| state.delegate().position(id));
                state.set_selected_index(index, window, cx);
                cx.notify();
            }
            state.delegate_mut().loading = self.loading;
            state.delegate_mut().more = self.more;
            state.delegate_mut().silent = false;
            self.options_dirty = false;
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
impl TypedNativeComponent for NativeList {
    type Props = ListProps;
    type Event = BindingEvent;
    fn render(&mut self, _: &ListProps, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let state = self.sync(window, cx);
        Some(
            List::new(&state)
                .when_some(self.placeholder.clone(), |this, value| {
                    this.search_placeholder(value)
                })
                .size_full()
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn validate_props(&self, p: &ListProps) -> Result<(), nsh_core::ui::PropError> {
        let mut ids = HashSet::new();
        if p.items.iter().any(|item| !ids.insert(&item.id)) {
            return Err(nsh_core::ui::PropError(
                "List item ids must be unique".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(&mut self, p: &ListProps, changed: &[String], _: &mut Window, _: &mut App) {
        if changed.iter().any(|f| f == "items") {
            self.items = Some(
                p.items
                    .iter()
                    .map(|item| Item {
                        id: item.id.clone(),
                        label: item.label.clone().into(),
                        disabled: item.disabled,
                        section: item.section.clone().into(),
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
        self.selectable = p.selectable;
        self.loading = p.loading;
        self.more = p.has_more;
        self.placeholder = p.placeholder.clone().map(Into::into);
        self.options_dirty = true;
    }
    fn command(
        &mut self,
        _: &ListProps,
        name: &str,
        payload: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.sync(window, cx);
        match name {
            "focus" => window.focus(&state.focus_handle(cx), cx),
            "scrollTo" => {
                let id = payload.as_str().unwrap_or_default();
                state.update(cx, |state, cx| {
                    if let Some(index) = state.delegate().position(id) {
                        state.scroll_to_item(index, gpui_kit::ScrollStrategy::Top, window, cx);
                    }
                });
            }
            _ => {}
        }
    }
}

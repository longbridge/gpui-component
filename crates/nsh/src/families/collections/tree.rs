use super::schema::{TreeEvent as BindingEvent, TreeProps};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Global, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, SharedString, Styled, Subscription, Window,
    base::actions::Confirm,
    component::{
        IconName, h_flex,
        list::ListItem,
        tree::{Tree, TreeEvent, TreeItem, TreeState},
    },
    px,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
type ComponentContext = TypedComponentContext<BindingEvent>;
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[derive(Clone)]
struct Item {
    id: String,
    label: SharedString,
    disabled: bool,
    expanded: bool,
    children: Vec<Item>,
}
fn build(items: &[Item], expanded: &HashMap<String, bool>) -> Vec<TreeItem> {
    items
        .iter()
        .map(|item| {
            TreeItem::new(item.id.clone(), item.label.clone())
                .disabled(item.disabled)
                .expanded(expanded.get(&item.id).copied().unwrap_or(item.expanded))
                .children(build(&item.children, expanded))
        })
        .collect()
}
fn find<'a>(items: &'a [TreeItem], id: &str) -> Option<&'a TreeItem> {
    items.iter().find_map(|item| {
        if item.id.as_ref() == id {
            Some(item)
        } else {
            find(&item.children, id)
        }
    })
}

struct TreeKeysRegistered;
impl Global for TreeKeysRegistered {}

pub(super) struct NativeTree {
    context: ComponentContext,
    state: Option<Entity<TreeState>>,
    focus: Option<FocusHandle>,
    subscriptions: Vec<Subscription>,
    items: Vec<Item>,
    items_dirty: bool,
    pending_expansion: Option<HashSet<String>>,
    selected: Option<Option<String>>,
    expanded: Rc<RefCell<HashMap<String, bool>>>,
    reported: Rc<RefCell<Option<String>>>,
    silent: Rc<Cell<bool>>,
}
impl NativeTree {
    pub(super) fn new(context: ComponentContext) -> Self {
        Self {
            context,
            state: None,
            focus: None,
            subscriptions: Vec::new(),
            items: Vec::new(),
            items_dirty: false,
            pending_expansion: None,
            selected: None,
            expanded: Rc::default(),
            reported: Rc::default(),
            silent: Rc::default(),
        }
    }
    fn sync(&mut self, window: &mut Window, cx: &mut App) -> Entity<TreeState> {
        if !cx.has_global::<TreeKeysRegistered>() {
            cx.bind_keys([KeyBinding::new(
                "enter",
                Confirm { secondary: false },
                Some("NshComponentTree"),
            )]);
            cx.set_global(TreeKeysRegistered);
        }

        let state = self
            .state
            .get_or_insert_with(|| {
                let state = cx.new(|cx| TreeState::new(cx));
                let context = self.context.clone();
                let reported = self.reported.clone();
                self.subscriptions
                    .push(window.observe(&state, cx, move |state, _, cx| {
                        let selected = state
                            .read(cx)
                            .selected_item()
                            .map(|item| item.id.to_string());
                        if *reported.borrow() != selected {
                            *reported.borrow_mut() = selected.clone();
                            context.emit(BindingEvent::SelectionChange(selected));
                        }
                    }));
                let context = self.context.clone();
                let expanded = self.expanded.clone();
                let silent = self.silent.clone();
                self.subscriptions
                    .push(window.subscribe(&state, cx, move |_, event, _, _| {
                        let (id, open) = match event {
                            TreeEvent::Expanded(id) => (id, true),
                            TreeEvent::Collapsed(id) => (id, false),
                        };
                        expanded.borrow_mut().insert(id.to_string(), open);
                        if !silent.get() {
                            context.emit(BindingEvent::ExpandedChange {
                                id: id.to_string(),
                                expanded: open,
                            });
                        }
                    }));
                state
            })
            .clone();
        if self.focus.is_none() {
            let focus = cx.focus_handle();
            let observed = focus.clone();
            let state = state.clone();
            self.subscriptions
                .push(window.on_focus_in(&focus, cx, move |window, cx| {
                    if observed.is_focused(window) {
                        state.update(cx, |state, cx| state.focus(window, cx));
                    }
                }));
            self.focus = Some(focus);
        }
        if let Some(ids) = self.pending_expansion.take() {
            fn apply(items: &[Item], ids: &HashSet<String>, expanded: &mut HashMap<String, bool>) {
                for item in items {
                    expanded.insert(item.id.clone(), ids.contains(&item.id));
                    apply(&item.children, ids, expanded);
                }
            }
            apply(&self.items, &ids, &mut self.expanded.borrow_mut());
            self.items_dirty = true;
        }
        let old = state
            .read(cx)
            .selected_item()
            .map(|item| item.id.to_string());
        if self.items_dirty || self.selected.is_some() {
            let selected = self.selected.take().unwrap_or(old);
            let items = build(&self.items, &self.expanded.borrow());
            self.silent.set(true);
            state.update(cx, |state, cx| {
                if self.items_dirty {
                    state.set_items(items.clone(), cx);
                }
                state.set_selected_item(selected.as_deref().and_then(|id| find(&items, id)), cx);
                *self.reported.borrow_mut() = state.selected_item().map(|item| item.id.to_string());
            });
            self.items_dirty = false;
            let silent = self.silent.clone();
            window.defer(cx, move |_, _| silent.set(false));
        }
        state
    }
}
impl TypedNativeComponent for NativeTree {
    type Props = TreeProps;
    type Event = BindingEvent;
    fn render(&mut self, _: &TreeProps, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let state = self.sync(window, cx);
        let context = self.context.clone();
        let tree = Tree::new(&state, move |_, entry, _, _, _| {
            let item = entry.item();
            let context_confirm = context.clone();
            let id = item.id.clone();
            ListItem::new(item.id.clone())
                .pl(px(12. + entry.depth() as f32 * 16.))
                .child(
                    crate::bridge::slot(&context, &format!("item:{}", item.id)).unwrap_or_else(
                        || {
                            h_flex()
                                .gap_2()
                                .child(if entry.is_folder() {
                                    if entry.is_expanded() {
                                        IconName::FolderOpen
                                    } else {
                                        IconName::Folder
                                    }
                                } else {
                                    IconName::File
                                })
                                .child(item.label.clone())
                                .into_any_element()
                        },
                    ),
                )
                .on_click(move |event, _, _| {
                    if event.click_count() == 2 {
                        context_confirm.emit(BindingEvent::Confirm(id.to_string()));
                    }
                })
        })
        .size_full();
        let confirm = self.context.clone();
        Some(
            gpui_kit::div()
                .id("nsh-tree")
                .track_focus(self.focus.as_ref().expect("tree focus initialized"))
                .key_context("NshComponentTree")
                .size_full()
                .capture_action(move |_: &Confirm, _, cx| {
                    if let Some(item) = state
                        .read(cx)
                        .selected_item()
                        .filter(|item| !item.is_disabled())
                    {
                        confirm.emit(BindingEvent::Confirm(item.id.to_string()));
                    }
                    cx.propagate();
                })
                .child(tree)
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn validate_props(&self, p: &TreeProps) -> Result<(), nsh_core::ui::PropError> {
        fn validate(items: &[super::schema::TreeItem], ids: &mut HashSet<String>) -> bool {
            items
                .iter()
                .all(|item| ids.insert(item.id.clone()) && validate(&item.children, ids))
        }
        if !validate(&p.items, &mut HashSet::new()) {
            return Err(nsh_core::ui::PropError(
                "Tree item ids must be unique across the hierarchy".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(&mut self, p: &TreeProps, changed: &[String], _: &mut Window, _: &mut App) {
        if changed.iter().any(|f| f == "items") {
            fn convert(items: &[super::schema::TreeItem]) -> Vec<Item> {
                items
                    .iter()
                    .map(|item| Item {
                        id: item.id.clone(),
                        label: item.label.clone().into(),
                        disabled: item.disabled,
                        expanded: item.expanded,
                        children: convert(&item.children),
                    })
                    .collect()
            }
            let items = convert(&p.items);
            self.items = items;
            self.items_dirty = true;
        }
        if changed.iter().any(|f| f == "selectedId") {
            self.selected = Some(p.selected_id.clone());
        }
        if changed.iter().any(|f| f == "expandedIds") {
            if let Some(ids) = &p.expanded_ids {
                let ids: HashSet<String> = ids.iter().cloned().collect();
                fn matches(
                    items: &[Item],
                    ids: &HashSet<String>,
                    expanded: &HashMap<String, bool>,
                ) -> bool {
                    items.iter().all(|item| {
                        expanded.get(&item.id).copied().unwrap_or(item.expanded)
                            == ids.contains(&item.id)
                            && matches(&item.children, ids, expanded)
                    })
                }
                if self.items_dirty
                    || self.items.is_empty()
                    || !matches(&self.items, &ids, &self.expanded.borrow())
                {
                    self.pending_expansion = Some(ids);
                }
            }
        }
    }
    fn command(
        &mut self,
        _: &TreeProps,
        name: &str,
        payload: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.sync(window, cx);
        match name {
            "focus" => state.update(cx, |state, cx| state.focus(window, cx)),
            "reveal" => {
                if let Some(id) = payload.as_str() {
                    state.update(cx, |state, cx| {
                        state.reveal_item(&id.to_owned().into(), gpui_kit::ScrollStrategy::Top, cx)
                    });
                }
            }
            _ => {}
        }
    }
}

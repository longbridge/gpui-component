//! Retained popup menus and explicitly leased application menus.
use gpui_kit::{
    Action, AnyElement, App, AsyncApp, AsyncWindowContext, BorrowAppContext, DismissEvent, Entity,
    FocusHandle, Focusable, Global, IntoElement, Menu, MenuItem, OwnedMenu, OwnedMenuItem,
    Subscription, Window,
    component::{
        GlobalState, Side,
        menu::{AppMenuBar, PopupMenu, PopupMenuItem},
    },
    prelude::FluentBuilder,
    px,
};
use nsh_core::ui::{self, Children, Event, JsObject, PropEnum, Props, Readonly};
use nsh_platform_gpui::{ComponentContext, TypedNativeComponent};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    collections::HashSet,
    rc::{Rc, Weak},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_OWNER: AtomicUsize = AtomicUsize::new(1);
#[derive(Action, Clone, PartialEq)]
#[action(no_json)]
struct MenuSelection {
    owner: usize,
    id: String,
}

#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct MenuItemValue {
    #[prop(default)]
    pub(crate) id: String,
    #[prop(default)]
    pub(crate) label: String,
    #[prop(default)]
    pub(crate) disabled: bool,
    #[prop(default)]
    pub(crate) checked: bool,
    #[prop(default)]
    pub(crate) separator: bool,
    #[prop(default)]
    pub(crate) items: Option<Vec<MenuItemValue>>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct ApplicationMenu {
    name: String,
    #[prop(default)]
    disabled: bool,
    items: Readonly<Vec<MenuItemValue>>,
}
pub(crate) fn validate(
    items: &[MenuItemValue],
    ids: &mut HashSet<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > 32 {
        return Err("Menu nesting must not exceed 32 levels".into());
    }
    for item in items {
        if item.separator {
            continue;
        }
        if item.label.is_empty() {
            return Err("Menu items require a nonempty label".into());
        }
        if let Some(children) = &item.items {
            validate(children, ids, depth + 1)?;
        } else if item.id.is_empty() || !ids.insert(item.id.clone()) {
            return Err("Menu action ids must be nonempty and unique".into());
        }
    }
    Ok(())
}
pub(crate) fn emit_event<E: Event>(context: &ComponentContext, event: E) {
    match ui::event_json(&event) {
        Ok(value) => context.emit(event.name(), value.unwrap_or(Value::Null)),
        Err(error) => context.emit("error", json!(error.to_string())),
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum MenuCheckSide {
    Left,
    Right,
}
/// A retained native popup. Dynamic item:<id> slots are live NSH subtrees.
#[derive(Clone, PartialEq, Props)]
pub(crate) struct PopupMenuProps {
    open: bool,
    items: Readonly<Vec<MenuItemValue>>,
    min_width: Option<f64>,
    max_width: Option<f64>,
    max_height: Option<f64>,
    #[prop(default)]
    scrollable: bool,
    check_side: Option<MenuCheckSide>,
    #[prop(default = true)]
    external_link_icon: bool,
    children: Children,
}
#[derive(Event)]
pub(crate) enum PopupMenuEvent {
    Select(String),
    OpenChange(bool),
    /// The shared typed decoder emits domain and codec errors.
    #[allow(dead_code)]
    Error(String),
}
/// Leases application-global menus. The most recently mounted active owner wins.
#[derive(Clone, PartialEq, Props)]
pub(crate) struct AppMenuBarProps {
    menus: Readonly<Vec<ApplicationMenu>>,
    #[prop(default = true)]
    active: bool,
}
#[derive(Event)]
pub(crate) enum AppMenuBarEvent {
    Select(String),
    /// The shared typed decoder emits domain and codec errors.
    #[allow(dead_code)]
    Error(String),
}
struct Session {
    alive: Cell<bool>,
    open: Cell<bool>,
    context: ComponentContext,
}
fn popup_items(
    mut menu: PopupMenu,
    items: &[MenuItemValue],
    session: &Rc<Session>,
    window: &mut Window,
    cx: &mut gpui_kit::Context<PopupMenu>,
) -> PopupMenu {
    for item in items {
        if item.separator {
            menu = menu.separator();
            continue;
        }
        if let Some(items) = &item.items {
            let items = items.clone();
            let weak = Rc::downgrade(session);
            menu = menu.submenu_with_disabled(
                item.label.clone(),
                item.disabled,
                window,
                cx,
                move |menu, window, cx| match weak.upgrade().filter(|session| session.alive.get()) {
                    Some(session) => popup_items(menu, &items, &session, window, cx),
                    None => menu,
                },
            );
        } else {
            let id = item.id.clone();
            let weak = Rc::downgrade(session);
            let context = session.context.clone();
            let slot_name = format!("item:{}", item.id);
            let custom = context.slot(&slot_name).is_some();
            let entry = if custom {
                PopupMenuItem::element(move |_, _| {
                    crate::bridge::slot(&context, &slot_name)
                        .unwrap_or_else(|| gpui_kit::div().into_any_element())
                })
            } else {
                PopupMenuItem::new(item.label.clone())
            };
            menu = menu.item(
                entry
                    .disabled(item.disabled)
                    .checked(item.checked)
                    .on_click(move |_, _, _| {
                        if let Some(session) = weak
                            .upgrade()
                            .filter(|session| session.alive.get() && session.open.get())
                        {
                            emit_event(&session.context, PopupMenuEvent::Select(id.clone()));
                        }
                    }),
            );
        }
    }
    menu
}

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativePopup>("gpui-component.PopupMenu", "PopupMenu", |context| {
        NativePopup::new(context.raw().clone())
    });
    registry
        .metadata()
        .native_controlled("gpui-component.PopupMenu", &[("openChange", &["open"])]);
    registry.register::<NativeMenuBar>("gpui-component.AppMenuBar", "AppMenuBar", |context| {
        NativeMenuBar::new(context.raw().clone())
    });
}
struct NativePopup {
    session: Rc<Session>,
    items: Vec<MenuItemValue>,
    state: Option<Entity<PopupMenu>>,
    focus: Option<FocusHandle>,
    previous: Option<FocusHandle>,
    subscription: Option<Subscription>,
    window: Option<AsyncWindowContext>,
    dirty: bool,
    opening: bool,
    min_width: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    scrollable: bool,
    check_right: bool,
    external_link_icon: bool,
}
impl NativePopup {
    fn new(context: ComponentContext) -> Self {
        Self {
            session: Rc::new(Session {
                alive: Cell::new(true),
                open: Cell::new(false),
                context,
            }),
            items: vec![],
            state: None,
            focus: None,
            previous: None,
            subscription: None,
            window: None,
            dirty: true,
            opening: false,
            min_width: None,
            max_width: None,
            max_height: None,
            scrollable: false,
            check_right: false,
            external_link_icon: true,
        }
    }
    fn close(&mut self, window: &mut Window, cx: &mut App) {
        self.session.open.set(false);
        if self
            .focus
            .as_ref()
            .is_some_and(|focus| focus.contains_focused(window, cx))
        {
            if let Some(previous) = &self.previous {
                previous.focus(window, cx);
            }
        }
        self.session.context.redraw();
    }
}
impl TypedNativeComponent for NativePopup {
    type Props = PopupMenuProps;
    type Event = PopupMenuEvent;
    fn render(
        &mut self,
        _: &PopupMenuProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.window = Some(window.to_async(cx));
        if !self.session.open.get() {
            return None;
        }
        if self.opening {
            self.previous = window.focused(cx);
            self.dirty = true;
        }
        if self.state.is_none() {
            let state = PopupMenu::build(window, cx, |menu, _, _| menu);
            let weak = Rc::downgrade(&self.session);
            self.subscription =
                Some(
                    window.subscribe(&state, cx, move |_, _: &DismissEvent, _, _| {
                        if let Some(session) = weak
                            .upgrade()
                            .filter(|session| session.alive.get() && session.open.replace(false))
                        {
                            emit_event(&session.context, PopupMenuEvent::OpenChange(false));
                            session.context.redraw();
                        }
                    }),
                );
            self.focus = Some(state.focus_handle(cx));
            self.state = Some(state);
        }
        let state = self.state.as_ref().unwrap().clone();
        if self.dirty {
            self.dirty = false;
            state.update(cx, |menu, cx| {
                menu.rebuild(window, cx, |menu, window, cx| {
                    popup_items(menu, &self.items, &self.session, window, cx)
                        .when_some(self.previous.clone(), |menu, previous| {
                            menu.action_context(previous)
                        })
                        .when_some(self.min_width, |menu, width| menu.min_w(px(width)))
                        .when_some(self.max_width, |menu, width| menu.max_w(px(width)))
                        .when_some(self.max_height, |menu, height| menu.max_h(px(height)))
                        .scrollable(self.scrollable)
                        .check_side(if self.check_right {
                            Side::Right
                        } else {
                            Side::Left
                        })
                        .external_link_icon(self.external_link_icon)
                })
            });
        }
        if self.opening {
            self.opening = false;
            let focus = self.focus.clone().unwrap();
            let weak = Rc::downgrade(&self.session);
            window.defer(cx, move |window, cx| {
                if weak
                    .upgrade()
                    .is_some_and(|session| session.alive.get() && session.open.get())
                {
                    focus.focus(window, cx);
                }
            });
        }
        Some(state.into_any_element())
    }
    fn validate_props(&self, props: &PopupMenuProps) -> Result<(), ui::PropError> {
        validate(&props.items, &mut HashSet::new(), 0).map_err(ui::PropError)?;
        if [props.min_width, props.max_width, props.max_height]
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || *value <= 0. || *value > f32::MAX as f64)
        {
            return Err(ui::PropError(
                "PopupMenu dimensions must be positive finite logical pixels".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(
        &mut self,
        props: &PopupMenuProps,
        changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        if changed.iter().any(|name| name == "items")
            && self.items.as_slice() != props.items.as_slice()
        {
            self.items = props.items.to_vec();
            self.dirty = true;
        }
        if changed.iter().any(|name| name == "open") {
            if props.open && !self.session.open.replace(true) {
                self.opening = true;
            } else if !props.open {
                self.close(window, cx);
            }
        }
        self.min_width = props.min_width.map(|value| value as f32);
        self.max_width = props.max_width.map(|value| value as f32);
        self.max_height = props.max_height.map(|value| value as f32);
        self.scrollable = props.scrollable;
        self.check_right = props.check_side == Some(MenuCheckSide::Right);
        self.external_link_icon = props.external_link_icon;
        self.dirty = true;
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| self.session.open.get())
    }
    fn command(
        &mut self,
        _: &PopupMenuProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        match name {
            "focus" => {
                if let Some(focus) = self.focus_handle() {
                    focus.focus(window, cx);
                }
            }
            "open" => {
                if !self.session.open.replace(true) {
                    self.opening = true;
                }
                self.session.context.redraw();
            }
            "close" => self.close(window, cx),
            _ => {}
        }
    }
}
impl Drop for NativePopup {
    fn drop(&mut self) {
        self.session.alive.set(false);
        self.subscription.take();
        if let (Some(window), Some(focus), Some(previous)) =
            (&self.window, self.focus.clone(), self.previous.clone())
        {
            window
                .spawn(async move |cx| {
                    let _ = cx.update(|window, cx| {
                        if focus.contains_focused(window, cx) {
                            previous.focus(window, cx);
                        }
                    });
                })
                .detach();
        }
    }
}

struct Lease {
    owner: usize,
    menus: Vec<OwnedMenu>,
    session: Weak<Session>,
}
#[derive(Clone)]
struct MenuSnapshot {
    styled: Vec<OwnedMenu>,
    native: Option<Vec<OwnedMenu>>,
}
#[derive(Default)]
struct MenuOwners {
    entries: Vec<Lease>,
    baseline: Option<MenuSnapshot>,
    applied: Option<Vec<OwnedMenu>>,
    revision: u64,
}
impl Global for MenuOwners {}
fn same_menus(a: &[OwnedMenu], b: &[OwnedMenu]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.name == b.name
                && a.disabled == b.disabled
                && a.items.len() == b.items.len()
                && a.items.iter().zip(&b.items).all(|(a, b)| match (a, b) {
                    (OwnedMenuItem::Separator, OwnedMenuItem::Separator) => true,
                    (OwnedMenuItem::Submenu(a), OwnedMenuItem::Submenu(b)) => {
                        same_menus(std::slice::from_ref(a), std::slice::from_ref(b))
                    }
                    (OwnedMenuItem::SystemMenu(a), OwnedMenuItem::SystemMenu(b)) => {
                        a.name == b.name && a.menu_type == b.menu_type
                    }
                    (
                        OwnedMenuItem::Action {
                            name: left_name,
                            action: left_action,
                            checked: left_checked,
                            disabled: left_disabled,
                            os_action: left_os_action,
                        },
                        OwnedMenuItem::Action {
                            name: right_name,
                            action: right_action,
                            checked: right_checked,
                            disabled: right_disabled,
                            os_action: right_os_action,
                        },
                    ) => {
                        left_name == right_name
                            && left_action.partial_eq(right_action.as_ref())
                            && left_checked == right_checked
                            && left_disabled == right_disabled
                            && left_os_action == right_os_action
                    }
                    _ => false,
                })
        })
}
fn platform_menu(menu: OwnedMenu) -> Menu {
    Menu {
        name: menu.name,
        disabled: menu.disabled,
        items: menu
            .items
            .into_iter()
            .map(|item| match item {
                OwnedMenuItem::Separator => MenuItem::Separator,
                OwnedMenuItem::Submenu(menu) => MenuItem::Submenu(platform_menu(menu)),
                OwnedMenuItem::SystemMenu(menu) => MenuItem::os_submenu(menu.name, menu.menu_type),
                OwnedMenuItem::Action {
                    name,
                    action,
                    checked,
                    disabled,
                    os_action,
                } => MenuItem::Action {
                    name: name.into(),
                    action,
                    checked,
                    disabled,
                    os_action,
                },
            })
            .collect(),
    }
}
fn action_enabled(menus: &[OwnedMenu], target: &MenuSelection) -> bool {
    menus.iter().any(|menu| {
        !menu.disabled
            && menu.items.iter().any(|item| match item {
                OwnedMenuItem::Submenu(menu) => action_enabled(std::slice::from_ref(menu), target),
                OwnedMenuItem::Action {
                    action, disabled, ..
                } => !disabled && action.partial_eq(target),
                _ => false,
            })
    })
}
fn ensure_owners(cx: &mut App) {
    if cx.has_global::<MenuOwners>() {
        return;
    }
    cx.set_global(MenuOwners::default());
    cx.on_action(|action: &MenuSelection, cx| {
        let owners = cx.global::<MenuOwners>();
        if !owners
            .applied
            .as_ref()
            .is_some_and(|menus| same_menus(menus, GlobalState::global(cx).app_menus()))
        {
            return;
        }
        if owners.applied.as_ref().is_some_and(|menus| {
            cx.get_menus()
                .as_ref()
                .is_some_and(|native| !same_menus(menus, native))
        }) {
            return;
        }
        if let Some(entry) = owners
            .entries
            .last()
            .filter(|entry| entry.owner == action.owner && action_enabled(&entry.menus, action))
        {
            if let Some(session) = entry
                .session
                .upgrade()
                .filter(|session| session.alive.get())
            {
                emit_event(&session.context, AppMenuBarEvent::Select(action.id.clone()));
            }
        }
    });
}
fn update_lease(owner: usize, menus: Option<Vec<OwnedMenu>>, session: Weak<Session>, cx: &mut App) {
    ensure_owners(cx);
    let current = MenuSnapshot {
        styled: GlobalState::global(cx).app_menus().to_vec(),
        native: cx.get_menus(),
    };
    let result = cx.update_global::<MenuOwners, _>(|owners, _| {
        let mut baseline = current.clone();
        let styled_takeover = owners
            .applied
            .as_ref()
            .is_some_and(|applied| !same_menus(applied, &current.styled));
        let native_takeover = owners.applied.as_ref().is_some_and(|applied| {
            current
                .native
                .as_ref()
                .is_some_and(|native| !same_menus(applied, native))
        });
        if styled_takeover || native_takeover {
            // The host may replace native menus without touching Component's
            // styled cache. Preserve that takeover in both presentations.
            if native_takeover && !styled_takeover {
                baseline.styled = current.native.clone().unwrap();
            } else if styled_takeover && !native_takeover {
                baseline.native = Some(current.styled.clone());
            }
            owners.entries.clear();
            owners.applied = None;
            owners.baseline = None;
        }
        if owners.baseline.is_none() && menus.is_some() {
            owners.baseline = Some(baseline.clone());
        }
        match menus {
            Some(menus) => {
                if let Some(entry) = owners.entries.iter_mut().find(|entry| entry.owner == owner) {
                    entry.menus = menus;
                    entry.session = session;
                } else {
                    owners.entries.push(Lease {
                        owner,
                        menus,
                        session,
                    });
                }
            }
            None => owners.entries.retain(|entry| entry.owner != owner),
        }
        owners.entries.retain(|entry| {
            entry
                .session
                .upgrade()
                .is_some_and(|session| session.alive.get())
        });
        let next = if let Some(entry) = owners.entries.last() {
            owners.applied = Some(entry.menus.clone());
            MenuSnapshot {
                styled: entry.menus.clone(),
                native: Some(entry.menus.clone()),
            }
        } else {
            owners.applied = None;
            owners.baseline.take().unwrap_or(baseline)
        };
        if !same_menus(&next.styled, &current.styled) {
            owners.revision = owners.revision.wrapping_add(1);
        }
        next
    });
    if !same_menus(&result.styled, &current.styled) {
        GlobalState::global_mut(cx).set_app_menus(result.styled.clone());
        cx.refresh_windows();
    }
    let native = result.native.unwrap_or(result.styled);
    if !current
        .native
        .as_ref()
        .is_some_and(|current| same_menus(current, &native))
    {
        cx.set_menus(native.into_iter().map(platform_menu));
    }
}

fn app_items(items: &[MenuItemValue], owner: usize) -> Vec<MenuItem> {
    items
        .iter()
        .map(|item| {
            if item.separator {
                MenuItem::Separator
            } else if let Some(items) = &item.items {
                MenuItem::submenu(Menu::new(item.label.clone()).items(app_items(items, owner)))
                    .disabled(item.disabled)
            } else {
                MenuItem::action(
                    item.label.clone(),
                    MenuSelection {
                        owner,
                        id: item.id.clone(),
                    },
                )
                .checked(item.checked)
                .disabled(item.disabled)
            }
        })
        .collect()
}
struct NativeMenuBar {
    owner: usize,
    session: Rc<Session>,
    menus: Vec<ApplicationMenu>,
    menus_set: bool,
    active: bool,
    state: Option<Entity<AppMenuBar>>,
    focuses: Vec<FocusHandle>,
    app: Option<AsyncApp>,
    dirty: bool,
    revision: u64,
}
impl NativeMenuBar {
    fn new(context: ComponentContext) -> Self {
        Self {
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            session: Rc::new(Session {
                alive: Cell::new(true),
                open: Cell::new(true),
                context,
            }),
            menus: vec![],
            menus_set: false,
            active: true,
            state: None,
            focuses: vec![],
            app: None,
            dirty: false,
            revision: u64::MAX,
        }
    }
}
impl TypedNativeComponent for NativeMenuBar {
    type Props = AppMenuBarProps;
    type Event = AppMenuBarEvent;
    fn render(
        &mut self,
        _: &AppMenuBarProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.app = Some(cx.to_async());
        ensure_owners(cx);
        if self.dirty {
            self.dirty = false;
            let menus = (self.active && self.menus_set).then(|| {
                self.menus
                    .iter()
                    .map(|menu| {
                        Menu::new(menu.name.clone())
                            .disabled(menu.disabled)
                            .items(app_items(&menu.items, self.owner))
                            .owned()
                    })
                    .collect()
            });
            let owner = self.owner;
            let weak = Rc::downgrade(&self.session);
            window.defer(cx, move |_, cx| {
                if weak.upgrade().is_some_and(|session| session.alive.get()) {
                    update_lease(owner, menus, weak, cx);
                }
            });
        }
        let revision = cx.global::<MenuOwners>().revision;
        let state = self
            .state
            .get_or_insert_with(|| AppMenuBar::new(cx))
            .clone();
        if self.revision != revision {
            self.revision = revision;
            state.update(cx, |state, cx| state.reload(cx));
        }
        self.focuses = state.read(cx).focus_handles(cx);
        Some(state.into_any_element())
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focuses.clone()
    }
    fn validate_props(&self, props: &AppMenuBarProps) -> Result<(), ui::PropError> {
        let mut ids = HashSet::new();
        for menu in props.menus.iter() {
            if menu.name.is_empty() {
                return Err(ui::PropError(
                    "Application menus require a nonempty name".into(),
                ));
            }
            validate(&menu.items, &mut ids, 0).map_err(ui::PropError)?;
        }
        Ok(())
    }
    fn props_changed(
        &mut self,
        props: &AppMenuBarProps,
        changed: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        if changed.iter().any(|name| name == "menus") {
            if !self.menus_set || self.menus.as_slice() != props.menus.as_slice() {
                self.menus = props.menus.to_vec();
                self.menus_set = true;
                self.dirty = true;
            }
        }
        if self.active != props.active {
            self.active = props.active;
            self.dirty = true;
        }
    }
}
impl Drop for NativeMenuBar {
    fn drop(&mut self) {
        self.session.alive.set(false);
        if let Some(app) = &self.app {
            let owner = self.owner;
            app.spawn(async move |cx| cx.update(|cx| update_lease(owner, None, Weak::new(), cx)))
                .detach();
        }
    }
}

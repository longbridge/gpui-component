//! Dock and splitter adapters. Layout data never contains rendered JSX.
use gpui_kit::component::{
    Disableable as _,
    button::Button,
    dock::{
        self, BasePanel, DockArea, DockAreaState, DockEvent as NativeDockEvent,
        DockLayout as NativeDockLayout, DockPlacement, DockSkin, Panel, PanelControl, PanelEvent,
        PanelInfo, PanelState, PanelStyle, TabGroup,
    },
    menu::{PopupMenu, PopupMenuItem},
    resizable::{self, ResizableState},
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EntityId, EventEmitter, FocusHandle,
    Focusable, Global, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, WeakEntity, Window, div, px,
};
use nsh_core::ui::PropError;
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[path = "dock_handle.rs"]
mod handle;

#[path = "dock_types.rs"]
mod types;
use types::*;

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    handle::register_typed(registry);
    registry.register::<NativeDock>("gpui-component.DockArea", "DockArea", NativeDock::new);
    registry.register::<NativeResizable>(
        "gpui-component.Resizable",
        "Resizable",
        NativeResizable::new,
    );
}

struct ScriptPanel {
    options: DockPanel,
    context: TypedComponentContext<DockEvent>,
    alive: Rc<Cell<bool>>,
    focus: FocusHandle,
    group: Option<WeakEntity<TabGroup>>,
}
impl Focusable for ScriptPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl EventEmitter<PanelEvent> for ScriptPanel {}
impl Render for ScriptPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus)
            .size_full()
            .children(crate::bridge::slot(&self.context, &self.options.id))
    }
}
impl BasePanel for ScriptPanel {
    fn panel_name(&self) -> &'static str {
        "NSHPanel"
    }
    fn visible(&self, _: &App) -> bool {
        self.options.visible
    }
    fn closable(&self, _: &App) -> bool {
        self.options.closable
    }
    fn zoomable(&self, _: &App) -> bool {
        self.options.zoomable
    }
    fn set_active(&mut self, active: bool, _: &mut Window, _: &mut Context<Self>) {
        if self.alive.get() {
            self.context.emit(DockEvent::ActiveChange {
                id: self.options.id.clone(),
                active,
            });
        }
    }
    fn set_zoomed(&mut self, zoomed: bool, _: &mut Window, _: &mut Context<Self>) {
        if self.alive.get() {
            self.context.emit(DockEvent::ZoomChange {
                id: self.options.id.clone(),
                zoomed,
            });
        }
    }
    fn on_added_to(&mut self, group: WeakEntity<TabGroup>, _: &mut Window, _: &mut Context<Self>) {
        self.group = Some(group);
    }
    fn on_removed(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.group = None;
        if self.alive.get() {
            self.context.emit(DockEvent::PanelRemoved {
                id: self.options.id.clone(),
            });
        }
    }
    fn dump(&self, _: &App) -> PanelState {
        let mut state = PanelState::new(self.panel_name());
        state.info = PanelInfo::panel(json!({"id": self.options.id, "data": self.options.data}));
        state
    }
}
impl Panel for ScriptPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::bridge::slot(&self.context, &format!("{}:title", self.options.id))
            .unwrap_or_else(|| SharedString::from(self.options.title.clone()).into_any_element())
    }
    fn title_suffix(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<impl IntoElement> {
        crate::bridge::slot(&self.context, &format!("{}:titleSuffix", self.options.id))
    }
    fn toolbar_buttons(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<Vec<Button>> {
        Some(
            self.options
                .toolbar
                .iter()
                .map(|action| {
                    let context = self.context.clone();
                    let alive = self.alive.clone();
                    let id = self.options.id.clone();
                    let action_id = action.id.clone();
                    let content =
                        crate::bridge::slot(&context, &format!("{id}:toolbar:{action_id}"));
                    Button::new(SharedString::from(action.id.clone()))
                        .disabled(action.disabled)
                        .map(|button| match content {
                            Some(content) => button.child(content),
                            None => button.label(action.label.clone()),
                        })
                        .on_click(move |_, _, _| {
                            if alive.get() {
                                context.emit(DockEvent::PanelAction {
                                    id: id.clone(),
                                    action: action_id.clone(),
                                });
                            }
                        })
                })
                .collect(),
        )
    }
    fn dropdown_menu(
        &mut self,
        mut menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> PopupMenu {
        for action in self.options.menu.iter() {
            let context = self.context.clone();
            let alive = self.alive.clone();
            let id = self.options.id.clone();
            let action_id = action.id.clone();
            menu = menu.item(
                PopupMenuItem::new(action.label.clone())
                    .disabled(action.disabled)
                    .on_click(move |_, _, _| {
                        if alive.get() {
                            context.emit(DockEvent::PanelAction {
                                id: id.clone(),
                                action: action_id.clone(),
                            });
                        }
                    }),
            );
        }
        menu
    }
    fn tab_name(&self, _: &App) -> Option<SharedString> {
        self.options.tab_name.clone().map(SharedString::from)
    }
    fn zoom_control(&self, _: &App) -> Option<PanelControl> {
        match self.options.zoom_control {
            ZoomControl::None => None,
            ZoomControl::Menu => Some(PanelControl::Menu),
            ZoomControl::Toolbar => Some(PanelControl::Toolbar),
            ZoomControl::Both => Some(PanelControl::Both),
        }
    }
    fn title_bar(&self, _: &App) -> bool {
        self.options.title_bar
    }
    fn inner_padding(&self, _: &App) -> bool {
        self.options.inner_padding
    }
}

type PanelMap = HashMap<String, Entity<ScriptPanel>>;
type RegistryMap = Rc<RefCell<HashMap<EntityId, HashMap<String, WeakEntity<ScriptPanel>>>>>;
struct ScriptPanelRegistry(RegistryMap);
impl Global for ScriptPanelRegistry {}

struct NativeDock {
    context: TypedComponentContext<DockEvent>,
    alive: Rc<Cell<bool>>,
    area: Option<Entity<DockArea>>,
    focus: Option<FocusHandle>,
    skin: Option<Rc<DockSkin>>,
    panels: PanelMap,
    options: Vec<DockPanel>,
    options_dirty: bool,
    layout: Option<Value>,
    restore: Option<Value>,
    locked: bool,
    tab_bar: bool,
    close_buttons: bool,
    dock_toggles: bool,
    presentation_dirty: bool,
    initialized: bool,
    subscription: Option<Subscription>,
    registry: Option<RegistryMap>,
}
impl NativeDock {
    fn new(context: TypedComponentContext<DockEvent>) -> Self {
        Self {
            context,
            alive: Rc::new(Cell::new(true)),
            area: None,
            focus: None,
            skin: None,
            panels: HashMap::new(),
            options: vec![],
            options_dirty: true,
            layout: None,
            restore: None,
            locked: false,
            tab_bar: false,
            close_buttons: false,
            dock_toggles: true,
            presentation_dirty: true,
            initialized: false,
            subscription: None,
            registry: None,
        }
    }
    fn error(&self, message: impl ToString) {
        self.context.emit(DockEvent::Error(message.to_string()));
    }
    fn ensure(&mut self, window: &mut Window, cx: &mut App) {
        if self.area.is_some() {
            return;
        }
        if cx.try_global::<ScriptPanelRegistry>().is_none() {
            let registry: RegistryMap = Rc::default();
            let weak_registry = Rc::downgrade(&registry);
            dock::register_panel(cx, "NSHPanel", move |build, _, cx| {
                let area = build
                    .dock_area()
                    .upgrade()
                    .expect("dock registry builder has a live area");
                let PanelInfo::Panel(value) = build.info() else {
                    unreachable!("NSHPanel uses panel state")
                };
                let id = value.get("id").and_then(Value::as_str).unwrap_or_default();
                let panel = weak_registry.upgrade().and_then(|registry| {
                    registry
                        .borrow()
                        .get(&area.entity_id())
                        .and_then(|panels| panels.get(id))
                        .and_then(|panel| panel.upgrade())
                });
                // All persisted IDs are validated before DockArea::load.
                let panel = panel.expect("validated NSH panel remains mounted");
                panel.update(cx, |panel, cx| {
                    panel.options.data = value.get("data").cloned().unwrap_or(Value::Null);
                    if panel.alive.get() {
                        panel.context.emit(DockEvent::PanelRestore {
                            id: panel.options.id.clone(),
                            data: panel.options.data.clone(),
                        });
                    }
                    cx.notify();
                });
                dock::panel_handle(panel)
            });
            cx.set_global(ScriptPanelRegistry(registry));
        }
        self.registry = Some(cx.global::<ScriptPanelRegistry>().0.clone());
        let (area, skin) = DockSkin::dock_area("nsh-dock", None, window, cx);
        let context = self.context.clone();
        let alive = self.alive.clone();
        self.subscription = Some(window.subscribe(&area, cx, move |area, event, _, cx| {
            if alive.get() && matches!(event, NativeDockEvent::LayoutChanged) {
                match serde_json::to_value(area.read(cx).dump(cx)) {
                    Ok(state) => context.emit(DockEvent::LayoutChange(DockState(state))),
                    Err(error) => context.emit(DockEvent::Error(error.to_string())),
                }
            }
        }));
        self.focus = Some(area.focus_handle(cx));
        self.area = Some(area);
        self.skin = Some(skin);
    }
    fn build_layout(
        &self,
        value: &Value,
        used: &mut HashSet<String>,
        cx: &App,
    ) -> Result<NativeDockLayout, String> {
        if let Some(ids) = value.get("panels").and_then(Value::as_array) {
            let mut layout = NativeDockLayout::tabs();
            for id in ids {
                let id = id.as_str().ok_or("panel IDs must be strings")?;
                let panel = self
                    .panels
                    .get(id)
                    .ok_or_else(|| format!("unknown panel: {id}"))?;
                if !used.insert(id.to_owned()) {
                    return Err(format!("panel occurs more than once: {id}"));
                }
                layout = layout.panel_view(dock::panel_handle(panel.clone()), cx);
            }
            return Ok(layout.active_index(
                value
                    .get("activeIndex")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
            ));
        }
        let mut layout = match value.get("axis").and_then(Value::as_str) {
            Some("horizontal") => NativeDockLayout::h_split(),
            Some("vertical") => NativeDockLayout::v_split(),
            _ => return Err("split requires axis horizontal or vertical".into()),
        };
        for child in value
            .get("children")
            .and_then(Value::as_array)
            .ok_or("split requires children")?
        {
            let size = child
                .get("size")
                .filter(|size| !size.is_null())
                .map(|value| finite_size(value, "split size"))
                .transpose()?;
            layout = layout.child(self.build_layout(child, used, cx)?, size.map(px));
        }
        Ok(layout)
    }
    fn validate_state(&self, state: &PanelState, used: &mut HashSet<String>) -> Result<(), String> {
        if state.panel_name == "NSHPanel" {
            let PanelInfo::Panel(info) = &state.info else {
                return Err("invalid NSHPanel info".into());
            };
            let id = info
                .get("id")
                .and_then(Value::as_str)
                .ok_or("missing restored panel ID")?;
            if !self.panels.contains_key(id) {
                return Err(format!("unknown restored panel: {id}"));
            }
            if !used.insert(id.to_owned()) {
                return Err(format!("duplicate restored panel: {id}"));
            }
        } else if state.panel_name != "StackPanel" && state.panel_name != "TabPanel" {
            return Err(format!(
                "unregistered restored panel type: {}",
                state.panel_name
            ));
        }
        for child in &state.children {
            self.validate_state(child, used)?;
        }
        Ok(())
    }
    fn load(&self, value: &Value, window: &mut Window, cx: &mut App) -> Result<(), String> {
        let state: DockAreaState =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        let mut used = HashSet::new();
        self.validate_state(&state.center, &mut used)?;
        for dock in [&state.left_dock, &state.right_dock, &state.bottom_dock]
            .into_iter()
            .flatten()
        {
            self.validate_state(dock.panel(), &mut used)?;
        }
        let area = self.area.as_ref().ok_or("dock not initialized")?;
        if area.read(cx).dump(cx) == state {
            return Ok(());
        }
        area.update(cx, |area, cx| area.load(state, window, cx))
            .map_err(|error| error.to_string())
    }
}
fn finite_size(value: &Value, name: &str) -> Result<f32, String> {
    value
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0. && *value <= f32::MAX as f64)
        .map(|value| value as f32)
        .ok_or_else(|| format!("{name} must be a finite nonnegative number"))
}
impl TypedNativeComponent for NativeDock {
    type Props = DockAreaProps;
    type Event = DockEvent;
    fn render(
        &mut self,
        _: &DockAreaProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.ensure(window, cx);
        let area = self.area.clone()?;
        if self.options_dirty {
            self.options_dirty = false;
            let ids: HashSet<_> = self
                .options
                .iter()
                .map(|option| option.id.clone())
                .collect();
            self.panels.retain(|id, panel| {
                if ids.contains(id) {
                    true
                } else {
                    area.update(cx, |area, cx| area.remove_panel(panel.clone(), window, cx));
                    false
                }
            });
            let mut added = vec![];
            for options in &self.options {
                if let Some(panel) = self.panels.get(&options.id) {
                    panel.update(cx, |panel, cx| {
                        panel.options = options.clone();
                        if let Some(group) = panel.group.as_ref().and_then(WeakEntity::upgrade) {
                            group.update(cx, |_, cx| cx.notify());
                        }
                        cx.notify();
                    });
                } else {
                    let panel = cx.new(|cx| ScriptPanel {
                        options: options.clone(),
                        context: self.context.clone(),
                        alive: self.alive.clone(),
                        focus: cx.focus_handle(),
                        group: None,
                    });
                    added.push(panel.clone());
                    self.panels.insert(options.id.clone(), panel);
                }
            }
            self.registry.as_ref()?.borrow_mut().insert(
                area.entity_id(),
                self.panels
                    .iter()
                    .map(|(id, panel)| (id.clone(), panel.downgrade()))
                    .collect(),
            );
            // Initial tabs are only installed once. A metadata echo never resets a drag.
            if !self.initialized && self.layout.is_none() && self.restore.is_none() {
                let layout =
                    self.options
                        .iter()
                        .fold(NativeDockLayout::tabs(), |layout, option| {
                            layout
                                .panel_view(dock::panel_handle(self.panels[&option.id].clone()), cx)
                        });
                area.update(cx, |area, cx| area.set_center(layout, window, cx));
            } else if self.initialized && self.layout.is_none() && self.restore.is_none() {
                for panel in added {
                    area.update(cx, |area, cx| {
                        area.add_panel_view(
                            dock::panel_handle(panel),
                            DockPlacement::Center,
                            None,
                            window,
                            cx,
                        )
                    });
                }
            }
            self.initialized = true;
            area.update(cx, |_, cx| cx.notify());
        }
        if let Some(value) = self.layout.take() {
            let mut used = HashSet::new();
            let built = (|| {
                let center = self.build_layout(
                    value.get("center").ok_or("layout requires center")?,
                    &mut used,
                    cx,
                )?;
                let mut docks = vec![];
                for (name, placement) in [
                    ("left", DockPlacement::Left),
                    ("right", DockPlacement::Right),
                    ("bottom", DockPlacement::Bottom),
                ] {
                    if let Some(value) = value.get(name).filter(|value| !value.is_null()) {
                        docks.push((placement, self.build_layout(value, &mut used, cx)?));
                    }
                }
                Ok::<_, String>((center, docks))
            })();
            match built {
                Ok((center, docks)) => area.update(cx, |area, cx| {
                    area.set_center(center, window, cx);
                    for placement in [
                        DockPlacement::Left,
                        DockPlacement::Right,
                        DockPlacement::Bottom,
                    ] {
                        area.remove_dock(placement, window, cx);
                    }
                    for (placement, layout) in docks {
                        area.set_dock(placement, layout, window, cx);
                    }
                }),
                Err(error) => self.error(error),
            }
        }
        if let Some(value) = self.restore.take() {
            if let Err(error) = self.load(&value, window, cx) {
                self.error(error);
            }
        }
        area.update(cx, |area, cx| area.set_locked(self.locked, window, cx));
        if self.presentation_dirty {
            self.presentation_dirty = false;
            let skin = self.skin.as_ref()?;
            skin.set_panel_style(
                if self.tab_bar {
                    PanelStyle::TabBar
                } else {
                    PanelStyle::Auto
                },
                cx,
            );
            skin.set_close_button_visible(self.close_buttons, cx);
            skin.set_toggle_button_visible(self.dock_toggles, cx);
        }
        Some(area.into_any_element())
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn validate_props(&self, props: &DockAreaProps) -> Result<(), PropError> {
        let options: &Vec<DockPanel> = &props.panels;
        let valid = options.iter().all(|option| {
            !option.id.is_empty()
                && [&option.toolbar, &option.menu].into_iter().all(|actions| {
                    actions.iter().all(|action| !action.id.is_empty())
                        && actions
                            .iter()
                            .map(|action| &action.id)
                            .collect::<HashSet<_>>()
                            .len()
                            == actions.len()
                })
        }) && options
            .iter()
            .map(|option| &option.id)
            .collect::<HashSet<_>>()
            .len()
            == options.len();
        if valid {
            Ok(())
        } else {
            Err(PropError(
                "panels and their actions require unique nonempty IDs".into(),
            ))
        }
    }
    fn props_changed(
        &mut self,
        props: &DockAreaProps,
        changed: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        if changed.iter().any(|name| name == "panels")
            && self.options.as_slice() != props.panels.as_slice()
        {
            self.options = props.panels.to_vec();
            self.options_dirty = true;
        }
        if changed.iter().any(|name| name == "layout") {
            self.layout = props
                .layout
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .unwrap_or_else(|error| {
                    self.error(error);
                    None
                });
        }
        if changed.iter().any(|name| name == "state") {
            self.restore = props
                .state
                .as_ref()
                .map(|state| state.0.clone())
                .filter(|value| !value.is_null());
        }
        self.locked = props.locked;
        if (self.tab_bar, self.close_buttons, self.dock_toggles)
            != (props.tab_bar, props.close_buttons, props.dock_toggles)
        {
            self.tab_bar = props.tab_bar;
            self.close_buttons = props.close_buttons;
            self.dock_toggles = props.dock_toggles;
            self.presentation_dirty = true;
        }
    }
    fn command(
        &mut self,
        _: &Self::Props,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.ensure(window, cx);
        let Some(area) = self.area.clone() else {
            return;
        };
        match name {
            "save" => match serde_json::to_value(area.read(cx).dump(cx)) {
                Ok(value) => self.context.emit(DockEvent::Save(DockState(value))),
                Err(error) => self.error(error),
            },
            "load" => {
                if let Err(error) = self.load(args, window, cx) {
                    self.error(error);
                }
            }
            "select" | "close" | "zoom" => {
                let Some(panel) = args.as_str().and_then(|id| self.panels.get(id)).cloned() else {
                    self.error("command requires a known panel ID");
                    return;
                };
                area.update(cx, |area, cx| match name {
                    "select" => area.select_panel(panel.entity_id().into(), window, cx),
                    "close" => {
                        if panel.read(cx).options.closable {
                            area.remove_panel(panel, window, cx);
                        }
                    }
                    _ => {
                        if panel.read(cx).options.zoomable
                            && panel.read(cx).options.zoom_control != ZoomControl::None
                        {
                            let panel_id = panel.entity_id().into();
                            area.select_panel(panel_id, window, cx);
                            let node = [
                                DockPlacement::Center,
                                DockPlacement::Left,
                                DockPlacement::Right,
                                DockPlacement::Bottom,
                            ]
                            .into_iter()
                            .find_map(|placement| {
                                area.layout(placement)
                                    .and_then(|tree| tree.find_panel_node(panel_id))
                            });
                            if let Some(node) = node {
                                area.set_zoomed_in(node, window, cx);
                            }
                        }
                    }
                });
            }
            "move" => {
                let parsed = (|| {
                    let source = args
                        .get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| self.panels.get(id))
                        .ok_or("move requires known id")?;
                    let target = args
                        .get("target")
                        .and_then(Value::as_str)
                        .and_then(|id| self.panels.get(id))
                        .ok_or("move requires known target panel")?;
                    let target_id = target.entity_id().into();
                    let node = [
                        DockPlacement::Center,
                        DockPlacement::Left,
                        DockPlacement::Right,
                        DockPlacement::Bottom,
                    ]
                    .into_iter()
                    .find_map(|placement| {
                        area.read(cx)
                            .layout(placement)
                            .and_then(|tree| tree.find_panel_node(target_id))
                    })
                    .ok_or("target panel is not installed")?;
                    let target = if let Some(placement) =
                        args.get("placement").and_then(Value::as_str)
                    {
                        let placement = match placement {
                            "left" => gpui_kit::component::Placement::Left,
                            "right" => gpui_kit::component::Placement::Right,
                            "top" => gpui_kit::component::Placement::Top,
                            "bottom" => gpui_kit::component::Placement::Bottom,
                            _ => {
                                return Err(
                                    "split placement must be left, right, top or bottom".to_owned()
                                );
                            }
                        };
                        let size = args
                            .get("size")
                            .map(|value| finite_size(value, "split size"))
                            .transpose()?
                            .map(px);
                        dock::InsertTarget::Split {
                            node,
                            placement,
                            size,
                        }
                    } else {
                        dock::InsertTarget::Tabs {
                            node,
                            ix: args
                                .get("index")
                                .and_then(Value::as_u64)
                                .map(|index| index as usize),
                            activate: true,
                        }
                    };
                    Ok::<_, String>((source.entity_id().into(), target))
                })();
                match parsed {
                    Ok((source, target)) => {
                        area.update(cx, |area, cx| area.move_panel(source, target, window, cx))
                    }
                    Err(error) => self.error(error),
                }
            }
            "dockSize" => {
                let placement = match args.get("placement").and_then(Value::as_str) {
                    Some("left") => DockPlacement::Left,
                    Some("right") => DockPlacement::Right,
                    Some("bottom") => DockPlacement::Bottom,
                    _ => {
                        self.error("dockSize requires left, right or bottom placement");
                        return;
                    }
                };
                let size = match args
                    .get("size")
                    .ok_or("dockSize requires size".to_owned())
                    .and_then(|value| finite_size(value, "dock size"))
                {
                    Ok(size) => size,
                    Err(error) => {
                        self.error(error);
                        return;
                    }
                };
                area.update(cx, |area, cx| {
                    area.set_dock_size(placement, px(size), window, cx)
                });
            }
            "unzoom" => area.update(cx, |area, cx| area.set_zoomed_out(window, cx)),
            "toggleDock" => {
                let placement = match args.as_str() {
                    Some("left") => DockPlacement::Left,
                    Some("right") => DockPlacement::Right,
                    Some("bottom") => DockPlacement::Bottom,
                    _ => {
                        self.error("dock placement must be left, right or bottom");
                        return;
                    }
                };
                area.update(cx, |area, cx| area.toggle_dock(placement, window, cx));
            }
            _ => self.error(format!("unknown DockArea command: {name}")),
        }
    }
}
impl Drop for NativeDock {
    fn drop(&mut self) {
        self.alive.set(false);
        if let (Some(area), Some(registry)) = (&self.area, &self.registry) {
            registry.borrow_mut().remove(&area.entity_id());
        }
    }
}

struct NativeResizable {
    context: TypedComponentContext<ResizableEvent>,
    state: Option<Entity<ResizableState>>,
    panels: Vec<ResizablePanel>,
    vertical: bool,
    subscription: Option<Subscription>,
    remembered_sizes: HashMap<String, f32>,
}
impl NativeResizable {
    fn new(context: TypedComponentContext<ResizableEvent>) -> Self {
        Self {
            context,
            state: None,
            panels: vec![],
            vertical: false,
            subscription: None,
            remembered_sizes: HashMap::new(),
        }
    }
    fn error(&self, error: impl ToString) {
        self.context.emit(ResizableEvent::Error(error.to_string()));
    }
}
impl TypedNativeComponent for NativeResizable {
    type Props = ResizableProps;
    type Event = ResizableEvent;
    fn render(
        &mut self,
        _: &ResizableProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        if self.state.is_none() {
            let state = cx.new(|_| ResizableState::default());
            let context = self.context.clone();
            self.subscription = Some(window.subscribe(
                &state,
                cx,
                move |state, _: &resizable::ResizablePanelEvent, _, cx| {
                    context.emit(ResizableEvent::Resize(
                        state
                            .read(cx)
                            .sizes()
                            .iter()
                            .map(|size| f32::from(*size))
                            .collect::<Vec<_>>()
                            .into(),
                    ));
                },
            ));
            self.state = Some(state);
        }
        let state = self.state.as_ref()?;
        let group = if self.vertical {
            resizable::v_resizable("split")
        } else {
            resizable::h_resizable("split")
        };
        Some(
            group
                .with_state(state)
                .children(self.panels.iter().map(|options| {
                    resizable::resizable_panel()
                        .visible(options.visible)
                        .children(crate::bridge::slot(&self.context, &options.id))
                        .when_some(
                            self.remembered_sizes
                                .get(&options.id)
                                .copied()
                                .or(options.size),
                            |panel, size| panel.size(px(size)),
                        )
                        .size_range(
                            px(options.min_size.unwrap_or(100.))
                                ..px(options.max_size.unwrap_or(f32::MAX)),
                        )
                        .when(options.fixed, |panel| panel.fixed())
                }))
                .into_any_element(),
        )
    }
    fn validate_props(&self, props: &ResizableProps) -> Result<(), PropError> {
        let panels: &Vec<ResizablePanel> = &props.panels;
        let valid = panels.iter().all(|panel| {
            !panel.id.is_empty()
                && [panel.size, panel.min_size, panel.max_size]
                    .into_iter()
                    .flatten()
                    .all(|size| size.is_finite() && size >= 0.)
                && panel.min_size.unwrap_or(100.) <= panel.max_size.unwrap_or(f32::MAX)
        }) && panels
            .iter()
            .map(|panel| &panel.id)
            .collect::<HashSet<_>>()
            .len()
            == panels.len();
        if valid {
            Ok(())
        } else {
            Err(PropError(
                "split panels require unique IDs and finite ordered size constraints".into(),
            ))
        }
    }
    fn props_changed(
        &mut self,
        props: &ResizableProps,
        changed: &[String],
        _: &mut Window,
        cx: &mut App,
    ) {
        self.vertical = props.vertical;
        if !changed.iter().any(|name| name == "panels") {
            return;
        }
        let panels: &Vec<ResizablePanel> = &props.panels;
        if self
            .panels
            .iter()
            .map(|panel| &panel.id)
            .ne(panels.iter().map(|panel| &panel.id))
        {
            if let Some(state) = &self.state {
                self.remembered_sizes = self
                    .panels
                    .iter()
                    .zip(state.read(cx).sizes())
                    .map(|(panel, size)| (panel.id.clone(), f32::from(*size)))
                    .collect();
            }
            self.state = None;
            self.subscription = None;
        }
        self.panels = panels.clone();
    }
    fn command(
        &mut self,
        _: &Self::Props,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name != "resize" {
            self.error(format!("unknown Resizable command: {name}"));
            return;
        }
        let Some(index) = args.get("index").and_then(Value::as_u64) else {
            self.error("resize requires index");
            return;
        };
        let size = match args
            .get("size")
            .ok_or("resize requires size".to_string())
            .and_then(|value| finite_size(value, "size"))
        {
            Ok(size) => size,
            Err(error) => {
                self.error(error);
                return;
            }
        };
        if let Some(state) = &self.state {
            state.update(cx, |state, cx| {
                state.resize_panel(index as usize, px(size), window, cx)
            });
        }
    }
}

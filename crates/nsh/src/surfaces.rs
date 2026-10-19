//! Composed anchored surfaces. NSH retains ownership of every child subtree.
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use crate::menus::MenuItemValue;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{
    Action, Anchor, AnyElement, App, Bounds, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement,
    Window,
    component::{
        Selectable,
        button::Button,
        hover_card::HoverCard,
        menu::{ContextMenuExt, DropdownMenuPopover, PopupMenu, PopupMenuItem},
        native_menu::NativeMenu,
        popover::Popover,
        tooltip::{ManagedTooltipExt, Tooltip},
    },
    div, point,
    prelude::FluentBuilder,
    px,
};
use nsh_core::ui::{Children, Event, PropEnum, Props, Slot};
use nsh_platform_gpui::{ComponentContext, TypedComponentContext, TypedNativeComponent};
use serde_json::{Value, json};

static NEXT_OWNER: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum SurfaceAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    LeftCenter,
    RightCenter,
}
impl From<SurfaceAnchor> for Anchor {
    fn from(value: SurfaceAnchor) -> Self {
        match value {
            SurfaceAnchor::TopLeft => Self::TopLeft,
            SurfaceAnchor::TopRight => Self::TopRight,
            SurfaceAnchor::BottomLeft => Self::BottomLeft,
            SurfaceAnchor::BottomRight => Self::BottomRight,
            SurfaceAnchor::TopCenter => Self::TopCenter,
            SurfaceAnchor::BottomCenter => Self::BottomCenter,
            SurfaceAnchor::LeftCenter => Self::LeftCenter,
            SurfaceAnchor::RightCenter => Self::RightCenter,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum SurfaceMouseButton {
    Left,
    Right,
}
#[derive(Event)]
pub(crate) enum SurfaceOpenEvent {
    OpenChange(bool),
    Error(String),
}
#[derive(Event)]
pub(crate) enum SurfaceErrorEvent {
    Error(String),
}
#[derive(Event)]
pub(crate) enum SurfaceMenuEvent {
    Select(String),
    Error(String),
}
#[derive(Event)]
pub(crate) enum SurfaceDropdownEvent {
    Select(String),
    OpenChange(bool),
    Error(String),
}
trait SurfaceEvents: Event {
    fn error(context: &ComponentContext, error: String);
    fn open(context: &ComponentContext, open: bool) {
        let _ = (context, open);
    }
    fn select(context: &ComponentContext, id: String) {
        let _ = (context, id);
    }
}
macro_rules! error_event {
    ($event:ident) => {
        fn error(context: &ComponentContext, error: String) {
            crate::menus::emit_event(context, $event::Error(error));
        }
    };
}
impl SurfaceEvents for SurfaceOpenEvent {
    error_event!(SurfaceOpenEvent);
    fn open(context: &ComponentContext, open: bool) {
        crate::menus::emit_event(context, Self::OpenChange(open));
    }
}
impl SurfaceEvents for SurfaceErrorEvent {
    error_event!(SurfaceErrorEvent);
}
impl SurfaceEvents for SurfaceMenuEvent {
    error_event!(SurfaceMenuEvent);
    fn select(context: &ComponentContext, id: String) {
        crate::menus::emit_event(context, Self::Select(id));
    }
}
impl SurfaceEvents for SurfaceDropdownEvent {
    error_event!(SurfaceDropdownEvent);
    fn open(context: &ComponentContext, open: bool) {
        crate::menus::emit_event(context, Self::OpenChange(open));
    }
    fn select(context: &ComponentContext, id: String) {
        crate::menus::emit_event(context, Self::Select(id));
    }
}
#[derive(Default)]
struct SurfaceValues {
    label: Option<String>,
    text: Option<String>,
    anchor: Option<SurfaceAnchor>,
    open: Option<bool>,
    default_open: Option<bool>,
    offset: Option<f64>,
    mouse_button: Option<SurfaceMouseButton>,
    arrow: Option<bool>,
    appearance: Option<bool>,
    overlay_closable: Option<bool>,
    open_delay: Option<u64>,
    close_delay: Option<u64>,
    items: Vec<MenuItemValue>,
}
trait SurfaceSchema: Props {
    type Event: SurfaceEvents;
    const NAME: &'static str;
    fn values(&self) -> SurfaceValues;
}
macro_rules! surface_schema {
    ($props:ident,$name:literal,$event:ty, { $( $(#[$attr:meta])* $field:ident : $ty:ty, )* })=>{
        #[derive(Clone,PartialEq,Props)]
        pub(crate) struct $props { $( $(#[$attr])* $field:$ty, )* children:Children, trigger:Option<Slot>, content:Option<Slot> }
        impl SurfaceSchema for $props {
            type Event=$event;
            const NAME:&'static str=$name;
            fn values(&self)->SurfaceValues { SurfaceValues { $($field: ::core::clone::Clone::clone(&self.$field),)* ..SurfaceValues::default() } }
        }
    }
}
surface_schema!(PopoverProps,"Popover",SurfaceOpenEvent,{
    label:Option<String>, open:Option<bool>, default_open:Option<bool>, anchor:Option<SurfaceAnchor>, offset:Option<f64>, mouse_button:Option<SurfaceMouseButton>, arrow:Option<bool>, appearance:Option<bool>, overlay_closable:Option<bool>,
});
surface_schema!(HoverCardProps,"HoverCard",SurfaceOpenEvent,{
    label:Option<String>, anchor:Option<SurfaceAnchor>, open_delay:Option<u64>, close_delay:Option<u64>, appearance:Option<bool>,
});
surface_schema!(TooltipProps,"Tooltip",SurfaceErrorEvent,{ text:Option<String>, });
surface_schema!(ContextMenuProps,"ContextMenu",SurfaceMenuEvent,{ items:Vec<MenuItemValue>, });
surface_schema!(NativeMenuProps,"NativeMenu",SurfaceMenuEvent,{ items:Vec<MenuItemValue>, label:Option<String>, });
surface_schema!(DropdownMenuProps,"DropdownMenu",SurfaceDropdownEvent,{ items:Vec<MenuItemValue>, label:Option<String>, anchor:Option<SurfaceAnchor>, });
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<Surface<PopoverProps>>("gpui-component.Popover", "Popover", Surface::new);
    registry
        .metadata()
        .native_controlled("gpui-component.Popover", &[("openChange", &["open"])]);
    registry.register::<Surface<HoverCardProps>>(
        "gpui-component.HoverCard",
        "HoverCard",
        Surface::new,
    );
    registry.register::<Surface<TooltipProps>>("gpui-component.Tooltip", "Tooltip", Surface::new);
    registry.register::<Surface<ContextMenuProps>>(
        "gpui-component.ContextMenu",
        "ContextMenu",
        Surface::new,
    );
    registry.register::<Surface<NativeMenuProps>>(
        "gpui-component.NativeMenu",
        "NativeMenu",
        Surface::new,
    );
    registry.register::<Surface<DropdownMenuProps>>(
        "gpui-component.DropdownMenu",
        "DropdownMenu",
        Surface::new,
    );
}
struct Surface<P: SurfaceSchema> {
    owner: usize,
    context: ComponentContext,
    props: SurfaceValues,
    label: SharedString,
    tooltip_text: SharedString,
    focus: Option<FocusHandle>,
    bounds: Option<Bounds<Pixels>>,
    schema: std::marker::PhantomData<P>,
}
#[derive(Action, Clone, PartialEq)]
#[action(no_json)]
struct NativeSelection {
    owner: usize,
    id: String,
}

fn native_menu(items: &[MenuItemValue], owner: usize) -> NativeMenu {
    items.iter().fold(NativeMenu::new(), |menu, item| {
        if item.separator {
            return menu.separator();
        }
        let label = item.label.clone();
        if let Some(items) = item.items.as_ref() {
            return menu.submenu_with_disabled(label, item.disabled, native_menu(items, owner));
        }
        let action = Box::new(NativeSelection {
            owner,
            id: item.id.clone(),
        });
        menu.menu_with_check_and_disabled(label, item.checked, action, item.disabled)
    })
}

impl<P: SurfaceSchema> Surface<P> {
    fn new(context: TypedComponentContext<P::Event>) -> Self {
        Self {
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            context: context.raw().clone(),
            props: SurfaceValues::default(),
            label: SharedString::default(),
            tooltip_text: SharedString::default(),
            focus: None,
            bounds: None,
            schema: std::marker::PhantomData,
        }
    }
    fn anchor(&self, default: Anchor) -> Anchor {
        self.props.anchor.map(Into::into).unwrap_or(default)
    }
    fn boolean(&self, name: &str, fallback: bool) -> bool {
        match name {
            "defaultOpen" => self.props.default_open,
            "arrow" => self.props.arrow,
            "appearance" => self.props.appearance,
            "overlayClosable" => self.props.overlay_closable,
            _ => None,
        }
        .unwrap_or(fallback)
    }
    fn delay(&self, name: &str, fallback: u64) -> Duration {
        Duration::from_millis(
            match name {
                "openDelay" => self.props.open_delay,
                "closeDelay" => self.props.close_delay,
                _ => None,
            }
            .unwrap_or(fallback),
        )
    }
}
pub(crate) fn typed_menu_items(
    mut menu: PopupMenu,
    items: &[MenuItemValue],
    context: &ComponentContext,
    window: &mut Window,
    cx: &mut gpui_kit::Context<PopupMenu>,
) -> PopupMenu {
    if let Err(error) = crate::menus::validate(items, &mut std::collections::HashSet::new(), 0) {
        context.emit("error", json!(error));
        return menu;
    }
    for item in items {
        if item.separator {
            menu = menu.separator();
        } else if let Some(children) = item.items.as_ref() {
            let children = children.clone();
            let context = context.clone();
            let label = item.label.clone();
            menu = menu.submenu_with_disabled(
                label,
                item.disabled,
                window,
                cx,
                move |menu, window, cx| typed_menu_items(menu, &children, &context, window, cx),
            );
        } else {
            let key = item.id.clone();
            let context = context.clone();
            let slot_name = format!("item:{}", item.id);
            let entry = if context.slot(&slot_name).is_some() {
                let context = context.clone();
                PopupMenuItem::element(move |_, _| {
                    crate::bridge::slot(&context, &slot_name)
                        .unwrap_or_else(|| div().into_any_element())
                })
            } else {
                PopupMenuItem::new(item.label.clone())
            };
            menu = menu.item(
                entry
                    .disabled(item.disabled)
                    .checked(item.checked)
                    .on_click(move |_, _, _| context.emit("select", json!(key))),
            );
        }
    }
    menu
}

impl<P: SurfaceSchema> TypedNativeComponent for Surface<P> {
    type Props = P;
    type Event = P::Event;
    fn render(&mut self, _: &P, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let context = self.context.clone();
        let change = self.context.clone();
        let label = self.label.clone();
        let trigger = crate::bridge::slot(&self.context, "trigger");
        if trigger.is_none() && matches!(P::NAME, "Popover" | "HoverCard" | "DropdownMenu") {
            self.focus.get_or_insert_with(|| cx.focus_handle());
        }
        let trigger_focus = self.focus.clone();
        let content_context = context.clone();
        let content = move || {
            div()
                .children(crate::bridge::children(&content_context))
                .children(crate::bridge::slot(&content_context, "content"))
        };
        Some(match P::NAME {
            "NativeMenu" => {
                let owner = self.owner;
                let focus = self.focus.get_or_insert_with(|| cx.focus_handle()).clone();
                let items = self.props.items.clone();
                let keys = items.clone();
                let anchor = self.bounds.map(|bounds| bounds.bottom_left());
                div()
                    .id("surface")
                    .test_support()
                    .role(Role::Group)
                    .when(!label.is_empty(), |element| element.aria_label(label))
                    .track_focus(&focus)
                    .children(trigger)
                    .children(crate::bridge::children(&self.context))
                    .on_action(move |action: &NativeSelection, _, cx| {
                        if action.owner == owner {
                            P::Event::select(&context, action.id.clone());
                        } else {
                            cx.propagate();
                        }
                    })
                    .on_key_down(move |event, window, cx| {
                        if (event.keystroke.key == "f10" && event.keystroke.modifiers.shift)
                            || event.keystroke.key == "contextmenu"
                        {
                            native_menu(&keys, owner).show(
                                anchor.unwrap_or_else(|| window.mouse_position()),
                                window,
                                cx,
                            );
                            cx.stop_propagation();
                        }
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        focus.focus(window, cx);
                        native_menu(&items, owner).show(event.position, window, cx);
                    })
                    .into_any_element()
            }
            "Popover" => Popover::new("surface")
                .anchor(self.anchor(Anchor::TopLeft))
                .default_open(self.boolean("defaultOpen", false))
                .mouse_button(
                    if self.props.mouse_button == Some(SurfaceMouseButton::Right) {
                        MouseButton::Right
                    } else {
                        MouseButton::Left
                    },
                )
                .when_some(self.props.offset, |popover, offset| {
                    popover.offset(px(offset as f32))
                })
                .trigger(SurfaceTrigger {
                    element: trigger,
                    label,
                    focus: trigger_focus,
                    open: false,
                })
                .arrow(self.boolean("arrow", false))
                .appearance(self.boolean("appearance", true))
                .overlay_closable(self.boolean("overlayClosable", true))
                .when_some(self.props.open, |popover, open| popover.open(open))
                .on_open_change(move |open, _, _| P::Event::open(&change, *open))
                .content(move |_, _, _| content())
                .into_any_element(),
            "HoverCard" => HoverCard::new("surface")
                .anchor(self.anchor(Anchor::TopCenter))
                .trigger(trigger.unwrap_or_else(|| {
                    Button::new("trigger")
                        .label(label)
                        .when_some(trigger_focus.clone(), |button, focus| {
                            button.track_focus(&focus)
                        })
                        .into_any_element()
                }))
                .open_delay(self.delay("openDelay", 600))
                .close_delay(self.delay("closeDelay", 300))
                .appearance(self.boolean("appearance", true))
                .on_open_change(move |open, _, _| P::Event::open(&change, *open))
                .content(move |_, _, _| content())
                .into_any_element(),
            "Tooltip" => {
                let text = self.tooltip_text.clone();
                div()
                    .id("surface")
                    .children(trigger)
                    .children(crate::bridge::children(&self.context))
                    .managed_tooltip(move |window, cx| {
                        Tooltip::element({
                            let context = context.clone();
                            let text = text.clone();
                            move |_, _| {
                                div()
                                    .child(text.clone())
                                    .children(crate::bridge::slot(&context, "content"))
                            }
                        })
                        .build(window, cx)
                    })
                    .into_any_element()
            }
            "DropdownMenu" => {
                let items = self.props.items.clone();
                DropdownMenuPopover::new(
                    "surface".into(),
                    self.anchor(Anchor::TopLeft),
                    SurfaceTrigger {
                        element: trigger,
                        label,
                        focus: trigger_focus,
                        open: false,
                    },
                    move |menu, window, cx| typed_menu_items(menu, &items, &context, window, cx),
                )
                .on_open_change(move |open, _, _| P::Event::open(&change, *open))
                .into_any_element()
            }
            "ContextMenu" => {
                let items = self.props.items.clone();
                div()
                    .id("surface")
                    .children(trigger)
                    .children(crate::bridge::children(&self.context))
                    .context_menu(move |menu, window, cx| {
                        typed_menu_items(menu, &items, &context, window, cx)
                    })
                    .into_any_element()
            }
            _ => unreachable!(),
        })
    }

    fn validate_props(&self, props: &P) -> Result<(), nsh_core::ui::PropError> {
        let next = props.values();
        crate::menus::validate(&next.items, &mut std::collections::HashSet::new(), 0)
            .map_err(nsh_core::ui::PropError)?;
        if next
            .offset
            .is_some_and(|value| !value.is_finite() || !(value as f32).is_finite())
        {
            return Err(nsh_core::ui::PropError(
                "offset must be a finite number".into(),
            ));
        }
        if [next.open_delay, next.close_delay]
            .iter()
            .flatten()
            .any(|value| {
                std::time::Instant::now()
                    .checked_add(Duration::from_millis(*value))
                    .is_none()
            })
        {
            return Err(nsh_core::ui::PropError(
                "delay must fit a nonnegative integer duration".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(&mut self, props: &P, _: &[String], _: &mut Window, _: &mut App) {
        let next = props.values();
        self.label = next.label.clone().unwrap_or_default().into();
        self.tooltip_text = next.text.clone().unwrap_or_default().into();
        self.props = next;
        self.context.redraw();
    }

    fn focus_handle(&self) -> Option<FocusHandle> {
        if matches!(P::NAME, "Popover" | "HoverCard" | "DropdownMenu")
            && self.context.slot("trigger").is_some()
        {
            None
        } else {
            self.focus.clone()
        }
    }

    fn placed(&mut self, bounds: Option<Bounds<Pixels>>) {
        self.bounds = bounds;
    }

    fn command(&mut self, _: &P, name: &str, args: &Value, window: &mut Window, cx: &mut App) {
        if name == "focus" {
            if let Some(focus) = self.focus_handle() {
                focus.focus(window, cx);
            }
        }
        if name == "open" && P::NAME == "NativeMenu" {
            let Some(bounds) = self.bounds else {
                P::Event::error(&self.context, "NativeMenu is not currently placed".into());
                return;
            };
            let position = if args.get("x").is_some() || args.get("y").is_some() {
                match (
                    args.get("x").and_then(Value::as_f64),
                    args.get("y").and_then(Value::as_f64),
                ) {
                    (Some(x), Some(y)) if (x as f32).is_finite() && (y as f32).is_finite() => {
                        point(px(x as f32), px(y as f32))
                    }
                    _ => {
                        P::Event::error(
                            &self.context,
                            "open position requires finite x and y".into(),
                        );
                        return;
                    }
                }
            } else {
                bounds.bottom_left()
            };
            if let Some(focus) = &self.focus {
                focus.focus(window, cx);
            }
            let items = self.props.items.clone();
            native_menu(&items, self.owner).show(position, window, cx);
        }
    }
}

#[derive(IntoElement)]
struct SurfaceTrigger {
    element: Option<AnyElement>,
    label: SharedString,
    focus: Option<FocusHandle>,
    open: bool,
}

impl Selectable for SurfaceTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }
    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for SurfaceTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.element.unwrap_or_else(|| {
            Button::new("trigger")
                .label(self.label)
                .open(self.open)
                .when_some(self.focus, |button, focus| button.track_focus(&focus))
                .into_any_element()
        })
    }
}

//! Typed display controls; the NSH owner remains authoritative for every value.
use super::*;
use crate::registration::{BindingRegistry, native_leaf};
use component::{ActiveTheme, Colorize, button::ToggleVariants};
use gpui_kit::{Axis, Hsla, Keystroke, Radians, Styled};
use nsh_core::ui::{JsHost, JsIn, JsOwned, JsValue, PropError, TsTypes};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum ColorName {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Error,
    Info,
    Foreground,
    Muted,
    Border,
}
#[derive(Clone, PartialEq)]
pub(crate) struct HexColor(String);
impl JsValue for HexColor {
    fn ts_type(_: &mut TsTypes) -> String {
        "`#${string}`".into()
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        let text = String::from_js(value)?;
        if !text.starts_with('#')
            || !text[1..].bytes().all(|b| b.is_ascii_hexdigit())
            || Hsla::parse_hex(&text).is_err()
        {
            return Err(PropError("expected a hexadecimal color".into()));
        }
        Ok(Self(text))
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        self.0.to_js(host)
    }
}
crate::registration::named_value! { pub(crate) struct ComponentColor(Either<ColorName,HexColor>); }
fn color(value: &ComponentColor, cx: &App) -> Hsla {
    match value.value() {
        Either::Right(v) => Hsla::parse_hex(&v.0).unwrap(),
        Either::Left(v) => match v {
            ColorName::Primary => cx.theme().primary,
            ColorName::Secondary => cx.theme().secondary,
            ColorName::Success => cx.theme().success,
            ColorName::Warning => cx.theme().warning,
            ColorName::Danger | ColorName::Error => cx.theme().danger,
            ColorName::Info => cx.theme().info,
            ColorName::Foreground => cx.theme().foreground,
            ColorName::Muted => cx.theme().muted_foreground,
            ColorName::Border => cx.theme().border,
        },
    }
}
fn icon(path: &Option<String>) -> Option<Icon> {
    path.clone().map(|p| Icon::default().path(p))
}
fn axis(vertical: bool) -> Axis {
    if vertical {
        Axis::Vertical
    } else {
        Axis::Horizontal
    }
}

#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct ControlOption {
    value: String,
    label: String,
    #[prop(default)]
    disabled: bool,
    icon: Option<String>,
    tooltip: Option<String>,
}
#[derive(Clone, PartialEq, Default)]
pub(crate) struct ControlItems(Vec<ControlOption>);
impl JsValue for ControlItems {
    fn ts_type(types: &mut TsTypes) -> String {
        Vec::<ControlOption>::ts_type(types)
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        let items = Vec::<ControlOption>::from_js(value)?;
        Ok(Self(items))
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        self.0.to_js(host)
    }
}
fn validate_items(items: &ControlItems) -> Result<(), PropError> {
    let mut seen = BTreeSet::new();
    if items
        .0
        .iter()
        .any(|i| i.value.is_empty() || !seen.insert(&i.value))
    {
        Err(PropError(
            "items must have unique nonempty string values".into(),
        ))
    } else {
        Ok(())
    }
}

struct Host<E: nsh_core::ui::Event> {
    context: TypedComponentContext<E>,
    handles: BTreeMap<String, FocusHandle>,
    available: Vec<FocusHandle>,
    active: Vec<FocusHandle>,
    tab_index: isize,
}
impl<E: nsh_core::ui::Event> Host<E> {
    fn new(context: TypedComponentContext<E>) -> Self {
        Self {
            context,
            handles: BTreeMap::new(),
            available: vec![],
            active: vec![],
            tab_index: 0,
        }
    }
    fn single(&mut self, cx: &mut App, enabled: bool, tab: bool) -> FocusHandle {
        let h = self
            .handles
            .entry(String::new())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        self.available = if enabled { vec![h.clone()] } else { vec![] };
        self.active = if tab { self.available.clone() } else { vec![] };
        h
    }
    fn group(&mut self, items: &ControlItems, cx: &mut App, disabled: bool) {
        self.handles
            .retain(|key, _| items.0.iter().any(|i| &i.value == key));
        for i in &items.0 {
            self.handles
                .entry(i.value.clone())
                .or_insert_with(|| cx.focus_handle());
        }
        self.available = items
            .0
            .iter()
            .filter(|i| !disabled && !i.disabled)
            .filter_map(|i| self.handles.get(&i.value).cloned())
            .collect();
        self.active = self.available.clone();
    }
    fn command(&mut self, name: &str, args: &Value, window: &mut Window, cx: &mut App) {
        if name == "focus" {
            let requested = args
                .as_str()
                .and_then(|v| self.handles.get(v))
                .filter(|h| self.available.contains(h))
                .cloned();
            if let Some(h) = requested.or_else(|| self.available.first().cloned()) {
                window.focus(&h, cx);
            }
        }
    }
}
// Lifecycle stays ordinary Rust. This only removes repeated Props/trait plumbing.
macro_rules! control {
    ($name:ident($props:ident,$event:ty){$($(#[$attr:meta])* $field:ident:$ty:ty),*$(,)?} |$p:ident,$host:ident,$window:ident,$cx:ident| $body:block $(validate $validate:expr)?) => {
        #[derive(Clone,PartialEq,Props)] pub(crate) struct $props{$($(#[$attr])* $field:$ty),*}
        pub(crate) struct $name {host:Host<$event>}
        impl $name {fn new(context:TypedComponentContext<$event>)->Self{Self{host:Host::new(context)}}}
        impl TypedNativeComponent for $name{
            type Props=$props;type Event=$event;
            fn render(&mut self,$p:&$props,$window:&mut Window,$cx:&mut App)->Option<AnyElement>{let $host=&mut self.host;let _=(&$p,&$host,&$window,&$cx);$body}
            fn validate_props(&self,p:&$props)->Result<(),PropError>{let _=p;control!(@validate p $(,$validate)?)}
            fn focus_handle(&self)->Option<FocusHandle>{self.host.available.first().cloned()}
            fn tab_index(&self)->isize{self.host.tab_index}
            fn focus_handles(&self)->Vec<FocusHandle>{self.host.active.clone()}
            fn command(&mut self,_:&$props,name:&str,args:&Value,window:&mut Window,cx:&mut App){self.host.command(name,args,window,cx);}
        }
    };
    (@validate $p:ident) => { Ok(()) };
    (@validate $p:ident,$validator:expr) => { ($validator)($p) };
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum BoolEvent {
    Change(bool),
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum RatingEvent {
    Change(usize),
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum GroupEvent {
    Change(Vec<String>),
    #[allow(dead_code)] // Shared prop decoder emits this event.
    Error(String),
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum RadioGroupEvent {
    Change(String),
    #[allow(dead_code)] // Shared prop decoder emits this event.
    Error(String),
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum ClipboardEvent {
    Copied(String),
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum AlertEvent {
    Close,
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum LinkEvent {
    Press,
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum KbdEvent {
    #[allow(dead_code)] // Shared prop decoder emits this event.
    Error(String),
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum DropdownEvent {
    Press,
    #[allow(dead_code)] // Shared popup helper emits this event.
    Select(String),
}

native_leaf!(IconControl(IconProps,context,cx,p)=>Icon::default()
    .when_some(p.path.clone(),|i,path|i.path(path))
    .when_some(p.svg.as_ref(),|i,v|i.data(v.as_bytes()))
    .rotate(Radians(p.rotation.unwrap_or(0.) as f32))
    .when_some(p.color.as_ref(),|i,v|i.text_color(color(v,cx)));{
    path:Option<String> =>skip;
    svg:Option<String> =>skip;
    color:Option<ComponentColor> =>skip;
    rotation:Option<f64> =>skip;
    size:Option<ComponentSize> =>size(size);
});
native_leaf!(Badge(BadgeProps,context,cx,p)=>component::badge::Badge::new()
    .count(p.count.unwrap_or(0)).max(p.max.unwrap_or(99))
    .when_some(icon(&p.icon),|b,i|b.icon(i))
    .when_some(p.color.as_ref(),|b,c|b.color(color(c,cx)))
    .children(crate::bridge::children(context.raw()));{
    children:Children=>skip;count:Option<usize> =>skip;max:Option<usize> =>skip;dot:bool=>flag;icon:Option<String> =>skip;color:Option<ComponentColor> =>skip;size:Option<ComponentSize> =>size(size);
});
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum TagVariant {
    Primary,
    Secondary,
    Danger,
    Success,
    Warning,
    Info,
}
fn tag_variant(v: Option<TagVariant>) -> component::tag::TagVariant {
    use component::tag::TagVariant as V;
    match v {
        Some(TagVariant::Primary) => V::Primary,
        Some(TagVariant::Danger) => V::Danger,
        Some(TagVariant::Success) => V::Success,
        Some(TagVariant::Warning) => V::Warning,
        Some(TagVariant::Info) => V::Info,
        _ => V::Secondary,
    }
}
native_leaf!(Tag(TagProps,context,cx,p)=>component::tag::Tag::new()
    .with_variant(tag_variant(p.variant)).child(p.label.clone().unwrap_or_default())
    .when(p.rounded_full,|t|t.rounded_full()).when_some(p.radius,|t,r|t.rounded(px(r as f32)))
    .children(crate::bridge::children(context.raw()));{
    children:Children=>skip;label:Option<String> =>skip;variant:Option<TagVariant> =>skip;outline:bool=>flag;rounded_full:bool=>skip;radius:Option<f64> =>skip;size:Option<ComponentSize> =>size(size);
});
native_leaf!(Spinner(SpinnerProps,context,cx,p)=>component::spinner::Spinner::new().when_some(p.color.as_ref(),|s,c|s.color(color(c,cx)));{
    icon:Option<String> =>mapped(|path|Icon::default().path(path));color:Option<ComponentColor> =>skip;size:Option<ComponentSize> =>size(size);
});
native_leaf!(Separator(SeparatorProps,context,cx,p)=>(if p.vertical{component::separator::Separator::vertical()}else{component::separator::Separator::horizontal()})
    .when_some(p.color.as_ref(),|s,c|s.color(color(c,cx)));{
    vertical:bool=>skip;dashed:bool=>flag;label:Option<String> =>optional;color:Option<ComponentColor> =>skip;
});
native_leaf!(MarkerIcon(MarkerIconProps,context,cx,p)=>component::marker::MarkerIcon::new().children(crate::bridge::children(context.raw())).children(icon(&p.icon));{children:Children=>skip;icon:Option<String> =>skip;});
native_leaf!(MarkerContent(MarkerContentProps,context,cx,p)=>component::marker::MarkerContent::new().text(p.text.clone().unwrap_or_default()).children(crate::bridge::children(context.raw()));{children:Children=>skip;text:Option<String> =>skip;});

control!(AvatarGroup(AvatarGroupProps,nsh_core::ui::NoEvent) {
    items: Vec<AvatarProps>,
    size: Option<ComponentSize>,
    limit: Option<usize>,
    #[prop(default)] ellipsis: bool,
} |p, h, w, cx| {
    Some(
        component::avatar::AvatarGroup::new()
            .with_size(size(&p.size))
            .limit(p.limit.unwrap_or(3))
            .when(p.ellipsis, |a| a.ellipsis())
            .children(p.items.iter().map(|i| {
                component::avatar::Avatar::new()
                    .with_size(size(&p.size))
                    .when_some(text(&i.name), |a, v| a.name(v))
                    .when_some(text(&i.src), |a, v| a.src(v))
                    .when_some(icon(&i.placeholder), |a, v| a.placeholder(v))
            }))
            .into_any_element(),
    )
});

control!(Label(LabelProps,nsh_core::ui::NoEvent) {
    label: String,
    secondary: Option<String>,
    #[prop(default)] masked: bool,
    highlights: Option<String>,
    #[prop(default)] prefix_match: bool,
} |p, h, w, cx| {
    Some(
        component::label::Label::new(p.label.clone())
            .masked(p.masked)
            .when_some(text(&p.secondary), |l, v| l.secondary(v))
            .when_some(text(&p.highlights), |l, v| {
                l.highlights(if p.prefix_match {
                    component::label::HighlightsMatch::Prefix(v)
                } else {
                    component::label::HighlightsMatch::Full(v)
                })
            })
            .into_any_element(),
    )
});
control!(Kbd(KbdProps,KbdEvent) {
    keystroke: String,
    #[prop(default = true)]appearance: bool,
    #[prop(default)] outline: bool,
} |p, h, w, cx| {
    match Keystroke::parse(&p.keystroke) {
        Ok(stroke) => Some(
            component::kbd::Kbd::new(stroke)
                .appearance(p.appearance)
                .when(p.outline, |k| k.outline())
                .into_any_element(),
        ),
        Err(error) => {
            h.context.emit(KbdEvent::Error(error.to_string()));
            None
        }
    }
});
control!(Link(LinkProps,LinkEvent) {
    children: Children,
    label: Option<String>,
    href: Option<String>,
    #[prop(default)] disabled: bool,
} |p, h, w, cx| {
    let event = h.context.clone();
    let disabled = p.disabled;
    Some(
        component::link::Link::new("control")
            .when_some(text(&p.href), |l, v| l.href(v))
            .disabled(p.disabled)
            .child(p.label.clone().unwrap_or_default())
            .children(crate::bridge::children(h.context.raw()))
            .on_click(move |_, _, _| {
                if !disabled {
                    event.emit(LinkEvent::Press);
                }
            })
            .into_any_element(),
    )
});

#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum AlertVariant {
    Default,
    Info,
    Success,
    Warning,
    Error,
}
control!(Alert(AlertProps,AlertEvent) {
    message: String,
    title: Option<String>,
    variant: Option<AlertVariant>,
    icon: Option<String>,
    #[prop(default = true)]visible: bool,
    #[prop(default)] banner: bool,
    #[prop(default)] closable: bool,
    size: Option<ComponentSize>,
} |p, h, w, cx| {
    let event = h.context.clone();
    Some(
        component::alert::Alert::new("control", p.message.clone())
            .with_variant(match p.variant {
                Some(AlertVariant::Info) => component::alert::AlertVariant::Info,
                Some(AlertVariant::Success) => component::alert::AlertVariant::Success,
                Some(AlertVariant::Warning) => component::alert::AlertVariant::Warning,
                Some(AlertVariant::Error) => component::alert::AlertVariant::Error,
                _ => component::alert::AlertVariant::Default,
            })
            .visible(p.visible)
            .with_size(size(&p.size))
            .when_some(text(&p.title), |a, v| a.title(v))
            .when_some(icon(&p.icon), |a, v| a.icon(v))
            .when(p.banner, |a| a.banner())
            .when(p.closable, |a| {
                a.on_close(move |_, _, _| event.emit(AlertEvent::Close))
            })
            .into_any_element(),
    )
});
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum MarkerVariant {
    Plain,
    Separator,
    Border,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum MarkerAlignment {
    Start,
    Center,
    End,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum MarkerLoadingStyle {
    Spinner,
    Shimmer,
}
fn shimmer(
    duration: Option<f64>,
    spread: Option<f64>,
    reverse: bool,
    once: bool,
    highlight: &Option<ComponentColor>,
    cx: &App,
) -> component::shimmer::ShimmerStyle {
    let style = component::shimmer::ShimmerStyle::new()
        .duration(Duration::from_secs_f64(
            duration.unwrap_or(2.).clamp(0.001, 86400.),
        ))
        .spread(spread.unwrap_or(0.3) as f32)
        .reverse(reverse)
        .once(once);
    match highlight {
        Some(c) => style.highlight_color(color(c, cx)),
        None => style,
    }
}
native_leaf!(Marker(MarkerProps,context,cx,p)=>component::marker::Marker::new()
    .with_variant(match p.variant{Some(MarkerVariant::Separator)=>component::marker::MarkerVariant::Separator,Some(MarkerVariant::Border)=>component::marker::MarkerVariant::Border,_=>component::marker::MarkerVariant::Plain})
    .alignment(match p.alignment{Some(MarkerAlignment::Center)=>component::marker::MarkerAlignment::Center,Some(MarkerAlignment::End)=>component::marker::MarkerAlignment::End,_=>component::marker::MarkerAlignment::Start})
    .loading(p.loading).with_loading_style(if p.loading_style==Some(MarkerLoadingStyle::Shimmer){component::marker::MarkerLoadingStyle::Shimmer}else{component::marker::MarkerLoadingStyle::Spinner})
    .with_shimmer_style(shimmer(p.duration,p.spread,p.reverse,p.once,&p.highlight_color,cx))
    .when_some(icon(&p.icon),|m,i|m.icon(component::marker::MarkerIcon::new().child(i)))
    .when_some(crate::bridge::slot(context.raw(),"icon"),|m,i|m.icon(component::marker::MarkerIcon::new().child(i)))
    .content(component::marker::MarkerContent::new().text(p.label.clone().unwrap_or_default()).children(crate::bridge::children(context.raw())).children(crate::bridge::slot(context.raw(),"content")));{
    children:Children=>skip;label:Option<String> =>skip;icon:Option<String> =>skip;variant:Option<MarkerVariant> =>skip;alignment:Option<MarkerAlignment> =>skip;loading:bool=>skip;loading_style:Option<MarkerLoadingStyle> =>skip;duration:Option<f64> =>skip;spread:Option<f64> =>skip;reverse:bool=>skip;once:bool=>skip;highlight_color:Option<ComponentColor> =>skip;
});
control!(ShimmerText(ShimmerTextProps,nsh_core::ui::NoEvent) {
    text: String,
    duration: Option<f64>,
    spread: Option<f64>,
    #[prop(default)] reverse: bool,
    #[prop(default)] once: bool,
    highlight_color: Option<ComponentColor>,
} |p, h, w, cx| {
    Some(
        component::shimmer::ShimmerText::new(p.text.clone())
            .id("control")
            .with_shimmer_style(shimmer(
                p.duration,
                p.spread,
                p.reverse,
                p.once,
                &p.highlight_color,
                cx,
            ))
            .into_any_element(),
    )
});
native_leaf!(Progress(ProgressProps,context,cx,p)=>component::progress::Progress::new("control").value(p.value.unwrap_or(0.) as f32).when_some(p.color.as_ref(),|e,c|e.color(color(c,cx))).when_some(text(&p.accessibility_label),|e,v|e.accessibility_label(v));{value:Option<f64> =>skip;loading:bool=>set;color:Option<ComponentColor> =>skip;accessibility_label:Option<String> =>skip;size:Option<ComponentSize> =>size(size);});
native_leaf!(ProgressCircle(ProgressCircleProps,context,cx,p)=>component::progress::ProgressCircle::new("control").value(p.value.unwrap_or(0.) as f32).when_some(p.color.as_ref(),|e,c|e.color(color(c,cx))).when_some(text(&p.accessibility_label),|e,v|e.accessibility_label(v)).children(crate::bridge::children(context.raw()));{children:Children=>skip;value:Option<f64> =>skip;loading:bool=>set;color:Option<ComponentColor> =>skip;accessibility_label:Option<String> =>skip;size:Option<ComponentSize> =>size(size);});
control!(Rating(RatingProps,RatingEvent) {
    value: usize,
    max: Option<usize>,
    #[prop(default)] disabled: bool,
    color: Option<ComponentColor>,
    size: Option<ComponentSize>,
} |p, h, w, cx| {
    let event = h.context.clone();
    Some(
        component::rating::Rating::new("rating")
            .with_size(size(&p.size))
            .disabled(p.disabled)
            .max(p.max.unwrap_or(5))
            .controlled_value(p.value)
            .when_some(p.color.as_ref(), |r, c| r.color(color(c, cx)))
            .on_click(move |v, _, _| event.emit(RatingEvent::Change(*v)))
            .into_any_element(),
    )
});
control!(Clipboard(ClipboardProps,ClipboardEvent) {
    value: String,
    tooltip: Option<String>,
    accessibility_label: Option<String>,
    size: Option<ComponentSize>,
} |p, h, w, cx| {
    let focus = h.single(cx, true, true);
    let event = h.context.clone();
    Some(
        component::clipboard::Clipboard::new("control")
            .track_focus(&focus)
            .with_size(if p.size.is_some() {
                size(&p.size)
            } else {
                Size::XSmall
            })
            .value(p.value.clone())
            .when_some(text(&p.tooltip), |c, v| c.tooltip(v))
            .when_some(text(&p.accessibility_label), |c, v| {
                c.accessibility_label(v)
            })
            .on_copied(move |v, _, _| event.emit(ClipboardEvent::Copied(v.to_string())))
            .into_any_element(),
    )
});

control!(Checkbox(CheckboxProps,BoolEvent) {
    children: Children,
    label: Option<String>,
    checked: bool,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    #[prop(default = true)]tab_stop: bool,
    #[prop(default)] tab_index: i64,
    accessibility_label: Option<String>,
    tooltip: Option<String>,
} |p, h, w, cx| {
    h.tab_index = p.tab_index as isize;
    let focus = h.single(cx, !p.disabled, p.tab_stop && p.tab_index >= 0);
    let event = h.context.clone();
    Some(
        component::checkbox::Checkbox::new("control")
            .track_focus(&focus)
            .when_some(text(&p.label), |e, v| e.label(v))
            .checked(p.checked)
            .disabled(p.disabled)
            .with_size(size(&p.size))
            .children(crate::bridge::children(h.context.raw()))
            .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled)
            .tab_index(p.tab_index as isize)
            .when_some(text(&p.tooltip), |e, v| e.tooltip(v))
            .when_some(text(&p.accessibility_label), |e, v| {
                e.accessibility_label(v)
            })
            .on_change(move |v, _, _| event.emit(BoolEvent::Change(*v)))
            .into_any_element(),
    )
});
control!(Radio(RadioProps,BoolEvent) {
    children: Children,
    label: Option<String>,
    checked: bool,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    #[prop(default = true)]tab_stop: bool,
    #[prop(default)] tab_index: i64,
    accessibility_label: Option<String>,
    tooltip: Option<String>,
} |p, h, w, cx| {
    h.tab_index = p.tab_index as isize;
    let focus = h.single(cx, !p.disabled, p.tab_stop && p.tab_index >= 0);
    let event = h.context.clone();
    Some(
        component::radio::Radio::new("control")
            .track_focus(&focus)
            .when_some(text(&p.label), |e, v| e.label(v))
            .checked(p.checked)
            .disabled(p.disabled)
            .with_size(size(&p.size))
            .children(crate::bridge::children(h.context.raw()))
            .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled)
            .tab_index(p.tab_index as isize)
            .when_some(text(&p.tooltip), |e, v| e.tooltip(v))
            .when_some(text(&p.accessibility_label), |e, v| {
                e.accessibility_label(v)
            })
            .on_change(move |v, _, _| event.emit(BoolEvent::Change(*v)))
            .into_any_element(),
    )
});
control!(Switch(SwitchProps,BoolEvent) {
    label: Option<String>,
    checked: bool,
    color: Option<ComponentColor>,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    #[prop(default = true)]tab_stop: bool,
    #[prop(default)] tab_index: i64,
    accessibility_label: Option<String>,
    tooltip: Option<String>,
} |p, h, w, cx| {
    h.tab_index = p.tab_index as isize;
    let focus = h.single(cx, !p.disabled, p.tab_stop && p.tab_index >= 0);
    let event = h.context.clone();
    Some(
        component::switch::Switch::new("control")
            .track_focus(&focus)
            .when_some(text(&p.label), |e, v| e.label(v))
            .checked(p.checked)
            .disabled(p.disabled)
            .with_size(size(&p.size))
            .when_some(p.color.as_ref(), |e, v| e.color(color(v, cx)))
            .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled)
            .tab_index(p.tab_index as isize)
            .when_some(text(&p.tooltip), |e, v| e.tooltip(v))
            .when_some(text(&p.accessibility_label), |e, v| {
                e.accessibility_label(v)
            })
            .on_change(move |v, _, _| event.emit(BoolEvent::Change(*v)))
            .into_any_element(),
    )
});
control!(Toggle(ToggleProps,BoolEvent) {
    children: Children,
    label: Option<String>,
    checked: bool,
    #[prop(default)] disabled: bool,
    icon: Option<String>,
    tooltip: Option<String>,
    accessibility_label: Option<String>,
    #[prop(default)] outline: bool,
    size: Option<ComponentSize>,
} |p, h, w, cx| {
    let focus = h.single(cx, !p.disabled, true);
    let event = h.context.clone();
    Some(
        component::button::Toggle::new("control")
            .track_focus(&focus)
            .when_some(text(&p.label), |e, v| e.label(v))
            .checked(p.checked)
            .disabled(p.disabled)
            .with_size(size(&p.size))
            .with_variant(if p.outline {
                component::button::ToggleVariant::Outline
            } else {
                component::button::ToggleVariant::Ghost
            })
            .when_some(icon(&p.icon), |e, v| e.icon(v))
            .when_some(text(&p.tooltip), |e, v| e.tooltip(v))
            .when_some(text(&p.accessibility_label), |e, v| {
                e.accessibility_label(v)
            })
            .children(crate::bridge::children(h.context.raw()))
            .on_click(move |v, _, _| event.emit(BoolEvent::Change(*v)))
            .into_any_element(),
    )
});

crate::registration::named_value! { #[derive(Default)] pub(crate) struct RadioSelection(Option<String>); }
control!(RadioGroup(RadioGroupProps,RadioGroupEvent) {
    items: ControlItems,
    value: RadioSelection,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    #[prop(default = true)]vertical: bool,
} |p, h, w, cx| {
    h.group(&p.items, cx, p.disabled);
    let event = h.context.clone();
    let values = p
        .items
        .0
        .iter()
        .map(|i| i.value.clone())
        .collect::<Vec<_>>();
    Some(
        component::radio::RadioGroup::new("control")
            .layout(axis(p.vertical))
            .disabled(p.disabled)
            .selected_index(
                p.items
                    .0
                    .iter()
                    .position(|i| Some(&i.value) == p.value.value().as_ref()),
            )
            .children(p.items.0.iter().map(|i| {
                component::radio::Radio::new(i.value.clone())
                    .track_focus(&h.handles[&i.value])
                    .with_size(size(&p.size))
                    .label(i.label.clone())
                    .disabled(i.disabled)
                    .children(icon(&i.icon))
                    .when_some(text(&i.tooltip), |r, v| r.tooltip(v))
            }))
            .on_change(move |index, _, _| {
                if let Some(v) = values.get(*index) {
                    event.emit(RadioGroupEvent::Change(v.clone()));
                }
            })
            .into_any_element(),
    )
} validate |p: &RadioGroupProps| validate_items(&p.items));
control!(ToggleGroup(ToggleGroupProps,GroupEvent) {
    items: ControlItems,
    value: Vec<String>,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    #[prop(default)] outline: bool,
    #[prop(default)] segmented: bool,
} |p, h, w, cx| {
    h.group(&p.items, cx, p.disabled);
    let event = h.context.clone();
    let values = p
        .items
        .0
        .iter()
        .map(|i| i.value.clone())
        .collect::<Vec<_>>();
    Some(
        component::button::ToggleGroup::new("control")
            .with_size(size(&p.size))
            .disabled(p.disabled)
            .with_variant(if p.outline {
                component::button::ToggleVariant::Outline
            } else {
                component::button::ToggleVariant::Ghost
            })
            .when(p.segmented, |g| g.segmented())
            .children(p.items.0.iter().map(|i| {
                component::button::Toggle::new(i.value.clone())
                    .track_focus(&h.handles[&i.value])
                    .label(i.label.clone())
                    .checked(p.value.contains(&i.value))
                    .disabled(p.disabled || i.disabled)
                    .when_some(icon(&i.icon), |t, v| t.icon(v))
                    .when_some(text(&i.tooltip), |t, v| t.tooltip(v))
            }))
            .on_click(move |flags, _, _| {
                event.emit(GroupEvent::Change(
                    flags
                        .iter()
                        .zip(&values)
                        .filter(|(checked, _)| **checked)
                        .map(|(_, v)| v.clone())
                        .collect(),
                ))
            })
            .into_any_element(),
    )
} validate |p: &ToggleGroupProps| validate_items(&p.items));
control!(ButtonGroup(ButtonGroupProps,GroupEvent) {
    items: ControlItems,
    value: Vec<String>,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    #[prop(default)] vertical: bool,
    #[prop(default)] multiple: bool,
    #[prop(default)] variant: ButtonVariant,
    #[prop(default)] outline: bool,
    #[prop(default)] compact: bool,
} |p, h, w, cx| {
    h.group(&p.items, cx, p.disabled);
    Some(
        component::button::ButtonGroup::new("control")
            .with_size(size(&p.size))
            .disabled(p.disabled)
            .layout(axis(p.vertical))
            .multiple(p.multiple)
            .with_variant(p.variant.resolve())
            .when(p.outline, |g| g.outline())
            .when(p.compact, |g| g.compact())
            .children(p.items.0.iter().map(|i| {
                let checked = p.value.contains(&i.value);
                let selected = p.value.clone();
                let value = i.value.clone();
                let multiple = p.multiple;
                let event = h.context.clone();
                component::button::Button::new(i.value.clone())
                    .track_focus(&h.handles[&i.value])
                    .label(i.label.clone())
                    .selected(checked)
                    .disabled(p.disabled || i.disabled)
                    .when_some(icon(&i.icon), |t, v| t.icon(v))
                    .when_some(text(&i.tooltip), |t, v| t.tooltip(v))
                    .on_click(move |_, _, _| {
                        let next = if !multiple {
                            vec![value.clone()]
                        } else if checked {
                            selected.iter().filter(|v| *v != &value).cloned().collect()
                        } else {
                            selected
                                .iter()
                                .cloned()
                                .chain(std::iter::once(value.clone()))
                                .collect()
                        };
                        event.emit(GroupEvent::Change(next));
                    })
            }))
            .into_any_element(),
    )
} validate |p: &ButtonGroupProps| validate_items(&p.items));

control!(DropdownButton(DropdownButtonProps,DropdownEvent) {
    items: Vec<crate::menus::MenuItemValue>,
    label: Option<String>,
    #[prop(default)] variant: ButtonVariant,
    icon: Option<ButtonIcon>,
    #[prop(default)] selected: bool,
    #[prop(default)] outline: bool,
    size: Option<ComponentSize>,
    #[prop(default)] disabled: bool,
    accessibility_label: Option<String>,
    tooltip: Option<String>,
    menu_accessibility_label: Option<String>,
} |p, h, w, cx| {
    let focus = h.single(cx, !p.disabled, true);
    let menu_focus = h
        .handles
        .entry("menu".into())
        .or_insert_with(|| cx.focus_handle())
        .clone();
    if !p.disabled {
        h.available.push(menu_focus.clone());
        h.active.push(menu_focus.clone());
    }
    let event = h.context.clone();
    let raw = h.context.raw().clone();
    let items = p.items.clone();
    Some(
        component::button::DropdownButton::new("control")
            .track_menu_focus(&menu_focus)
            .with_size(size(&p.size))
            .disabled(p.disabled)
            .with_variant(p.variant.resolve())
            .selected(p.selected)
            .when(p.outline, |b| b.outline())
            .when_some(text(&p.menu_accessibility_label), |b, v| {
                b.menu_accessibility_label(v)
            })
            .button(
                component::button::Button::new("action")
                    .track_focus(&focus)
                    .when_some(text(&p.label), |b, v| b.label(v))
                    .disabled(p.disabled)
                    .map(|b| apply_icon(b, p.icon.as_ref(), &p.size))
                    .when_some(text(&p.accessibility_label), |b, v| {
                        b.accessibility_label(v)
                    })
                    .when_some(text(&p.tooltip), |b, v| b.tooltip(v))
                    .on_click(move |_, _, _| event.emit(DropdownEvent::Press)),
            )
            .dropdown_menu(move |menu, window, cx| {
                crate::surfaces::typed_menu_items(menu, &items, &raw, window, cx)
            })
            .into_any_element(),
    )
} validate |p: &DropdownButtonProps| crate::menus::validate(&p.items, &mut std::collections::HashSet::new(), 0).map_err(PropError));

pub(crate) fn register(registry: &mut impl BindingRegistry) {
    macro_rules! add {($($name:ident),*$(,)?)=>{$(registry.register::<$name>(concat!("gpui-component.",stringify!($name)),stringify!($name),$name::new);)*};}
    registry.register::<IconControl>("gpui-component.Icon", "Icon", IconControl::new);
    add!(
        AvatarGroup,
        Badge,
        Tag,
        Alert,
        Marker,
        MarkerIcon,
        MarkerContent,
        Spinner,
        ShimmerText,
        Progress,
        ProgressCircle,
        Separator,
        Kbd,
        Label,
        Clipboard,
        Rating,
        Checkbox,
        Switch,
        Radio,
        Toggle,
        RadioGroup,
        ToggleGroup,
        ButtonGroup,
        Link,
        DropdownButton
    );
}

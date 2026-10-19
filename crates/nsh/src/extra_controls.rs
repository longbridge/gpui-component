//! Typed NSH color editors and live native step navigation.
use crate::{datetime::component_size, display::ComponentSize};
use gpui_kit::{
    Anchor, AnyElement, App, AppContext, Entity, FocusHandle, Focusable, Hsla, IntoElement,
    ParentElement, Styled, Subscription, Window,
    component::{
        Colorize, Icon, Sizable,
        color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState, ColorSelect},
        stepper::{Stepper, StepperItem},
    },
    prelude::FluentBuilder,
};
use nsh_core::ui::{
    AnyValue, Children, Either, Event, JsHost, JsIn, JsObject, JsOwned, JsValue, PropEnum,
    PropError, Props, Readonly, TsTypes,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::{cell::RefCell, collections::HashSet, marker::PhantomData, rc::Rc};

/// Normalized native HSLA channels, preserving more precision than hex transport.
#[derive(Clone, PartialEq, JsObject)]
struct HslaColor {
    h: f64,
    s: f64,
    l: f64,
    a: f64,
}
impl From<Hsla> for HslaColor {
    fn from(v: Hsla) -> Self {
        Self {
            h: v.h as f64,
            s: v.s as f64,
            l: v.l as f64,
            a: v.a as f64,
        }
    }
}
#[derive(Clone, PartialEq)]
struct ColorValue(Either<String, HslaColor>);
impl ColorValue {
    fn native(&self) -> Result<Hsla, String> {
        match &self.0 {
            Either::Left(text) => {
                let hex = text.strip_prefix('#').unwrap_or(text);
                if !matches!(hex.len(), 3 | 4 | 6 | 8)
                    || !hex.bytes().all(|v| v.is_ascii_hexdigit())
                {
                    return Err("Color must be RGB, RGBA, RRGGBB or RRGGBBAA hexadecimal".into());
                }
                let hex = if hex.len() <= 4 {
                    hex.chars().flat_map(|c| [c, c]).collect::<String>()
                } else {
                    hex.to_owned()
                };
                Hsla::parse_hex(&hex).map_err(|_| "Invalid hexadecimal color".into())
            }
            Either::Right(v) => {
                if [v.h, v.s, v.l, v.a]
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                {
                    return Err("HSLA channels must be in 0 through 1".into());
                }
                Ok(Hsla {
                    h: v.h as f32,
                    s: v.s as f32,
                    l: v.l as f32,
                    a: v.a as f32,
                })
            }
        }
    }
    fn hex_echo(&self) -> Option<String> {
        let Either::Left(value) = &self.0 else {
            return None;
        };
        let value = value.strip_prefix('#').unwrap_or(value);
        let mut value = if value.len() <= 4 {
            value.chars().flat_map(|c| [c, c]).collect::<String>()
        } else {
            value.to_owned()
        }
        .to_ascii_uppercase();
        if value.len() == 8 && value.ends_with("FF") {
            value.truncate(6);
        }
        Some(format!("#{value}"))
    }
}
// Domain validation uses NSH's existing codec, so props and commands cannot
// reach the native hex parser with malformed UTF-8 boundaries or invalid channels.
impl JsValue for ColorValue {
    fn ts_type(types: &mut TsTypes) -> String {
        types.declare("ColorValue", |types| {
            format!(
                "export type ColorValue = {};",
                <Option<Either<String, HslaColor>>>::ts_type(types)
            )
        })
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        let value = Self(<Either<String, HslaColor>>::from_js(value)?);
        value.native().map_err(PropError)?;
        Ok(value)
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        self.0.to_js(host)
    }
}
fn decode<T: JsValue + Clone + 'static>(args: &Value) -> Result<T, String> {
    nsh_core::ui::decode_json(|value| Ok(AnyValue::new(T::from_js(value)?)), args.clone())
        .map_err(|e| e.to_string())?
        .take::<T>()
        .ok_or_else(|| "Invalid command argument".into())
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum ColorAnchor {
    #[prop(alias = "top-left")]
    TopLeft,
    #[prop(alias = "top-right")]
    TopRight,
    #[prop(alias = "bottom-left")]
    BottomLeft,
    #[prop(alias = "bottom-right")]
    BottomRight,
    #[prop(alias = "top-center")]
    TopCenter,
    #[prop(alias = "bottom-center")]
    BottomCenter,
    #[prop(alias = "left-center")]
    LeftCenter,
    #[prop(alias = "right-center")]
    RightCenter,
}
impl ColorAnchor {
    fn native(self) -> Anchor {
        match self {
            Self::TopLeft => Anchor::TopLeft,
            Self::TopRight => Anchor::TopRight,
            Self::BottomLeft => Anchor::BottomLeft,
            Self::BottomRight => Anchor::BottomRight,
            Self::TopCenter => Anchor::TopCenter,
            Self::BottomCenter => Anchor::BottomCenter,
            Self::LeftCenter => Anchor::LeftCenter,
            Self::RightCenter => Anchor::RightCenter,
        }
    }
}
// Native state inputs only; presentation fields remain on their typed builders.
struct ColorSettings {
    value: Option<ColorValue>,
    open: Option<bool>,
    active_tab: Option<usize>,
    size: Option<ComponentSize>,
    featured_colors: Option<Readonly<Vec<ColorValue>>>,
}
macro_rules! color_props {
    ($($(#[$attr:meta])* $field:ident:$ty:ty,)*)=>{
        #[derive(Clone,PartialEq,Props)] struct ColorPickerProps{$($(#[$attr])* $field:$ty,)* icon:Option<String>,anchor:Option<ColorAnchor>,}
        #[derive(Clone,PartialEq,Props)] struct ColorSelectProps{$($(#[$attr])* $field:$ty,)* placeholder:Option<String>,}
        impl ColorProps for ColorPickerProps {
            fn settings(&self)->ColorSettings{color_props!(@settings self)}
            fn render(&self,state:&Entity<ColorPickerState>)->Result<AnyElement,String>{
                Ok(ColorPicker::new(state).with_size(component_size(&self.size)?).anchor(self.anchor.unwrap_or(ColorAnchor::TopLeft).native())
                    .when_some(featured_colors(&self.featured_colors)?,|picker,colors|picker.featured_colors(colors))
                    .when_some(self.label.clone(),|picker,label|picker.label(label))
                    .when_some(self.accessibility_label.clone(),|picker,label|picker.accessibility_label(label))
                    .when_some(self.icon.clone(),|picker,path|picker.icon(Icon::default().path(path))).into_any_element())
            }
        }
        impl ColorProps for ColorSelectProps {
            fn settings(&self)->ColorSettings{color_props!(@settings self)}
            fn render(&self,state:&Entity<ColorPickerState>)->Result<AnyElement,String>{
                Ok(ColorSelect::new(state).with_size(component_size(&self.size)?)
                    .when_some(featured_colors(&self.featured_colors)?,|picker,colors|picker.featured_colors(colors))
                    .when_some(self.placeholder.clone().or(self.label.clone()),|picker,label|picker.placeholder(label))
                    .when_some(self.accessibility_label.clone().or(self.label.clone()),|picker,label|picker.accessibility_label(label)).into_any_element())
            }
        }
    };
    (@settings $p:ident) => { ColorSettings {
        value:$p.value.clone(),open:$p.open,active_tab:$p.active_tab,
        size:$p.size.clone(),featured_colors:$p.featured_colors.clone(),
    }};
}
color_props! {
    value:Option<ColorValue>,open:Option<bool>,active_tab:Option<usize>,size:Option<ComponentSize>,
    label:Option<String>,accessibility_label:Option<String>,featured_colors:Option<Readonly<Vec<ColorValue>>>,
}
trait ColorProps: Props {
    fn settings(&self) -> ColorSettings;
    fn render(&self, state: &Entity<ColorPickerState>) -> Result<AnyElement, String>;
}
fn featured_colors(
    colors: &Option<Readonly<Vec<ColorValue>>>,
) -> Result<Option<Vec<Hsla>>, String> {
    colors
        .as_ref()
        .map(|v| v.iter().map(ColorValue::native).collect())
        .transpose()
}
#[derive(Clone, PartialEq, JsObject)]
struct ColorSnapshot {
    value: Option<String>,
    hsla: Option<HslaColor>,
    preview: Option<String>,
    preview_hsla: Option<HslaColor>,
    displayed_color: Option<String>,
    displayed_hsla: Option<HslaColor>,
    focused: bool,
    open: bool,
    active_tab: usize,
    hex_text: String,
    sliders: HslaColor,
}
#[derive(Clone, Event)]
enum ColorEvents {
    Change(Option<String>),
    HslaChange(Option<HslaColor>),
    OpenChange(bool),
    ActiveTabChange(usize),
    Snapshot(Box<ColorSnapshot>),
    Error(String),
}
#[derive(Default)]
struct Reported {
    open: bool,
    tab: usize,
}
// GPUI's draw-phase invalidation can skip observer effects. Commands delivered
// during layout still reconcile the actual state, not the controlled props.
fn report_state(
    state: &ColorPickerState,
    reported: &RefCell<Reported>,
    context: &TypedComponentContext<ColorEvents>,
) {
    let mut reported = reported.borrow_mut();
    if reported.open != state.is_open() {
        reported.open = state.is_open();
        context.emit(ColorEvents::OpenChange(reported.open));
    }
    if reported.tab != state.active_tab() {
        reported.tab = state.active_tab();
        context.emit(ColorEvents::ActiveTabChange(reported.tab));
    }
}
struct NativeColor<P: ColorProps> {
    context: TypedComponentContext<ColorEvents>,
    state: Option<Entity<ColorPickerState>>,
    subscriptions: Vec<Subscription>,
    reported: Rc<RefCell<Reported>>,
    focus: Option<FocusHandle>,
    pending: Vec<String>,
    props: PhantomData<P>,
}
impl<P: ColorProps> NativeColor<P> {
    fn new(context: TypedComponentContext<ColorEvents>) -> Self {
        Self {
            context,
            state: None,
            subscriptions: Vec::new(),
            reported: Rc::default(),
            focus: None,
            pending: vec!["value".into(), "open".into(), "activeTab".into()],
            props: PhantomData,
        }
    }
    fn sync(
        &mut self,
        p: &P,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ColorPickerState>, String> {
        let p = p.settings();
        component_size(&p.size)?;
        if p.active_tab.is_some_and(|v| v > 1) {
            return Err("activeTab must be 0 (palette) or 1 (HSLA)".into());
        }
        let value = p.value.as_ref().map(ColorValue::native).transpose()?;
        featured_colors(&p.featured_colors)?;
        // The callbacks only read state and emit events; no Window access is needed.
        if self.state.is_none() {
            let state = cx.new(|cx| ColorPickerState::new(window, cx));
            let context = self.context.clone();
            self.subscriptions
                .push(cx.subscribe(&state, move |_, event, _| {
                    let ColorPickerEvent::Change(value) = event;
                    context.emit(ColorEvents::Change(value.map(|v| v.to_hex())));
                    context.emit(ColorEvents::HslaChange(value.map(Into::into)));
                    context.redraw();
                }));
            let context = self.context.clone();
            let reported = self.reported.clone();
            self.subscriptions
                .push(cx.observe(&state, move |state, cx| {
                    report_state(state.read(cx), &reported, &context);
                }));
            self.focus = Some(state.focus_handle(cx));
            self.state = Some(state);
        }
        let state = self.state.as_ref().unwrap().clone();
        let fields = std::mem::take(&mut self.pending);
        let open = fields
            .iter()
            .any(|v| v == "open")
            .then_some(p.open.unwrap_or(false));
        let tab = fields
            .iter()
            .any(|v| v == "activeTab")
            .then_some(p.active_tab.unwrap_or(0));
        if let Some(open) = open {
            self.reported.borrow_mut().open = open;
        }
        if let Some(tab) = tab {
            self.reported.borrow_mut().tab = tab;
        }
        state.update(cx, |state, cx| {
            // A matching quantized hex echo must preserve untouched slider channels.
            let echo = p.value.as_ref().and_then(ColorValue::hex_echo);
            if fields.iter().any(|v| v == "value")
                && state.value() != value
                && !echo.is_some_and(|v| state.value().is_some_and(|value| value.to_hex() == v))
            {
                match value {
                    Some(value) => state.set_value(value, window, cx),
                    None => state.clear_value(window, cx),
                }
            }
            if let Some(open) = open {
                state.set_open(open, cx);
            }
            if let Some(tab) = tab {
                state.set_active_tab(tab, cx);
            }
        });
        report_state(state.read(cx), &self.reported, &self.context);
        Ok(state)
    }
    fn snapshot(&self, state: &ColorPickerState, window: &Window, cx: &App) -> ColorSnapshot {
        let sliders = state.sliders();
        ColorSnapshot {
            value: state.value().map(|v| v.to_hex()),
            hsla: state.value().map(Into::into),
            preview: state.preview().map(|v| v.to_hex()),
            preview_hsla: state.preview().map(Into::into),
            displayed_color: state.displayed_color().map(|v| v.to_hex()),
            displayed_hsla: state.displayed_color().map(Into::into),
            focused: state.focus_handle(cx).is_focused(window),
            open: state.is_open(),
            active_tab: state.active_tab(),
            hex_text: state.hex_input().read(cx).value().to_string(),
            sliders: HslaColor {
                h: sliders.hue().read(cx).value().start() as f64,
                s: sliders.saturation().read(cx).value().start() as f64,
                l: sliders.lightness().read(cx).value().start() as f64,
                a: sliders.alpha().read(cx).value().start() as f64,
            },
        }
    }
}
impl<P: ColorProps> TypedNativeComponent for NativeColor<P> {
    type Props = P;
    type Event = ColorEvents;
    fn validate_props(&self, props: &P) -> Result<(), PropError> {
        let props = props.settings();
        component_size(&props.size).map_err(PropError)?;
        if props.active_tab.is_some_and(|tab| tab > 1) {
            return Err(PropError(
                "activeTab must be 0 (palette) or 1 (HSLA)".into(),
            ));
        }
        props
            .value
            .as_ref()
            .map(ColorValue::native)
            .transpose()
            .map_err(PropError)?;
        featured_colors(&props.featured_colors).map_err(PropError)?;
        Ok(())
    }
    fn props_changed(&mut self, _: &P, fields: &[String], _: &mut Window, _: &mut App) {
        self.pending.extend_from_slice(fields);
    }
    fn render(&mut self, p: &P, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        match self.sync(p, window, cx).and_then(|state| p.render(&state)) {
            Ok(v) => Some(v),
            Err(e) => {
                self.context.emit(ColorEvents::Error(e));
                None
            }
        }
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn command(&mut self, p: &P, name: &str, args: &Value, window: &mut Window, cx: &mut App) {
        let state = match self.sync(p, window, cx) {
            Ok(v) => v,
            Err(e) => {
                self.context.emit(ColorEvents::Error(e));
                return;
            }
        };
        let result = state.update(cx, |state, cx| -> Result<(), String> {
            match name {
                "focus" => window.focus(&state.focus_handle(cx), cx),
                "open" => state.set_open(true, cx),
                "close" => state.set_open(false, cx),
                "toggle" => state.toggle_open(cx),
                "clear" => state.clear_value(window, cx),
                "previewColor" => {
                    state.preview_color(decode::<ColorValue>(args)?.native()?, window, cx)
                }
                "clearPreview" => state.clear_preview(window, cx),
                "previewHex" => {
                    if !state.preview_hex(&decode::<String>(args)?, window, cx) {
                        return Err("Invalid preview color".into());
                    }
                }
                "commitHex" => {
                    if state
                        .commit_hex(&decode::<String>(args)?, window, cx)
                        .is_none()
                    {
                        return Err("Invalid committed color".into());
                    }
                }
                "selectColor" => {
                    state.select_color(decode::<ColorValue>(args)?.native()?, window, cx)
                }
                "updateColor" => {
                    state.update_color(decode::<ColorValue>(args)?.native()?, window, cx)
                }
                "setActiveTab" => {
                    let tab = decode::<usize>(args)?;
                    if tab > 1 {
                        return Err("setActiveTab expects 0 or 1".into());
                    }
                    state.set_active_tab(tab, cx);
                }
                "snapshot" => {}
                _ => return Err(format!("Unknown color control command: {name}")),
            }
            Ok(())
        });
        report_state(state.read(cx), &self.reported, &self.context);
        self.context.emit(match result {
            Ok(()) => ColorEvents::Snapshot(Box::new(self.snapshot(state.read(cx), window, cx))),
            Err(e) => ColorEvents::Error(e),
        });
        self.context.redraw();
    }
}
#[derive(Clone, PartialEq, JsObject)]
struct StepperItemProps {
    id: Option<String>,
    #[prop(default)]
    label: String,
    #[prop(default)]
    disabled: bool,
    icon: Option<String>,
}
#[derive(Clone, PartialEq, Props)]
struct StepperProps {
    children: Children,
    #[prop(default)]
    items: Readonly<Vec<StepperItemProps>>,
    #[prop(default)]
    selected_index: usize,
    size: Option<ComponentSize>,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    vertical: bool,
    #[prop(default)]
    text_center: bool,
}
#[derive(Clone, Event)]
enum StepperEvents {
    Change(usize),
    Error(String),
}
fn validate_steps(items: &[StepperItemProps]) -> Result<(), String> {
    let mut identities = HashSet::new();
    if items
        .iter()
        .enumerate()
        .map(|(index, item)| item.id.clone().unwrap_or_else(|| index.to_string()))
        .any(|id| !identities.insert(id))
    {
        return Err("Stepper item ids and fallback indices must be unique".into());
    }
    Ok(())
}
struct NativeStepper {
    context: TypedComponentContext<StepperEvents>,
}
impl TypedNativeComponent for NativeStepper {
    type Props = StepperProps;
    type Event = StepperEvents;
    fn validate_props(&self, props: &StepperProps) -> Result<(), PropError> {
        component_size(&props.size).map_err(PropError)?;
        validate_steps(&props.items).map_err(PropError)
    }
    fn render(&mut self, p: &StepperProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let size = match component_size(&p.size).and_then(|size| {
            validate_steps(&p.items)?;
            Ok(size)
        }) {
            Ok(v) => v,
            Err(e) => {
                self.context.emit(StepperEvents::Error(e));
                return None;
            }
        };
        let context = self.context.clone();
        Some(
            Stepper::new("steps")
                .with_size(size)
                .selected_index(p.selected_index)
                .disabled(p.disabled)
                .text_center(p.text_center)
                .when(p.vertical, |steps| steps.vertical())
                .items(p.items.iter().enumerate().map(|(index, item)| {
                    StepperItem::new()
                        .min_w_0()
                        .disabled(item.disabled)
                        .when_some(item.icon.clone(), |step, path| {
                            step.icon(Icon::default().path(path))
                        })
                        .when(!item.label.is_empty(), |step| {
                            step.child(item.label.clone())
                        })
                        .children(crate::bridge::slot(
                            self.context.raw(),
                            &format!(
                                "step:{}",
                                item.id.clone().unwrap_or_else(|| index.to_string())
                            ),
                        ))
                }))
                .on_click(move |index, _, _| context.emit(StepperEvents::Change(*index)))
                .into_any_element(),
        )
    }
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeColor<ColorPickerProps>>(
        "gpui-component.ColorPicker",
        "ColorPicker",
        NativeColor::new,
    );
    registry.register::<NativeColor<ColorSelectProps>>(
        "gpui-component.ColorSelect",
        "ColorSelect",
        NativeColor::new,
    );
    registry.register::<NativeStepper>("gpui-component.Stepper", "Stepper", |context| {
        NativeStepper { context }
    });
    for name in ["gpui-component.ColorPicker", "gpui-component.ColorSelect"] {
        registry.metadata().native_controlled(
            name,
            &[
                ("change", &["value"]),
                ("hslaChange", &["value"]),
                ("openChange", &["open"]),
                ("activeTabChange", &["activeTab"]),
            ],
        );
    }
    registry
        .metadata()
        .native_controlled("gpui-component.Stepper", &[("change", &["selectedIndex"])]);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn step_identities_cannot_reuse_one_live_subtree_twice() {
        let item = |id: Option<&str>| StepperItemProps {
            id: id.map(str::to_owned),
            label: String::new(),
            disabled: false,
            icon: None,
        };
        assert!(validate_steps(&[item(Some("same")), item(Some("same"))]).is_err());
        assert!(validate_steps(&[item(Some("1")), item(None)]).is_err());
        assert!(validate_steps(&[item(Some("review")), item(Some("start"))]).is_ok());
        assert!(
            decode::<Vec<StepperItemProps>>(&serde_json::json!([{"disabled":"false"}])).is_err()
        );
    }
    #[test]
    fn color_wire_values_validate_before_native_hex_slicing() {
        let c = |value: Value| decode::<ColorValue>(&value).and_then(|v| v.native());
        assert_eq!(
            c(serde_json::json!("#f008")).unwrap(),
            c(serde_json::json!("#FF000088")).unwrap()
        );
        for value in ["aébcdef", "#+1FFFF", "##FFFFFF"] {
            assert!(c(serde_json::json!(value)).is_err());
        }
        assert!(c(serde_json::json!({"h":1.1,"s":0.5,"l":0.5,"a":1})).is_err());
        assert!(
            (c(serde_json::json!({"h":0.4321,"s":0.6789,"l":0.3456,"a":0.7654}))
                .unwrap()
                .h
                - 0.4321)
                .abs()
                < 0.000001
        );
        assert!(component_size(&Some(ComponentSize::new(Either::Right(0.)))).is_err());
        assert!(component_size(&Some(ComponentSize::new(Either::Right(-1.)))).is_err());
    }
}

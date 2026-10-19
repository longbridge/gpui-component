use gpui_kit::{
    AnyElement, App, AppContext, Entity, IntoElement, Subscription, Window,
    component::slider::{
        Slider, SliderEvent, SliderScale, SliderState, SliderValue as NativeValue,
    },
    prelude::FluentBuilder,
};
use nsh_core::ui::{Event, JsHost, JsIn, JsOwned, JsValue, PropEnum, PropError, Props, TsTypes};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};

/// One thumb's value, or two ordered range endpoints.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum SliderValue {
    Single(f32),
    Range([f32; 2]),
}
impl Default for SliderValue {
    fn default() -> Self {
        Self::Single(0.)
    }
}
impl SliderValue {
    fn native(self) -> NativeValue {
        match self {
            Self::Single(value) => NativeValue::Single(value),
            Self::Range([start, end]) => NativeValue::Range(start, end),
        }
    }
    fn from_native(value: &NativeValue) -> Self {
        match *value {
            NativeValue::Single(value) => Self::Single(value),
            NativeValue::Range(start, end) => Self::Range([start, end]),
        }
    }
}
impl JsValue for SliderValue {
    fn ts_type(types: &mut TsTypes) -> String {
        types.declare("SliderValue", |types| {
            format!(
                "export type SliderValue = {} | {};",
                f32::ts_type(types),
                <[f32; 2]>::ts_type(types)
            )
        })
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        if value.as_number().is_some() {
            let number = f32::from_js(value)?;
            if number.is_finite() {
                return Ok(Self::Single(number));
            }
        } else {
            let [start, end] = <[f32; 2]>::from_js(value)?;
            if start.is_finite() && end.is_finite() && start <= end {
                return Ok(Self::Range([start, end]));
            }
        }
        Err(value.expected("a finite number or ordered pair of finite numbers"))
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        match self {
            Self::Single(value) => value.to_js(host),
            Self::Range(range) => range.to_js(host),
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum SliderScaleValue {
    Linear,
    Logarithmic,
}
impl From<SliderScaleValue> for SliderScale {
    fn from(value: SliderScaleValue) -> Self {
        match value {
            SliderScaleValue::Linear => Self::Linear,
            SliderScaleValue::Logarithmic => Self::Logarithmic,
        }
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct SliderProps {
    value: SliderValue,
    #[prop(default = 0.)]
    min: f32,
    #[prop(default = 100.)]
    max: f32,
    #[prop(default = 1.)]
    step: f32,
    #[prop(default=SliderScaleValue::Linear)]
    scale: SliderScaleValue,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    vertical: bool,
    #[prop(default)]
    reverse: bool,
    #[prop(default)]
    accessibility_label: String,
}
#[derive(Event)]
pub(crate) enum SliderEvents {
    Change(SliderValue),
    Release(SliderValue),
    #[allow(
        dead_code,
        reason = "The typed host emits validation errors through this declared event."
    )]
    Error(String),
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeSlider>("gpui-component.Slider", "Slider", NativeSlider::new);
    registry.metadata().native_controlled(
        "gpui-component.Slider",
        &[("change", &["value"]), ("release", &["value"])],
    );
}
#[derive(Clone, Copy, PartialEq)]
struct Options {
    min: f32,
    max: f32,
    step: f32,
    scale: SliderScaleValue,
}
impl Options {
    fn from_props(props: &SliderProps) -> Self {
        Self {
            min: props.min,
            max: props.max,
            step: props.step,
            scale: props.scale,
        }
    }
    fn valid(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.step.is_finite()
            && self.min < self.max
            && self.step > 0.
            && (self.max - self.min).is_finite()
            && (self.scale != SliderScaleValue::Logarithmic
                || (self.min > 0. && (self.max / self.min).is_finite()))
    }
}
pub(super) struct NativeSlider {
    context: TypedComponentContext<SliderEvents>,
    state: Option<Entity<SliderState>>,
    events: Option<Subscription>,
    options: Options,
    options_dirty: bool,
    value: Option<SliderValue>,
}
impl NativeSlider {
    fn new(context: TypedComponentContext<SliderEvents>) -> Self {
        Self {
            context,
            state: None,
            events: None,
            options: Options::from_props(&SliderProps::initial()),
            options_dirty: false,
            value: None,
        }
    }
    fn sync(&mut self, window: &mut Window, cx: &mut App) -> Entity<SliderState> {
        let state = self
            .state
            .get_or_insert_with(|| {
                let state = cx.new(|_| SliderState::new());
                let context = self.context.clone();
                self.events = Some(window.subscribe(&state, cx, move |_, event, _, _| {
                    context.emit(match event {
                        SliderEvent::Change(value) => {
                            SliderEvents::Change(SliderValue::from_native(value))
                        }
                        SliderEvent::Release(value) => {
                            SliderEvents::Release(SliderValue::from_native(value))
                        }
                    });
                    context.redraw();
                }));
                state
            })
            .clone();
        if std::mem::take(&mut self.options_dirty) {
            let options = self.options;
            state.update(cx, |state, cx| {
                let value = state.value().clamp(options.min, options.max);
                *state = std::mem::replace(state, SliderState::new())
                    .scale(SliderScale::Linear)
                    .min(options.min)
                    .max(options.max)
                    .step(options.step)
                    .scale(options.scale.into())
                    .default_value(value);
                cx.notify();
            });
        }
        if let Some(value) = self.value.take() {
            let snapshot = state.read(cx);
            let value = value
                .native()
                .clamp(snapshot.min_value(), snapshot.max_value());
            if snapshot.value() != value {
                state.update(cx, |state, cx| state.set_value(value, window, cx));
            }
        }
        state
    }
}
impl TypedNativeComponent for NativeSlider {
    type Props = SliderProps;
    type Event = SliderEvents;
    fn validate_props(&self, props: &SliderProps) -> Result<(), PropError> {
        if !Options::from_props(props).valid() {
            return Err(PropError(
                "Slider requires finite min < max, step > 0, and min > 0 for logarithmic scale"
                    .into(),
            ));
        }
        Ok(())
    }
    fn render(
        &mut self,
        props: &SliderProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self.sync(window, cx);
        Some(
            Slider::new(&state)
                .accessibility_label(props.accessibility_label.clone())
                .disabled(props.disabled)
                .when(props.vertical, |slider| slider.vertical())
                .when(props.reverse, |slider| slider.reverse())
                .into_any_element(),
        )
    }
    fn props_changed(
        &mut self,
        props: &SliderProps,
        changed: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        let options = Options::from_props(props);
        if self.options != options {
            self.options = options;
            self.options_dirty = true
        }
        if changed.iter().any(|field| field == "value") {
            self.value = Some(props.value)
        }
    }
}

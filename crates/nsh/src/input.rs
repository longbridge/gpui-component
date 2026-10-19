use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Subscription, Window,
    component::{
        Disableable,
        input::{
            InlineToken, Input, InputContent as NativeContent,
            InputContentType as NativeContentType, InputEvent, InputGroup, InputGroupAddon,
            InputGroupAddonAlignment, InputState, InputToken, MaskPattern, NumberInput,
            NumberInputEvent, NumberStep, StepAction,
        },
    },
};
use nsh_core::ui::{Event, JsObject, PropEnum, PropError, Props, Slot};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;

/// An inline token occupying a UTF-8 byte range in the input text.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub(crate) struct InputTokenValue {
    id: String,
    text: String,
    label: Option<String>,
    start: usize,
    end: usize,
}
/// Text and its non-overlapping inline tokens.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub(crate) struct InputContent {
    text: String,
    #[prop(default)]
    tokens: Vec<InputTokenValue>,
}
impl InputContent {
    fn native(&self) -> Result<NativeContent, String> {
        self.tokens
            .iter()
            .try_fold(NativeContent::new(self.text.clone()), |content, span| {
                let token = InlineToken::new(span.id.clone(), span.text.clone())
                    .with_label(span.label.as_deref().unwrap_or(&span.text).to_owned());
                content
                    .with_token(span.start..span.end, token)
                    .map_err(|error| format!("invalid inline token: {error:?}"))
            })
    }
    fn from_native(content: &NativeContent) -> Self {
        Self {
            text: content.text().to_string(),
            tokens: content
                .tokens()
                .iter()
                .map(|span| InputTokenValue {
                    id: span.token().id().to_string(),
                    text: span.token().text().to_string(),
                    label: Some(span.token().label().to_string()),
                    start: span.range().start,
                    end: span.range().end,
                })
                .collect(),
        }
    }
}
macro_rules! content_types {
    ($($variant:ident),* $(,)?) => {
        /// The operating system's semantic text-input purpose.
        #[derive(Clone,Copy,Debug,PartialEq,PropEnum)]
        pub(crate) enum InputContentType { $($variant),* }
        impl From<InputContentType> for NativeContentType {
            fn from(value:InputContentType)->Self {
                match value { $(InputContentType::$variant=>Self::$variant),* }
            }
        }
    };
}
content_types! {
    Name,
    NamePrefix,
    GivenName,
    MiddleName,
    FamilyName,
    NameSuffix,
    Nickname,
    JobTitle,
    OrganizationName,
    Location,
    FullStreetAddress,
    StreetAddressLine1,
    StreetAddressLine2,
    AddressCity,
    AddressState,
    AddressCityAndState,
    Sublocality,
    CountryName,
    PostalCode,
    TelephoneNumber,
    EmailAddress,
    Url,
    CreditCardNumber,
    CreditCardName,
    CreditCardGivenName,
    CreditCardMiddleName,
    CreditCardFamilyName,
    CreditCardSecurityCode,
    CreditCardExpiration,
    CreditCardExpirationMonth,
    CreditCardExpirationYear,
    CreditCardType,
    Username,
    Password,
    NewPassword,
    OneTimeCode,
    ShipmentTrackingNumber,
    FlightNumber,
    DateTime,
    Birthdate,
    BirthdateDay,
    BirthdateMonth,
    BirthdateYear,
    CellularEid,
    CellularImei,
}
// Public Input and the existing Number/Group aliases share one field source.
// Specialized fields never leak into the generated Input API.
macro_rules! input_schema {
    ($( $(#[$attribute:meta])* $field:ident:$ty:ty, )*) => {
        /// A retained native text input. Supply either value or rich content.
        #[derive(Clone,PartialEq,Props)]
        pub(crate) struct InputProps { $( $(#[$attribute])* $field:$ty, )* }
        #[derive(Clone,PartialEq,Props)]
        struct NativeInputProps {
            $( $(#[$attribute])* $field:$ty, )*
            #[prop(default)] invalid:bool,
            min:Option<f64>,max:Option<f64>,step:Option<f64>,
            inline_start:Option<Slot>,inline_end:Option<Slot>,
            block_start:Option<Slot>,block_end:Option<Slot>,
        }
        impl From<&InputProps> for NativeInputProps {
            fn from(props:&InputProps)->Self{Self{$($field: ::core::clone::Clone::clone(&props.$field),)*..Self::initial()}}
        }
    };
}
input_schema! {
    #[prop(exclusive="content")] value:Option<String>,
    content:Option<InputContent>,
    #[prop(default)] placeholder:String,
    #[prop(default)] accessibility_label:String,
    content_type:Option<InputContentType>,
    #[prop(default)] disabled:bool,
    #[prop(default)] readonly:bool,
    #[prop(default)] cleanable:bool,
    #[prop(default)] mask_toggle:bool,
    #[prop(default)] masked:bool,
    mask_pattern:Option<String>,
    #[prop(default)] clean_on_escape:bool,
    #[prop(default=true)] submit_on_enter:bool,
    #[prop(default=true)] appearance:bool,
    #[prop(default=true)] bordered:bool,
    prefix:Option<Slot>,
    suffix:Option<Slot>,
}
#[derive(Clone, Event)]
pub(crate) enum NativeInputEvent {
    Change(String),
    ContentChange(InputContent),
    Focus,
    Blur,
    Submit {
        secondary: bool,
        shift: bool,
    },
    TokenClick {
        id: String,
        start: usize,
        end: usize,
    },
    TokenHover {
        id: String,
        start: usize,
        end: usize,
        hovered: bool,
    },
    Error(String),
}
#[derive(Event)]
enum NumberInputEvents {
    Change(String),
    Focus,
    Blur,
    Submit { secondary: bool, shift: bool },
    Step(InputStep),
    Error(String),
}
impl NumberInputEvents {
    fn from_input(event: NativeInputEvent) -> Option<Self> {
        Some(match event {
            NativeInputEvent::Change(value) => Self::Change(value),
            NativeInputEvent::Focus => Self::Focus,
            NativeInputEvent::Blur => Self::Blur,
            NativeInputEvent::Submit { secondary, shift } => Self::Submit { secondary, shift },
            NativeInputEvent::Error(error) => Self::Error(error),
            NativeInputEvent::ContentChange(_)
            | NativeInputEvent::TokenClick { .. }
            | NativeInputEvent::TokenHover { .. } => return None,
        })
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct NumberInputProps {
    value: String,
    #[prop(default)]
    placeholder: String,
    #[prop(default)]
    accessibility_label: String,
    #[prop(default)]
    disabled: bool,
    #[prop(default = true)]
    appearance: bool,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    prefix: Option<Slot>,
    suffix: Option<Slot>,
}
fn emit_input(
    context: &TypedComponentContext<NativeInputEvent>,
    number: bool,
    event: NativeInputEvent,
) {
    if number {
        if let Some(event) = NumberInputEvents::from_input(event) {
            TypedComponentContext::from(context.raw().clone()).emit(event)
        }
    } else {
        context.emit(event)
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum InputStep {
    Increment,
    Decrement,
}

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<InputControl>("gpui-component.Input", "Input", InputControl::new);
    registry.metadata().native_controlled(
        "gpui-component.Input",
        &[("change", &["value"]), ("contentChange", &["content"])],
    );
}

pub(crate) fn register_number(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<AliasInputControl>(
        "gpui-component.NumberInput",
        "NumberInput",
        |context| {
            let mut control = InputControl::new(TypedComponentContext::from(context.raw().clone()));
            control.number = true;
            AliasInputControl(control)
        },
    );
    registry
        .metadata()
        .native_controlled("gpui-component.NumberInput", &[("change", &["value"])]);
}
pub(crate) fn register_group_input(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<InputControl>(
        "gpui-component.InputGroupInput",
        "InputGroupInput",
        InputControl::new,
    );
    registry.metadata().native_controlled(
        "gpui-component.InputGroupInput",
        &[("change", &["value"]), ("contentChange", &["content"])],
    );
}
pub(crate) struct InputControl {
    context: TypedComponentContext<NativeInputEvent>,
    state: Option<Entity<InputState>>,
    focus: Option<FocusHandle>,
    events: Option<Subscription>,
    number_events: Option<Subscription>,
    props: NativeInputProps,
    content_dirty: bool,
    ownership_error: bool,
    number: bool,
    group: bool,
    bounds_dirty: bool,
}
impl InputControl {
    fn new(context: TypedComponentContext<NativeInputEvent>) -> Self {
        Self {
            context,
            state: None,
            focus: None,
            events: None,
            number_events: None,
            props: NativeInputProps::initial(),
            content_dirty: false,
            ownership_error: false,
            number: false,
            group: false,
            bounds_dirty: false,
        }
    }
    fn apply_content(&mut self, window: &mut Window, cx: &mut App) {
        if !self.content_dirty {
            return;
        }
        self.content_dirty = false;
        if self.props.value.is_some() && self.props.content.is_some() {
            if !self.ownership_error {
                self.context.emit(NativeInputEvent::Error(
                    "Input accepts either value or content, never both".into(),
                ));
            }
            self.ownership_error = true;
            return;
        }
        self.ownership_error = false;
        let state = self.state(window, cx);
        let content = match &self.props.content {
            Some(content) => match content.native() {
                Ok(content) => content,
                Err(error) => {
                    self.context.emit(NativeInputEvent::Error(error));
                    return;
                }
            },
            None => NativeContent::new(
                self.props
                    .value
                    .clone()
                    .unwrap_or_else(|| state.read(cx).value().to_string()),
            ),
        };
        // Accepted echoes never reset native selection, IME or undo history.
        if state.read(cx).content() != content {
            state.update(cx, |state, cx| state.set_value(content, window, cx));
        }
    }
    fn state(&mut self, window: &mut Window, cx: &mut App) -> Entity<InputState> {
        if let Some(state) = &self.state {
            return state.clone();
        }
        let state = cx.new(|cx| InputState::new(window, cx));
        self.focus = Some(state.focus_handle(cx));
        let context = self.context.clone();
        let number = self.number;
        self.events = Some(
            window.subscribe(&state, cx, move |state, event, _, cx| match event {
                InputEvent::Change => {
                    emit_input(
                        &context,
                        number,
                        NativeInputEvent::ContentChange(InputContent::from_native(
                            &state.read(cx).content(),
                        )),
                    );
                    emit_input(
                        &context,
                        number,
                        NativeInputEvent::Change(state.read(cx).value().to_string()),
                    );
                }
                InputEvent::Focus => emit_input(&context, number, NativeInputEvent::Focus),
                InputEvent::Blur => emit_input(&context, number, NativeInputEvent::Blur),
                InputEvent::PressEnter { secondary, shift } => emit_input(
                    &context,
                    number,
                    NativeInputEvent::Submit {
                        secondary: *secondary,
                        shift: *shift,
                    },
                ),
            }),
        );
        if self.number {
            let context =
                TypedComponentContext::<NumberInputEvents>::from(self.context.raw().clone());
            self.number_events = Some(window.subscribe(&state, cx, move |_, event, _, _| {
                let NumberInputEvent::Step(action) = event;
                context.emit(NumberInputEvents::Step(match action {
                    StepAction::Increment => InputStep::Increment,
                    StepAction::Decrement => InputStep::Decrement,
                }));
            }));
        }
        self.state = Some(state.clone());
        state
    }
}
impl InputControl {
    fn render_native(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        use gpui_kit::prelude::FluentBuilder;
        self.apply_content(window, cx);
        let state = self.state(window, cx);
        if self.bounds_dirty {
            self.bounds_dirty = false;
            if self.props.min.is_some_and(|value| !value.is_finite())
                || self.props.max.is_some_and(|value| !value.is_finite())
                || self
                    .props
                    .min
                    .zip(self.props.max)
                    .is_some_and(|(minimum, maximum)| minimum > maximum)
            {
                self.context
                    .emit(NativeInputEvent::Error("min must not exceed max".into()));
            } else {
                state.update(cx, |state, cx| {
                    state.set_min(self.props.min, window, cx);
                    state.set_max(self.props.max, window, cx);
                });
            }
        }
        if self.number {
            return Some(
                NumberInput::new(&state)
                    .placeholder(self.props.placeholder.clone())
                    .when_some(
                        crate::bridge::slot(&self.context, "prefix"),
                        |input, slot| input.prefix(slot),
                    )
                    .when_some(
                        crate::bridge::slot(&self.context, "suffix"),
                        |input, slot| input.suffix(slot),
                    )
                    .disabled(self.props.disabled)
                    .appearance(self.props.appearance)
                    .accessibility_label(self.props.accessibility_label.clone())
                    .into_any_element(),
            );
        }
        let token_context = self.context.clone();
        let click_context = self.context.clone();
        let hover_context = self.context.clone();
        let input = Input::new(&state)
            .token(move |token, _, _| {
                crate::bridge::slot(&token_context, &format!("token:{}", token.token().id()))
                    .unwrap_or_else(|| InputToken::new(token).into_any_element())
            })
            .on_token_click(move |event, _, _| {
                click_context.emit(NativeInputEvent::TokenClick {
                    id: event.token().id().to_string(),
                    start: event.range().start,
                    end: event.range().end,
                })
            })
            .on_token_hover(move |event, _, _| {
                hover_context.emit(NativeInputEvent::TokenHover {
                    id: event.token().id().to_string(),
                    start: event.range().start,
                    end: event.range().end,
                    hovered: event.is_hovered(),
                })
            })
            .when_some(
                self.props.content_type.map(Into::into),
                |input, content_type| input.content_type(content_type),
            )
            .when_some(
                crate::bridge::slot(&self.context, "prefix"),
                |input, slot| input.prefix(slot),
            )
            .when_some(
                crate::bridge::slot(&self.context, "suffix"),
                |input, slot| input.suffix(slot),
            )
            .cleanable(self.props.cleanable)
            .appearance(self.props.appearance)
            .bordered(self.props.bordered)
            .when(self.props.mask_toggle, |input| input.mask_toggle())
            .aria_label(self.props.accessibility_label.clone())
            .disabled(self.props.disabled)
            .readonly(self.props.readonly);
        if self.group {
            let group = InputGroup::new("input-group")
                .input(input)
                .disabled(self.props.disabled)
                .readonly(self.props.readonly)
                .invalid(self.props.invalid);
            Some(
                [
                    ("inlineStart", InputGroupAddonAlignment::InlineStart),
                    ("inlineEnd", InputGroupAddonAlignment::InlineEnd),
                    ("blockStart", InputGroupAddonAlignment::BlockStart),
                    ("blockEnd", InputGroupAddonAlignment::BlockEnd),
                ]
                .into_iter()
                .fold(group, |group, (name, alignment)| match crate::bridge::slot(
                    &self.context,
                    name,
                ) {
                    Some(slot) => {
                        group.addon(InputGroupAddon::new(name).align(alignment).child(slot))
                    }
                    None => group,
                })
                .into_any_element(),
            )
        } else {
            Some(input.into_any_element())
        }
    }

    fn focus_handle(&self) -> Option<FocusHandle> {
        // The handle is recorded when the state is created, before native
        // prepaint queries it. Reading an Entity here would require an App.
        self.focus.clone().filter(|_| !self.props.disabled)
    }

    fn update_props(&mut self, props: &NativeInputProps, window: &mut Window, cx: &mut App) {
        let previous = std::mem::replace(&mut self.props, props.clone());
        self.content_dirty = true;
        self.bounds_dirty |= previous.min != props.min || previous.max != props.max;
        let state = self.state(window, cx);
        state.update(cx, |state, cx| {
            if previous.placeholder != props.placeholder {
                state.set_placeholder(props.placeholder.clone(), window, cx);
            }
            if previous.masked != props.masked {
                state.set_masked(props.masked, window, cx);
            }
            if previous.mask_pattern != props.mask_pattern {
                state.set_mask_pattern(
                    props
                        .mask_pattern
                        .as_deref()
                        .map(MaskPattern::new)
                        .unwrap_or_default(),
                    window,
                    cx,
                );
            }
            if previous.clean_on_escape != props.clean_on_escape {
                state.set_clean_on_escape(props.clean_on_escape);
            }
            if previous.submit_on_enter != props.submit_on_enter {
                state.set_submit_on_enter(props.submit_on_enter, cx);
            }
            if previous.step != props.step {
                if props
                    .step
                    .is_some_and(|step| !step.is_finite() || step <= 0.)
                {
                    self.context.emit(NativeInputEvent::Error(
                        "step must be positive and finite".into(),
                    ));
                } else {
                    state.set_step(props.step.map(NumberStep::from), window, cx);
                }
            }
        });
    }
    fn command_native(&mut self, name: &str, args: &Value, window: &mut Window, cx: &mut App) {
        self.apply_content(window, cx);
        if !self.props.disabled {
            if name == "insert" && !self.props.readonly {
                if let Some(text) = args.as_str() {
                    self.state(window, cx)
                        .update(cx, |state, cx| state.insert(text, window, cx));
                }
            }
            if name == "selectAll" {
                self.state(window, cx)
                    .update(cx, |state, cx| state.select_all(window, cx));
            }
        }
        if name == "focus" && !self.props.disabled {
            let state = self.state(window, cx);
            let focus = state.focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
    }
}

impl TypedNativeComponent for InputControl {
    type Props = InputProps;
    type Event = NativeInputEvent;
    fn validate_props(&self, props: &InputProps) -> Result<(), PropError> {
        if let Some(content) = &props.content {
            content.native().map_err(PropError)?;
        }
        Ok(())
    }
    fn render(
        &mut self,
        _props: &InputProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.render_native(window, cx)
    }
    fn props_changed(
        &mut self,
        props: &InputProps,
        _changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        self.update_props(&NativeInputProps::from(props), window, cx)
    }
    fn command(
        &mut self,
        _props: &InputProps,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.command_native(name, args, window, cx)
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        InputControl::focus_handle(self)
    }
}
struct AliasInputControl(InputControl);
impl TypedNativeComponent for AliasInputControl {
    type Props = NumberInputProps;
    type Event = NumberInputEvents;
    fn validate_props(&self, props: &NumberInputProps) -> Result<(), PropError> {
        if props.min.is_some_and(|value| !value.is_finite())
            || props.max.is_some_and(|value| !value.is_finite())
            || props.min.zip(props.max).is_some_and(|(min, max)| min > max)
            || props
                .step
                .is_some_and(|value| !value.is_finite() || value <= 0.)
        {
            return Err(PropError(
                "numeric options must be finite, min <= max, and step positive".into(),
            ));
        }
        Ok(())
    }
    fn render(
        &mut self,
        _props: &NumberInputProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.0.render_native(window, cx)
    }
    fn props_changed(
        &mut self,
        props: &NumberInputProps,
        _changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0.update_props(
            &NativeInputProps {
                value: Some(props.value.clone()),
                placeholder: props.placeholder.clone(),
                accessibility_label: props.accessibility_label.clone(),
                disabled: props.disabled,
                appearance: props.appearance,
                min: props.min,
                max: props.max,
                step: props.step,
                prefix: props.prefix,
                suffix: props.suffix,
                ..NativeInputProps::initial()
            },
            window,
            cx,
        )
    }
    fn command(
        &mut self,
        _props: &NumberInputProps,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0.command_native(name, args, window, cx)
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.0.focus_handle()
    }
}

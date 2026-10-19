//! Retained native editing controls driven by NSH's typed schemas.
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, SharedString,
    Subscription, Window,
    component::{
        Disableable,
        input::{
            Editor, EditorState, Input, InputEvent, InputState, OtpEvent, OtpInput, OtpState,
            Textarea, TextareaState,
        },
    },
    prelude::FluentBuilder,
    px,
};
use nsh_core::ui::{Event, JsObject, PropEnum, PropError, Props, Slot};
use nsh_platform_gpui::{ComponentContext, TypedComponentContext, TypedNativeComponent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cell::RefCell, rc::Rc};
#[path = "input_parts.rs"]
pub(super) mod parts;
#[path = "input_services.rs"]
mod services;

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    parts::register_typed(registry);
    registry.register::<NativeTextarea>("gpui-component.Textarea", "Textarea", NativeTextarea::new);
    registry.register::<NativeTextarea>(
        "gpui-component.InputGroupTextarea",
        "InputGroupTextarea",
        NativeTextarea::new,
    );
    registry.register::<NativeEditor>("gpui-component.Editor", "Editor", NativeEditor::new);
    registry.register::<NativeOtp>("gpui-component.OtpInput", "OtpInput", NativeOtp::new);
    crate::input::register_number(registry);
    crate::input::register_group_input(registry);
    for name in [
        "gpui-component.Textarea",
        "gpui-component.InputGroupTextarea",
        "gpui-component.Editor",
        "gpui-component.OtpInput",
    ] {
        registry
            .metadata()
            .native_controlled(name, &[("change", &["value"])]);
    }
}
macro_rules! editing_schema {
    ($( $(#[$attr:meta])* $field:ident:$ty:ty, )*)=>{
        #[derive(Clone,PartialEq,Props)] pub(crate) struct TextareaProps {
            $($(#[$attr])* $field:$ty,)*
            #[prop(default=3)] rows:usize,auto_grow:Option<[usize;2]>,
        }
        #[derive(Clone,PartialEq,Props)] pub(crate) struct EditorProps {
            $($(#[$attr])* $field:$ty,)*
            #[prop(default)] language:String,
            #[prop(default=true)] line_numbers:bool,#[prop(default=true)] folding:bool,
            #[prop(default=true)] auto_close:bool,#[prop(default=true)] smart_indent:bool,
            #[prop(default)] diagnostics:Vec<EditorDiagnostic>,
            language_services:Option<services::EditorLanguageServices>,
        }
    };
}
editing_schema! {
    value:String,
    #[prop(default)] placeholder:String,
    #[prop(default)] accessibility_label:String,
    #[prop(default)] disabled:bool,#[prop(default)] readonly:bool,
    #[prop(default=true)] appearance:bool,#[prop(default=true)] bordered:bool,
    #[prop(default=160.)] height:f32,
    #[prop(default=true)] soft_wrap:bool,#[prop(default=true)] searchable:bool,
    #[prop(default)] show_whitespaces:bool,#[prop(default)] submit_on_enter:bool,
    #[prop(default=true)] context_menu:bool,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct InputGroupProps {
    value: String,
    #[prop(default)]
    placeholder: String,
    #[prop(default)]
    accessibility_label: String,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    readonly: bool,
    #[prop(default)]
    invalid: bool,
    #[prop(default=parts::InputGroupMode::Input)]
    mode: parts::InputGroupMode,
    #[prop(default = 160.)]
    height: f32,
    #[prop(default = 3)]
    rows: usize,
    auto_grow: Option<[usize; 2]>,
    #[prop(default = true)]
    soft_wrap: bool,
    inline_start: Option<Slot>,
    inline_end: Option<Slot>,
    block_start: Option<Slot>,
    block_end: Option<Slot>,
}
impl InputGroupProps {
    fn editing(&self) -> TextareaProps {
        TextareaProps {
            value: self.value.clone(),
            placeholder: self.placeholder.clone(),
            accessibility_label: self.accessibility_label.clone(),
            disabled: self.disabled,
            readonly: self.readonly,
            height: self.height,
            rows: self.rows,
            auto_grow: self.auto_grow,
            soft_wrap: self.soft_wrap,
            ..TextareaProps::initial()
        }
    }
}
#[derive(Clone, PartialEq, JsObject, Deserialize, Serialize)]
pub(crate) struct EditorPosition {
    line: u32,
    character: u32,
}
#[derive(Clone, PartialEq, JsObject, Deserialize, Serialize)]
pub(crate) struct EditorRange {
    start: EditorPosition,
    end: EditorPosition,
}
#[derive(Clone, Copy, PartialEq, PropEnum, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EditorDiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct EditorDiagnostic {
    start: EditorPosition,
    end: EditorPosition,
    message: String,
    #[prop(default=EditorDiagnosticSeverity::Info)]
    severity: EditorDiagnosticSeverity,
}
macro_rules! editing_events {($($event:tt)*)=>{
    #[derive(Event)] pub(crate) enum EditingEvents {$($event)*}
    #[derive(Event)] pub(crate) enum EditorEvents {$($event)* Request(Box<services::EditorRequest>),ShowDocument(services::EditorShowDocument),}
};}
editing_events! {Change(String),Focus,Blur,Submit{secondary:bool,shift:bool},#[allow(dead_code,reason="The typed host emits validation errors through this declared event.")]Error(String),}
macro_rules! editing_control {
    ($native:ident,$props:ident,$events:ident,$state:ident,$element:ident,$extra:ident,$validate:ident,$present:ident,$multiline:literal) => {
        pub(super) struct $native {
            context: TypedComponentContext<$events>,
            state: Option<Entity<$state>>,
            focus: Option<FocusHandle>,
            events: Option<Subscription>,
            requests: Rc<RefCell<services::Requests>>,
            props: $props,
            height: f32,
            group: bool,
            invalid: bool,
        }
        impl $native {
            pub(super) fn new(context: TypedComponentContext<$events>) -> Self {
                Self {
                    context,
                    state: None,
                    focus: None,
                    events: None,
                    requests: Rc::default(),
                    props: $props::initial(),
                    height: 160.,
                    group: false,
                    invalid: false,
                }
            }
            fn state(&mut self, window: &mut Window, cx: &mut App) -> Entity<$state> {
                if let Some(state) = &self.state {
                    return state.clone();
                }
                let state = cx.new(|cx| $state::new(window, cx));
                self.focus = Some(state.focus_handle(cx));
                let context = self.context.clone();
                self.events = Some(window.subscribe(&state, cx, move |state, event, _, cx| {
                    context.emit(match event {
                        InputEvent::Change => $events::Change(state.read(cx).value().to_string()),
                        InputEvent::Focus => $events::Focus,
                        InputEvent::Blur => $events::Blur,
                        InputEvent::PressEnter { secondary, shift } => $events::Submit {
                            secondary: *secondary,
                            shift: *shift,
                        },
                    })
                }));
                self.state = Some(state.clone());
                state
            }
        }
        impl TypedNativeComponent for $native {
            type Props = $props;
            type Event = $events;
            fn validate_props(&self, props: &$props) -> Result<(), PropError> {
                $validate(props)
            }
            fn render(
                &mut self,
                props: &$props,
                window: &mut Window,
                cx: &mut App,
            ) -> Option<AnyElement> {
                let state = self.state(window, cx);
                Some($present(
                    $element::new(&state)
                        .disabled(props.disabled)
                        .readonly(props.readonly)
                        .appearance(props.appearance)
                        .bordered(props.bordered)
                        .aria_label(props.accessibility_label.clone())
                        .when($multiline, |control| control.h(px(self.height))),
                    self.group,
                    self.invalid,
                    props.disabled,
                    props.readonly,
                    props.accessibility_label.clone().into(),
                    self.context.raw(),
                ))
            }
            fn focus_handle(&self) -> Option<FocusHandle> {
                self.focus.clone().filter(|_| !self.props.disabled)
            }
            fn props_changed(
                &mut self,
                props: &$props,
                _: &[String],
                window: &mut Window,
                cx: &mut App,
            ) {
                let old = std::mem::replace(&mut self.props, props.clone());
                self.height = props.height;
                let entity = self.state(window, cx);
                entity.update(cx, |state, cx| {
                    if state.value().as_ref() != props.value {
                        state.set_value(props.value.clone(), window, cx);
                    }
                    if old.placeholder != props.placeholder {
                        state.set_placeholder(props.placeholder.clone(), window, cx);
                    }
                    if old.show_whitespaces != props.show_whitespaces {
                        state.set_show_whitespaces(props.show_whitespaces, window, cx);
                    }
                    if old.submit_on_enter != props.submit_on_enter {
                        state.set_submit_on_enter(props.submit_on_enter, cx);
                    }
                    if old.context_menu != props.context_menu {
                        state.set_context_menu_enabled(props.context_menu);
                    }
                    $extra(
                        state,
                        &old,
                        props,
                        &self.context,
                        &self.requests,
                        window,
                        cx,
                    );
                });
            }
            fn command(
                &mut self,
                props: &$props,
                name: &str,
                args: &Value,
                window: &mut Window,
                cx: &mut App,
            ) {
                if name == "resolveRequest" {
                    self.requests.borrow_mut().resolve(args);
                    return;
                }
                if name == "focus" && !props.disabled {
                    let focus = self.state(window, cx).focus_handle(cx);
                    window.defer(cx, move |window, cx| window.focus(&focus, cx));
                    return;
                }
                if props.disabled {
                    return;
                }
                self.state(window, cx).update(cx, |state, cx| match name {
                    "insert" => {
                        if !props.readonly {
                            if let Some(text) = args.as_str() {
                                state.insert(text, window, cx);
                            }
                        }
                    }
                    "selectAll" => state.select_all(window, cx),
                    "select" => {
                        if let (Some(start), Some(end)) = (
                            args.get("start").and_then(Value::as_u64),
                            args.get("end").and_then(Value::as_u64),
                        ) {
                            let text = state.value();
                            let start = (start as usize).min(text.len());
                            let end = (end as usize).min(text.len());
                            if start <= end
                                && text.is_char_boundary(start)
                                && text.is_char_boundary(end)
                            {
                                state.set_selected_range(start..end, cx);
                            }
                        }
                    }
                    _ => {}
                });
            }
        }
    };
}
editing_control!(
    NativeTextarea,
    TextareaProps,
    EditingEvents,
    TextareaState,
    Textarea,
    textarea_props,
    validate_textarea,
    present_textarea,
    true
);
editing_control!(
    NativeEditor,
    EditorProps,
    EditorEvents,
    EditorState,
    Editor,
    editor_props,
    validate_editor,
    present_editor,
    true
);
editing_control!(
    GroupTextarea,
    TextareaProps,
    EditingEvents,
    TextareaState,
    Textarea,
    textarea_props,
    validate_textarea,
    present_textarea,
    true
);
editing_control!(
    GroupInput,
    TextareaProps,
    EditingEvents,
    InputState,
    Input,
    group_input_props,
    validate_textarea,
    present_input,
    false
);
fn validate_size(height: f32, auto_grow: Option<[usize; 2]>) -> Result<(), PropError> {
    if !height.is_finite() || height <= 0. {
        return Err(PropError("height must be positive and finite".into()));
    }
    if auto_grow.is_some_and(|[min, max]| min == 0 || max < min || max > 10000) {
        return Err(PropError(
            "autoGrow must have 1 <= min <= max <= 10000".into(),
        ));
    }
    Ok(())
}
fn validate_textarea(props: &TextareaProps) -> Result<(), PropError> {
    validate_size(props.height, props.auto_grow)
}
fn validate_editor(props: &EditorProps) -> Result<(), PropError> {
    validate_size(props.height, None)
}
pub(super) fn validate_group(props: &InputGroupProps) -> Result<(), PropError> {
    validate_size(props.height, props.auto_grow)
}
fn textarea_rows(
    state: &mut TextareaState,
    rows: usize,
    auto_grow: Option<[usize; 2]>,
    cx: &mut gpui_kit::Context<TextareaState>,
) {
    state.set_rows(rows.clamp(1, 10000), cx);
    if let Some([min, max]) = auto_grow {
        if min > 0 && max >= min && max <= 10000 {
            state.set_auto_grow(min, max, cx);
        }
    }
}
fn textarea_props(
    state: &mut TextareaState,
    old: &TextareaProps,
    props: &TextareaProps,
    _: &TypedComponentContext<EditingEvents>,
    _: &Rc<RefCell<services::Requests>>,
    window: &mut Window,
    cx: &mut gpui_kit::Context<TextareaState>,
) {
    if old.soft_wrap != props.soft_wrap {
        state.set_soft_wrap(props.soft_wrap, window, cx);
    }
    if old.searchable != props.searchable {
        state.set_searchable(props.searchable, cx);
    }
    if old.rows != props.rows || old.auto_grow != props.auto_grow {
        textarea_rows(state, props.rows, props.auto_grow, cx)
    }
}
fn group_input_props(
    _: &mut InputState,
    _: &TextareaProps,
    _: &TextareaProps,
    _: &TypedComponentContext<EditingEvents>,
    _: &Rc<RefCell<services::Requests>>,
    _: &mut Window,
    _: &mut gpui_kit::Context<InputState>,
) {
}
fn editor_props(
    state: &mut EditorState,
    old: &EditorProps,
    props: &EditorProps,
    context: &TypedComponentContext<EditorEvents>,
    requests: &Rc<RefCell<services::Requests>>,
    window: &mut Window,
    cx: &mut gpui_kit::Context<EditorState>,
) {
    if old.soft_wrap != props.soft_wrap {
        state.set_soft_wrap(props.soft_wrap, window, cx);
    }
    if old.searchable != props.searchable {
        state.set_searchable(props.searchable, cx);
    }
    if old.line_numbers != props.line_numbers {
        state.set_line_number(props.line_numbers, window, cx)
    }
    if old.folding != props.folding {
        state.set_folding(props.folding, window, cx)
    }
    if old.auto_close != props.auto_close {
        state.set_auto_close(props.auto_close, window, cx)
    }
    if old.smart_indent != props.smart_indent {
        state.set_smart_indent(props.smart_indent, window, cx)
    }
    if old.language_services != props.language_services {
        services::configure(
            state,
            props.language_services.as_ref(),
            context,
            requests,
            cx,
        );
    }
    if old.language != props.language && state.language_name().as_ref() != props.language {
        state.set_highlighter(props.language.clone(), cx);
    }
    if old.diagnostics != props.diagnostics {
        use gpui_kit::base::input::{Diagnostic, DiagnosticSeverity};
        use gpui_kit::component::input::Position;
        let diagnostics = props
            .diagnostics
            .iter()
            .filter_map(|entry| {
                let start = Position::new(entry.start.line, entry.start.character);
                let end = Position::new(entry.end.line, entry.end.character);
                (start <= end).then(|| {
                    Diagnostic::new(start..end, entry.message.clone()).with_severity(
                        match entry.severity {
                            EditorDiagnosticSeverity::Error => DiagnosticSeverity::Error,
                            EditorDiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
                            EditorDiagnosticSeverity::Info => DiagnosticSeverity::Info,
                            EditorDiagnosticSeverity::Hint => DiagnosticSeverity::Hint,
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        if let Some(set) = state.diagnostics_mut() {
            set.clear();
            set.extend(diagnostics);
        }
        cx.notify();
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct OtpInputProps {
    value: String,
    #[prop(default = 6)]
    length: usize,
    #[prop(default = 2)]
    groups: usize,
    #[prop(default)]
    masked: bool,
    #[prop(default)]
    disabled: bool,
}
#[derive(Event)]
pub(crate) enum OtpInputEvents {
    Change(String),
    Complete(String),
    Focus,
    Blur,
    #[allow(
        dead_code,
        reason = "The typed host emits validation errors through this declared event."
    )]
    Error(String),
}
struct NativeOtp {
    context: TypedComponentContext<OtpInputEvents>,
    state: Option<Entity<OtpState>>,
    focus: Option<FocusHandle>,
    events: Option<Subscription>,
    props: OtpInputProps,
}
impl NativeOtp {
    fn new(context: TypedComponentContext<OtpInputEvents>) -> Self {
        Self {
            context,
            state: None,
            focus: None,
            events: None,
            props: OtpInputProps::initial(),
        }
    }
    fn state(&mut self, window: &mut Window, cx: &mut App) -> Entity<OtpState> {
        if let Some(state) = &self.state {
            return state.clone();
        }
        let props = &self.props;
        let state = cx.new(|cx| {
            OtpState::new(props.length, window, cx)
                .masked(props.masked)
                .default_value(props.value.clone())
        });
        self.focus = Some(state.focus_handle(cx));
        let context = self.context.clone();
        self.events = Some(window.subscribe(&state, cx, move |state, event, _, cx| {
            context.emit(match event {
                OtpEvent::Change => OtpInputEvents::Change(state.read(cx).value().to_string()),
                OtpEvent::Complete => OtpInputEvents::Complete(state.read(cx).value().to_string()),
                OtpEvent::Focus => OtpInputEvents::Focus,
                OtpEvent::Blur => OtpInputEvents::Blur,
            })
        }));
        self.state = Some(state.clone());
        state
    }
}
impl TypedNativeComponent for NativeOtp {
    type Props = OtpInputProps;
    type Event = OtpInputEvents;
    fn validate_props(&self, props: &OtpInputProps) -> Result<(), PropError> {
        if !(1..=64).contains(&props.length) {
            return Err(PropError("length must be between 1 and 64".into()));
        }
        if !props.value.bytes().all(|c| c.is_ascii_digit()) || props.value.len() > props.length {
            return Err(PropError(
                "OTP value must contain at most length ASCII digits".into(),
            ));
        }
        Ok(())
    }
    fn render(
        &mut self,
        _: &OtpInputProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self.state(window, cx);
        Some(
            OtpInput::new(&state)
                .groups(self.props.groups)
                .disabled(self.props.disabled)
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| !self.props.disabled)
    }
    fn props_changed(
        &mut self,
        props: &OtpInputProps,
        _: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        let old = std::mem::replace(&mut self.props, props.clone());
        self.props.groups = self.props.groups.clamp(1, 64);
        if old.length != props.length {
            self.state = None;
            self.focus = None;
            self.events = None
        }
        let state = self.state(window, cx);
        state.update(cx, |state, cx| {
            if old.masked != props.masked {
                state.set_masked(props.masked, window, cx)
            }
            if state.value().as_ref() != props.value {
                state.set_value(props.value.clone(), window, cx)
            }
        });
    }
    fn command(
        &mut self,
        props: &OtpInputProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" && !props.disabled {
            let focus = self.state(window, cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
    }
}
fn present_textarea(
    input: Textarea,
    group: bool,
    invalid: bool,
    disabled: bool,
    readonly: bool,
    _label: SharedString,
    context: &ComponentContext,
) -> AnyElement {
    use gpui_kit::{
        ParentElement,
        component::input::{InputGroup, InputGroupAddon, InputGroupAddonAlignment},
    };
    if !group {
        return input.into_any_element();
    }
    let group = InputGroup::new("input-group")
        .input(input)
        .disabled(disabled)
        .readonly(readonly)
        .invalid(invalid);
    [
        ("inlineStart", InputGroupAddonAlignment::InlineStart),
        ("inlineEnd", InputGroupAddonAlignment::InlineEnd),
        ("blockStart", InputGroupAddonAlignment::BlockStart),
        ("blockEnd", InputGroupAddonAlignment::BlockEnd),
    ]
    .into_iter()
    .fold(
        group,
        |group, (name, alignment)| match crate::bridge::slot(context, name) {
            Some(slot) => group.addon(InputGroupAddon::new(name).align(alignment).child(slot)),
            None => group,
        },
    )
    .into_any_element()
}
fn present_editor(
    input: Editor,
    _: bool,
    _: bool,
    _: bool,
    _: bool,
    _: SharedString,
    _: &ComponentContext,
) -> AnyElement {
    input.into_any_element()
}
fn present_input(
    input: Input,
    group: bool,
    invalid: bool,
    disabled: bool,
    readonly: bool,
    _: SharedString,
    context: &ComponentContext,
) -> AnyElement {
    use gpui_kit::{
        ParentElement,
        component::input::{InputGroup, InputGroupAddon, InputGroupAddonAlignment},
    };
    if !group {
        return input.into_any_element();
    }
    let group = InputGroup::new("input-group")
        .input(input)
        .invalid(invalid)
        .disabled(disabled)
        .readonly(readonly);
    [
        ("inlineStart", InputGroupAddonAlignment::InlineStart),
        ("inlineEnd", InputGroupAddonAlignment::InlineEnd),
        ("blockStart", InputGroupAddonAlignment::BlockStart),
        ("blockEnd", InputGroupAddonAlignment::BlockEnd),
    ]
    .into_iter()
    .fold(
        group,
        |group, (name, alignment)| match crate::bridge::slot(context, name) {
            Some(slot) => group.addon(InputGroupAddon::new(name).align(alignment).child(slot)),
            None => group,
        },
    )
    .into_any_element()
}

//! Real input-group parts and explicit retained Input/Textarea composition.
use super::{EditingEvents, GroupInput, GroupTextarea, InputGroupProps};
use gpui_kit::{
    AnyElement, App, FocusHandle, IntoElement, ParentElement, Window,
    component::{
        Disableable,
        input::{InputGroupAddon, InputGroupAddonAlignment, InputGroupButton, InputGroupText},
    },
};
use nsh_core::ui::{Children, Event, NoEvent, PropEnum, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum InputGroupMode {
    Input,
    Textarea,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum InputGroupAlignment {
    InlineStart,
    InlineEnd,
    BlockStart,
    BlockEnd,
}
impl From<InputGroupAlignment> for InputGroupAddonAlignment {
    fn from(value: InputGroupAlignment) -> Self {
        match value {
            InputGroupAlignment::InlineStart => Self::InlineStart,
            InputGroupAlignment::InlineEnd => Self::InlineEnd,
            InputGroupAlignment::BlockStart => Self::BlockStart,
            InputGroupAlignment::BlockEnd => Self::BlockEnd,
        }
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct InputGroupAddonProps {
    #[prop(default=InputGroupAlignment::InlineStart)]
    alignment: InputGroupAlignment,
    children: Children,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct InputGroupTextProps {
    #[prop(default)]
    label: String,
    children: Children,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct InputGroupButtonProps {
    label: String,
    #[prop(default)]
    disabled: bool,
}
#[derive(Event)]
enum PartEvents {
    Press,
}
struct Addon {
    context: TypedComponentContext<NoEvent>,
}
struct Text {
    context: TypedComponentContext<NoEvent>,
}
struct Button {
    context: TypedComponentContext<PartEvents>,
}
pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeGroup>("gpui-component.InputGroup", "InputGroup", NativeGroup::new);
    registry
        .metadata()
        .native_controlled("gpui-component.InputGroup", &[("change", &["value"])]);
    registry.register::<Addon>(
        "gpui-component.InputGroupAddon",
        "InputGroupAddon",
        |context| Addon { context },
    );
    registry.register::<Text>(
        "gpui-component.InputGroupText",
        "InputGroupText",
        |context| Text { context },
    );
    registry.register::<Button>(
        "gpui-component.InputGroupButton",
        "InputGroupButton",
        |context| Button { context },
    );
}
impl TypedNativeComponent for Addon {
    type Props = InputGroupAddonProps;
    type Event = NoEvent;
    fn render(&mut self, props: &Self::Props, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            InputGroupAddon::new("addon")
                .align(props.alignment.into())
                .children(crate::bridge::children(&self.context))
                .into_any_element(),
        )
    }
}
impl TypedNativeComponent for Text {
    type Props = InputGroupTextProps;
    type Event = NoEvent;
    fn render(&mut self, props: &Self::Props, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            InputGroupText::new()
                .child(props.label.clone())
                .children(crate::bridge::children(&self.context))
                .into_any_element(),
        )
    }
}
impl TypedNativeComponent for Button {
    type Props = InputGroupButtonProps;
    type Event = PartEvents;
    fn render(&mut self, props: &Self::Props, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let context = self.context.clone();
        Some(
            InputGroupButton::new("button")
                .label(props.label.clone())
                .accessibility_label(props.label.clone())
                .disabled(props.disabled)
                .on_click(move |_, _, _| context.emit(PartEvents::Press))
                .into_any_element(),
        )
    }
}
enum Control {
    Input(GroupInput),
    Textarea(GroupTextarea),
}
struct NativeGroup {
    context: TypedComponentContext<EditingEvents>,
    control: Option<Control>,
    mode: InputGroupMode,
}
impl NativeGroup {
    fn new(context: TypedComponentContext<EditingEvents>) -> Self {
        Self {
            context,
            control: None,
            mode: InputGroupMode::Input,
        }
    }
    fn update(
        &mut self,
        props: &InputGroupProps,
        changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        let switched = self.control.is_none() || self.mode != props.mode;
        let focused = switched
            && self
                .focus_handle()
                .is_some_and(|focus| focus.is_focused(window));
        if switched {
            self.mode = props.mode;
            self.control = Some(match props.mode {
                InputGroupMode::Input => {
                    let mut control = GroupInput::new(self.context.clone());
                    control.group = true;
                    Control::Input(control)
                }
                InputGroupMode::Textarea => {
                    let mut control = GroupTextarea::new(self.context.clone());
                    control.group = true;
                    Control::Textarea(control)
                }
            })
        }
        match self.control.as_mut().unwrap() {
            Control::Input(control) => {
                control.invalid = props.invalid;
                control.props_changed(&props.editing(), changed, window, cx);
                if focused {
                    control.command(&props.editing(), "focus", &Value::Null, window, cx)
                }
            }
            Control::Textarea(control) => {
                control.invalid = props.invalid;
                control.props_changed(&props.editing(), changed, window, cx);
                if focused {
                    control.command(&props.editing(), "focus", &Value::Null, window, cx)
                }
            }
        }
    }
}
impl TypedNativeComponent for NativeGroup {
    type Props = InputGroupProps;
    type Event = EditingEvents;
    fn validate_props(&self, props: &InputGroupProps) -> Result<(), nsh_core::ui::PropError> {
        super::validate_group(props)
    }
    fn render(
        &mut self,
        props: &InputGroupProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        if self.control.is_none() {
            self.update(props, &[], window, cx)
        }
        match self.control.as_mut().unwrap() {
            Control::Input(control) => control.render(&props.editing(), window, cx),
            Control::Textarea(control) => control.render(&props.editing(), window, cx),
        }
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        match self.control.as_ref()? {
            Control::Input(control) => control.focus_handle(),
            Control::Textarea(control) => control.focus_handle(),
        }
    }
    fn props_changed(
        &mut self,
        props: &InputGroupProps,
        changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        self.update(props, changed, window, cx)
    }
    fn command(
        &mut self,
        props: &InputGroupProps,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.control.is_none() {
            self.update(props, &[], window, cx)
        }
        match self.control.as_mut().unwrap() {
            Control::Input(control) => control.command(&props.editing(), name, args, window, cx),
            Control::Textarea(control) => control.command(&props.editing(), name, args, window, cx),
        }
    }
}

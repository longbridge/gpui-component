use std::sync::Arc;

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError,
    TypeScriptDescriptor, anyhow,
    gpui::{self, Anchor, IntoElement as _, ParentElement as _, div},
    gpui_component::{
        button::{Button, ButtonVariants as _},
        popover::Popover,
    },
};

#[derive(Clone)]
struct PopoverPayload {
    id: String,
    label: String,
}

#[derive(Clone)]
enum PopoverOp {
    Anchor(Anchor),
    DefaultOpen(bool),
    Open(bool),
    Appearance(bool),
    OverlayClosable(bool),
    OnOpenChange(ComponentArgument),
}

struct PopoverMaterializer;

impl ComponentMaterializer for PopoverMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<PopoverPayload>()
            .ok_or_else(|| anyhow::anyhow!("Popover received an incompatible payload"))?
            .clone();
        let content = request
            .take_slot_factory("content")
            .ok_or_else(|| anyhow::anyhow!("Popover requires content(element)"))?;

        let mut popover = Popover::new(payload.id.clone()).trigger(
            Button::new(format!("popover-trigger:{}", payload.id))
                .ghost()
                .label(payload.label),
        );
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<PopoverOp>().cloned())
        {
            popover = match operation {
                PopoverOp::Anchor(anchor) => popover.anchor(anchor),
                PopoverOp::DefaultOpen(open) => popover.default_open(open),
                PopoverOp::Open(open) => popover.open(open),
                PopoverOp::Appearance(appearance) => popover.appearance(appearance),
                PopoverOp::OverlayClosable(closable) => popover.overlay_closable(closable),
                PopoverOp::OnOpenChange(argument) => {
                    let callback = request.resolve_callback(&argument)?;
                    popover.on_open_change(move |open, window, cx| {
                        callback.invoke_and_report_with(
                            "Popover.onOpenChange callback failed",
                            &[ComponentCallbackArgument::Boolean(*open)],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        popover = popover.content(move |_, window, cx| match content.build(window, cx) {
            Ok(element) => element,
            Err(error) => div()
                .child(format!("Failed to render Popover content: {error:#}"))
                .into_any_element(),
        });
        request.finish(popover)
    }
}

fn boolean_op(
    component_method: &'static str,
    arguments: &[ComponentArgument],
    wrap: impl FnOnce(bool) -> PopoverOp,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(wrap(*value))),
        _ => Err(format!(
            "Popover.{component_method}(value) expects a boolean"
        )),
    }
}

fn anchor_op(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    let [ComponentArgument::Enum(anchor)] = arguments else {
        return Err("Popover.anchor(anchor) expects an anchor literal".into());
    };
    let anchor = match anchor.as_str() {
        "topLeft" => Anchor::TopLeft,
        "topCenter" => Anchor::TopCenter,
        "topRight" => Anchor::TopRight,
        "bottomLeft" => Anchor::BottomLeft,
        "bottomCenter" => Anchor::BottomCenter,
        "bottomRight" => Anchor::BottomRight,
        "leftCenter" => Anchor::LeftCenter,
        "rightCenter" => Anchor::RightCenter,
        _ => return Err(format!("unsupported Popover anchor `{anchor}`")),
    };
    Ok(ComponentPayload::new(PopoverOp::Anchor(anchor)))
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    let boolean = |name, wrap: fn(bool) -> PopoverOp| {
        MethodDescriptor::new(
            name,
            vec![ArgumentDescriptor::new("value", ArgumentSchema::Boolean)],
            move |arguments| boolean_op(name, arguments, wrap),
        )
    };
    registry.register(ComponentDescriptor {
        name: "Popover",
        constructors: vec![ConstructorDescriptor::new(
            "Popover",
            vec![
                ArgumentDescriptor::new("id", ArgumentSchema::String),
                ArgumentDescriptor::new("label", ArgumentSchema::String),
            ],
            |arguments| match arguments {
                [
                    ComponentArgument::String(id),
                    ComponentArgument::String(label),
                ] if !id.trim().is_empty() && !label.trim().is_empty() => {
                    Ok(ComponentPayload::new(PopoverPayload {
                        id: id.clone(),
                        label: label.clone(),
                    }))
                }
                [ComponentArgument::String(_), ComponentArgument::String(_)] => {
                    Err("Popover id and label must not be empty".into())
                }
                _ => Err("Popover(id, label) expects two strings".into()),
            },
        )],
        methods: vec![
            MethodDescriptor::new(
                "anchor",
                vec![ArgumentDescriptor::new(
                    "anchor",
                    ArgumentSchema::Enum(&[
                        "topLeft",
                        "topCenter",
                        "topRight",
                        "bottomLeft",
                        "bottomCenter",
                        "bottomRight",
                        "leftCenter",
                        "rightCenter",
                    ]),
                )],
                anchor_op,
            )
            .documented("Positions the popover relative to its trigger."),
            boolean("defaultOpen", PopoverOp::DefaultOpen)
                .documented("Sets the initial uncontrolled open state."),
            boolean("open", PopoverOp::Open).documented("Controls whether the popover is open."),
            boolean("appearance", PopoverOp::Appearance)
                .documented("Controls the native popover surface styling."),
            boolean("overlayClosable", PopoverOp::OverlayClosable)
                .documented("Controls whether pressing outside dismisses the popover."),
            MethodDescriptor::new(
                "onOpenChange",
                vec![ArgumentDescriptor::new(
                    "callback",
                    ArgumentSchema::Callback("(open: boolean, cx: Context) => void"),
                )],
                |arguments| match arguments {
                    [argument @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(
                        PopoverOp::OnOpenChange(argument.clone()),
                    )),
                    _ => Err("Popover.onOpenChange(callback) expects a callback".into()),
                },
            )
            .documented("Runs when pointer interaction changes the open state."),
        ],
        typescript: TypeScriptDescriptor::new(
            "A button-triggered popover with lazy content(element).",
        ),
        materializer: Arc::new(PopoverMaterializer),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_schema_includes_the_script_context() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptor = frozen.descriptors().next().unwrap();
        assert_eq!(
            descriptor.methods[5].arguments[0].schema,
            ArgumentSchema::Callback("(open: boolean, cx: Context) => void")
        );
    }
}

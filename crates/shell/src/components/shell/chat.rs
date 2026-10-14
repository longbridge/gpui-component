use std::{sync::Arc, time::Duration};

use gpui_component::{
    Sizable as _, Size,
    attachment::{Attachment, AttachmentContent, AttachmentStatus},
    bubble::{Bubble, BubbleVariant},
    marker::{Marker, MarkerLoadingStyle, MarkerVariant},
    message::{Message, MessageAlignment, MessageContent},
    message_scroller::{MessageScroller, MessageScrollerState},
    shimmer::ShimmerText,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentElementCallback, ComponentMaterializer, ComponentPayload,
    ComponentRegistry, ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError,
    StateDescriptor, anyhow,
    gpui::{
        self, App, AppContext as _, Axis, Entity, IntoElement as _, ParentElement as _,
        Refineable as _, RenderOnce, Styled as _, Window,
    },
};

#[derive(Clone)]
struct Payload {
    kind: Kind,
}

#[derive(Clone)]
enum Kind {
    Attachment {
        id: String,
    },
    Bubble,
    Marker {
        id: String,
    },
    Message,
    ShimmerText {
        text: String,
    },
    MessageScroller {
        id: String,
        state: ComponentArgument,
        render_item: ComponentArgument,
    },
}

#[derive(Clone)]
enum Op {
    Status(AttachmentStatus),
    Axis(Axis),
    Size(Size),
    Alignment(MessageAlignment),
    BubbleVariant(BubbleVariant),
    MarkerVariant(MarkerVariant),
    Loading(bool),
    LoadingStyle(MarkerLoadingStyle),
    Id(String),
    Duration(Duration),
    Spread(f32),
    Reverse(bool),
    Once(bool),
    Scrollbar(bool),
    JumpButton(bool),
    JumpButtonLabel(String),
}

struct Materializer;

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("chat component received an incompatible payload"))?
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
            .cloned()
            .collect::<Vec<_>>();

        match payload.kind {
            Kind::Attachment { id } => {
                let mut attachment = Attachment::new().id(id);
                for operation in &operations {
                    attachment = match operation {
                        Op::Status(value) => attachment.status(*value),
                        Op::Axis(value) => attachment.axis(*value),
                        Op::Size(value) => attachment.with_size(*value),
                        _ => attachment,
                    };
                }
                let children = request.take_children()?;
                if !children.is_empty() {
                    attachment = attachment.content(AttachmentContent::new().children(children));
                }
                attachment.style().refine(&request.take_style());
                Ok(attachment.into_any_element())
            }
            Kind::Bubble => {
                let mut bubble = Bubble::new();
                for operation in &operations {
                    bubble = match operation {
                        Op::Alignment(value) => bubble.alignment(*value),
                        Op::BubbleVariant(value) => bubble.with_variant(*value),
                        _ => bubble,
                    };
                }
                request.finish(bubble)
            }
            Kind::Marker { id } => {
                let mut marker = Marker::new().id(id);
                for operation in &operations {
                    marker = match operation {
                        Op::MarkerVariant(value) => marker.with_variant(*value),
                        Op::Loading(value) => marker.loading(*value),
                        Op::LoadingStyle(value) => marker.with_loading_style(*value),
                        _ => marker,
                    };
                }
                request.finish(marker)
            }
            Kind::Message => {
                let mut message = Message::new();
                for operation in &operations {
                    if let Op::Alignment(value) = operation {
                        message = message.alignment(*value);
                    }
                }
                let children = request.take_children()?;
                if !children.is_empty() {
                    message = message.content(MessageContent::new().children(children));
                }
                message.style().refine(&request.take_style());
                Ok(message.into_any_element())
            }
            Kind::ShimmerText { text } => {
                anyhow::ensure!(
                    request.children_len() == 0,
                    "ShimmerText does not accept children"
                );
                let mut shimmer = ShimmerText::new(text);
                for operation in &operations {
                    shimmer = match operation {
                        Op::Id(value) => shimmer.id(value.clone()),
                        Op::Duration(value) => shimmer.duration(*value),
                        Op::Spread(value) => shimmer.spread(*value),
                        Op::Reverse(value) => shimmer.reverse(*value),
                        Op::Once(value) => shimmer.once(*value),
                        _ => shimmer,
                    };
                }
                shimmer.style().refine(&request.take_style());
                Ok(shimmer.into_any_element())
            }
            Kind::MessageScroller {
                id,
                state,
                render_item,
            } => {
                anyhow::ensure!(
                    request.children_len() == 0,
                    "MessageScroller does not accept children"
                );
                let state =
                    request.with_state::<Entity<MessageScrollerState>, _>(&state, Clone::clone)?;
                let renderer = request.resolve_element_callback(&render_item)?;
                Ok(BoundMessageScroller {
                    id,
                    state,
                    renderer,
                    operations,
                    style: request.take_style(),
                }
                .into_any_element())
            }
        }
    }
}

#[derive(gpui::IntoElement)]
struct BoundMessageScroller {
    id: String,
    state: Entity<MessageScrollerState>,
    renderer: ComponentElementCallback,
    operations: Vec<Op>,
    style: gpui::StyleRefinement,
}

impl RenderOnce for BoundMessageScroller {
    fn render(self, _: &mut Window, _: &mut App) -> impl gpui::IntoElement {
        let renderer = self.renderer;
        let mut scroller =
            MessageScroller::new(self.id, self.state, move |index, window, cx| match renderer
                .build_with(
                    &[ComponentCallbackArgument::Number(index as f64)],
                    window,
                    cx,
                ) {
                Ok(Some(element)) => element,
                Ok(None) => gpui::div().into_any_element(),
                Err(error) => gpui::div()
                    .child(format!("Failed to render message row: {error:#}"))
                    .into_any_element(),
            });
        for operation in self.operations {
            scroller = match operation {
                Op::Scrollbar(value) => scroller.scrollbar(value),
                Op::JumpButton(value) => scroller.jump_button(value),
                Op::JumpButtonLabel(value) => scroller.with_jump_button_label(value),
                _ => scroller,
            };
        }
        scroller.style().refine(&self.style);
        scroller.into_any_element()
    }
}

fn enum_method(
    component: &'static str,
    name: &'static str,
    values: &'static [&'static str],
    documentation: &'static str,
    parse: fn(&str) -> Option<Op>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Enum(values))],
        move |arguments| match arguments {
            [ComponentArgument::Enum(value)] => parse(value)
                .map(ComponentPayload::new)
                .ok_or_else(|| format!("unsupported {component}.{name} value `{value}`")),
            _ => Err(format!("{component}.{name} expects one enum literal")),
        },
    )
    .with_documentation(documentation)
}

fn bool_method(
    component: &'static str,
    name: &'static str,
    documentation: &'static str,
    wrap: fn(bool) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |arguments| match arguments {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(wrap(*value))),
            _ => Err(format!("{component}.{name} expects one boolean")),
        },
    )
    .with_documentation(documentation)
}

fn no_arg(name: &'static str, kind: Kind) -> ConstructorDescriptor {
    ConstructorDescriptor::new(name, vec![], move |_| {
        Ok(ComponentPayload::new(Payload { kind: kind.clone() }))
    })
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Attachment", Arc::new(Materializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Attachment",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] if !id.trim().is_empty() => Ok(ComponentPayload::new(Payload { kind: Kind::Attachment { id: id.clone() } })),
                    _ => Err("Attachment expects a non-empty stable id".into()),
                },
            )])
            .with_methods(vec![
                enum_method("Attachment", "status", &["pending", "uploading", "processing", "failed", "complete"], "Sets the attachment lifecycle status.", |value| Some(Op::Status(match value { "pending" => AttachmentStatus::Pending, "uploading" => AttachmentStatus::Uploading, "processing" => AttachmentStatus::Processing, "failed" => AttachmentStatus::Failed, "complete" => AttachmentStatus::Complete, _ => return None }))),
                enum_method("Attachment", "axis", &["horizontal", "vertical"], "Sets the attachment layout axis.", |value| Some(Op::Axis(match value { "horizontal" => Axis::Horizontal, "vertical" => Axis::Vertical, _ => return None }))),
                enum_method("Attachment", "size", &["xsmall", "small", "medium", "large"], "Sets the semantic attachment size.", |value| Some(Op::Size(match value { "xsmall" => Size::XSmall, "small" => Size::Small, "medium" => Size::Medium, "large" => Size::Large, _ => return None }))),
            ])
            .with_documentation("A file or image attachment. Ordinary children are composed into its metadata content slot."),
    )?;
    registry.register(
        ComponentDescriptor::new("Bubble", Arc::new(Materializer))
            .with_constructors(vec![no_arg("Bubble", Kind::Bubble)])
            .with_methods(vec![
                enum_method(
                    "Bubble",
                    "alignment",
                    &["start", "end"],
                    "Sets the message-edge alignment.",
                    |value| {
                        Some(Op::Alignment(match value {
                            "start" => MessageAlignment::Start,
                            "end" => MessageAlignment::End,
                            _ => return None,
                        }))
                    },
                ),
                enum_method(
                    "Bubble",
                    "variant",
                    &[
                        "filled",
                        "secondary",
                        "muted",
                        "tinted",
                        "outline",
                        "ghost",
                        "destructive",
                    ],
                    "Sets the semantic bubble treatment.",
                    |value| {
                        Some(Op::BubbleVariant(match value {
                            "filled" => BubbleVariant::Filled,
                            "secondary" => BubbleVariant::Secondary,
                            "muted" => BubbleVariant::Muted,
                            "tinted" => BubbleVariant::Tinted,
                            "outline" => BubbleVariant::Outline,
                            "ghost" => BubbleVariant::Ghost,
                            "destructive" => BubbleVariant::Destructive,
                            _ => return None,
                        }))
                    },
                ),
            ])
            .with_documentation(
                "A message bubble whose ordinary children form its visible content.",
            ),
    )?;
    registry.register(
        ComponentDescriptor::new("Marker", Arc::new(Materializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Marker",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                        Ok(ComponentPayload::new(Payload {
                            kind: Kind::Marker { id: id.clone() },
                        }))
                    }
                    _ => Err("Marker expects a non-empty stable id".into()),
                },
            )])
            .with_methods(vec![
                enum_method(
                    "Marker",
                    "variant",
                    &["plain", "separator", "border"],
                    "Sets the marker treatment.",
                    |value| {
                        Some(Op::MarkerVariant(match value {
                            "plain" => MarkerVariant::Plain,
                            "separator" => MarkerVariant::Separator,
                            "border" => MarkerVariant::Border,
                            _ => return None,
                        }))
                    },
                ),
                bool_method(
                    "Marker",
                    "loading",
                    "Sets whether the marker displays a loading treatment.",
                    Op::Loading,
                ),
                enum_method(
                    "Marker",
                    "loading_style",
                    &["spinner", "shimmer"],
                    "Sets the loading treatment.",
                    |value| {
                        Some(Op::LoadingStyle(match value {
                            "spinner" => MarkerLoadingStyle::Spinner,
                            "shimmer" => MarkerLoadingStyle::Shimmer,
                            _ => return None,
                        }))
                    },
                ),
            ])
            .with_documentation("A compact conversation status marker with composable children."),
    )?;
    registry.register(
        ComponentDescriptor::new("Message", Arc::new(Materializer))
            .with_constructors(vec![no_arg("Message", Kind::Message)])
            .with_methods(vec![enum_method(
                "Message",
                "alignment",
                &["start", "end"],
                "Sets the sender-edge alignment.",
                |value| {
                    Some(Op::Alignment(match value {
                        "start" => MessageAlignment::Start,
                        "end" => MessageAlignment::End,
                        _ => return None,
                    }))
                },
            )])
            .with_documentation(
                "A message row. Ordinary children are composed into its content slot.",
            ),
    )?;
    registry.register(
        ComponentDescriptor::new("ShimmerText", Arc::new(Materializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "ShimmerText",
                vec![ArgumentDescriptor::new("text", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(text)] => Ok(ComponentPayload::new(Payload {
                        kind: Kind::ShimmerText { text: text.clone() },
                    })),
                    _ => Err("ShimmerText expects text".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "id",
                    vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                            Ok(ComponentPayload::new(Op::Id(id.clone())))
                        }
                        _ => Err("ShimmerText.id expects a non-empty stable id".into()),
                    },
                )
                .with_documentation("Sets an explicit stable animation identity."),
                MethodDescriptor::new(
                    "duration_ms",
                    vec![ArgumentDescriptor::new(
                        "duration_ms",
                        ArgumentSchema::Number,
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Number(value)]
                            if value.is_finite() && *value >= 0.0 && *value <= u64::MAX as f64 =>
                        {
                            Ok(ComponentPayload::new(Op::Duration(Duration::from_millis(
                                *value as u64,
                            ))))
                        }
                        _ => Err(
                            "ShimmerText.duration_ms expects a finite non-negative duration".into(),
                        ),
                    },
                )
                .with_documentation("Sets one shimmer sweep duration in milliseconds."),
                MethodDescriptor::new(
                    "spread",
                    vec![ArgumentDescriptor::new("spread", ArgumentSchema::Number)],
                    |arguments| match arguments {
                        [ComponentArgument::Number(value)]
                            if value.is_finite()
                                && *value >= f32::MIN as f64
                                && *value <= f32::MAX as f64 =>
                        {
                            Ok(ComponentPayload::new(Op::Spread(*value as f32)))
                        }
                        _ => Err("ShimmerText.spread expects a finite f32 fraction".into()),
                    },
                )
                .with_documentation("Sets the relative highlight half-width."),
                bool_method(
                    "ShimmerText",
                    "reverse",
                    "Reverses the shimmer direction.",
                    Op::Reverse,
                ),
                bool_method(
                    "ShimmerText",
                    "once",
                    "Runs one sweep instead of looping.",
                    Op::Once,
                ),
            ])
            .with_documentation("Theme-aware animated loading text."),
    )?;
    registry.register_state(
        StateDescriptor::new(
            "MessageScrollerState",
            "MessageScrollerState",
            vec![ArgumentDescriptor::new(
                "item_count",
                ArgumentSchema::Number,
            )],
            |arguments, _, cx| match arguments {
                [ComponentArgument::Number(value)]
                    if value.is_finite()
                        && value.fract() == 0.0
                        && *value >= 0.0
                        && *value <= usize::MAX as f64 =>
                {
                    Ok(Box::new(
                        cx.new(|cx| MessageScrollerState::new(*value as usize, cx)),
                    ))
                }
                _ => Err("MessageScrollerState expects a non-negative integer item_count".into()),
            },
        )
        .with_documentation(
            "Retained virtual-list and tail-following state for a message transcript.",
        ),
    )?;
    registry.register(
        ComponentDescriptor::new("MessageScroller", Arc::new(Materializer))
            .with_constructors(vec![ConstructorDescriptor::new("MessageScroller", vec![ArgumentDescriptor::new("id", ArgumentSchema::String), ArgumentDescriptor::new("state", ArgumentSchema::Entity("MessageScrollerState")), ArgumentDescriptor::new("render_item", ArgumentSchema::Callback("(index: number) => Element | null"))], |arguments| match arguments { [ComponentArgument::String(id), state @ ComponentArgument::Entity { .. }, render_item @ ComponentArgument::Callback(_)] if !id.trim().is_empty() => Ok(ComponentPayload::new(Payload { kind: Kind::MessageScroller { id: id.clone(), state: state.clone(), render_item: render_item.clone() } })), _ => Err("MessageScroller expects a non-empty id, MessageScrollerState, and row renderer".into()) })])
            .with_methods(vec![
                bool_method("MessageScroller", "scrollbar", "Enables its virtual-list scrollbar.", Op::Scrollbar),
                bool_method("MessageScroller", "jump_button", "Enables the jump-to-latest button.", Op::JumpButton),
                MethodDescriptor::new("jump_button_label", vec![ArgumentDescriptor::new("jump_button_label", ArgumentSchema::String)], |arguments| match arguments { [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(Op::JumpButtonLabel(value.clone()))), _ => Err("MessageScroller.jump_button_label expects non-empty text".into()) }).with_documentation("Sets the jump-to-latest button label."),
            ])
            .with_documentation("A virtualized message transcript with retained scroll and tail-following state."),
    )?;
    Ok(())
}

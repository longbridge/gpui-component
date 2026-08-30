pub(super) use crate::shell::support::{bool_method, string_method};

use gpui_component::Size;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentPayload, MethodDescriptor,
};

#[derive(Clone)]
pub(super) enum CommonOp {
    Size(Size),
    Label(String),
    Tooltip(String),
    Checked(bool),
    Outline,
}

pub(super) fn size_method() -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        |arguments| match arguments {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "xsmall" => Ok(ComponentPayload::new(CommonOp::Size(Size::XSmall))),
                "small" => Ok(ComponentPayload::new(CommonOp::Size(Size::Small))),
                "medium" => Ok(ComponentPayload::new(CommonOp::Size(Size::Medium))),
                "large" => Ok(ComponentPayload::new(CommonOp::Size(Size::Large))),
                _ => Err(format!("unsupported size `{value}`")),
            },
            _ => Err("size expects a semantic size literal".into()),
        },
    )
    .with_documentation("Sets the semantic control size.")
}

pub(super) fn outline_method() -> MethodDescriptor {
    MethodDescriptor::new("outline", Vec::new(), |_| {
        Ok(ComponentPayload::new(CommonOp::Outline))
    })
    .with_documentation("Uses the component's outline presentation.")
}

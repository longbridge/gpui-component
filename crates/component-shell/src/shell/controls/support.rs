use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentPayload, MethodDescriptor,
    gpui_component::Size,
};

#[derive(Clone)]
pub(super) enum CommonOp {
    Size(Size),
    Label(String),
    Tooltip(String),
    Checked(bool),
    Outline,
}

pub(super) fn string_method(
    name: &'static str,
    documentation: &'static str,
    make: fn(String) -> CommonOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] => Ok(ComponentPayload::new(make(value.to_owned()))),
            _ => Err(format!("{name} expects one string")),
        },
    )
    .documented(documentation)
}

pub(super) fn bool_method(
    name: &'static str,
    documentation: &'static str,
    make: fn(bool) -> CommonOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |arguments| match arguments {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
            _ => Err(format!("{name} expects one boolean")),
        },
    )
    .documented(documentation)
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
    .documented("Sets the semantic control size.")
}

pub(super) fn outline_method() -> MethodDescriptor {
    MethodDescriptor::new("outline", Vec::new(), |_| {
        Ok(ComponentPayload::new(CommonOp::Outline))
    })
    .documented("Uses the component's outline presentation.")
}

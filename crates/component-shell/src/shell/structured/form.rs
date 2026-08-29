use super::common::{
    TypedChildElement, nonnegative_f32, positive_u16, require_child, take_element,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _, px},
    gpui_component::{
        Sizable as _, Size,
        form::{Field, h_form, v_form},
    },
};
use std::sync::Arc;

#[derive(Clone, Copy)]
struct FieldPayload;
#[derive(Clone)]
enum FieldOp {
    Label(String),
    Description(String),
    Required(bool),
    Visible(bool),
    LabelIndent(bool),
    Align(Align),
    ColSpan(u16),
}
#[derive(Clone, Copy)]
enum Align {
    Start,
    Center,
    End,
}
struct FieldMaterializer;
impl ComponentMaterializer for FieldMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<FieldPayload>()
            .ok_or_else(|| anyhow::anyhow!("Field received an incompatible payload"))?;
        let mut field = Field::new();
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FieldOp>())
        {
            field = match op {
                FieldOp::Label(v) => field.label(v.clone()),
                FieldOp::Description(v) => field.description(v.clone()),
                FieldOp::Required(v) => field.required(*v),
                FieldOp::Visible(v) => field.visible(*v),
                FieldOp::LabelIndent(v) => field.label_indent(*v),
                FieldOp::Align(Align::Start) => field.items_start(),
                FieldOp::Align(Align::Center) => field.items_center(),
                FieldOp::Align(Align::End) => field.items_end(),
                FieldOp::ColSpan(v) => field.col_span(*v),
            };
        }
        field.style().refine(&request.take_style());
        field.extend(request.take_children()?);
        Ok(TypedChildElement::new(field).into_any_element())
    }
}

#[derive(Clone, Copy)]
enum FormPayload {
    Vertical,
    Horizontal,
}
#[derive(Clone, Copy)]
enum FormOp {
    Columns(usize),
    LabelWidth(f32),
    Size(Size),
}
struct FormMaterializer;
impl ComponentMaterializer for FormMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<FormPayload>()
            .ok_or_else(|| anyhow::anyhow!("Form received an incompatible payload"))?;
        let mut form = match payload {
            FormPayload::Vertical => v_form(),
            FormPayload::Horizontal => h_form(),
        };
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            form = match op {
                FormOp::Columns(v) => form.columns(*v),
                FormOp::LabelWidth(v) => form.label_width(px(*v)),
                FormOp::Size(v) => form.with_size(*v),
            };
        }
        for mut child in request.take_typed_children() {
            require_child("Form", "Field", child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            form = form.child(take_element::<Field>(&mut element, "Field")?);
        }
        form.style().refine(&request.take_style());
        Ok(form.into_any_element())
    }
}

fn bool_method(name: &'static str, make: fn(bool) -> FieldOp) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |a| match a {
            [ComponentArgument::Boolean(v)] => Ok(ComponentPayload::new(make(*v))),
            _ => Err(format!("Field.{name}({name}) expects a boolean")),
        },
    )
}
fn form_columns(a: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match a {
        [ComponentArgument::Number(value)] => positive_u16(*value, "Form.columns")
            .map(|value| ComponentPayload::new(FormOp::Columns(usize::from(value)))),
        _ => Err("Form.columns expects an exactly representable positive integer".into()),
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor {
        name: "Field",
        constructors: vec![ConstructorDescriptor::new("Field", vec![], |_| {
            Ok(ComponentPayload::new(FieldPayload))
        })],
        methods: vec![
            MethodDescriptor::new(
                "label",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |a| match a {
                    [ComponentArgument::String(v)] => {
                        Ok(ComponentPayload::new(FieldOp::Label(v.clone())))
                    }
                    _ => Err("Field.label(label) expects a string".into()),
                },
            )
            .documented("Sets the field label."),
            MethodDescriptor::new(
                "description",
                vec![ArgumentDescriptor::new(
                    "description",
                    ArgumentSchema::String,
                )],
                |a| match a {
                    [ComponentArgument::String(v)] => {
                        Ok(ComponentPayload::new(FieldOp::Description(v.clone())))
                    }
                    _ => Err("Field.description(description) expects a string".into()),
                },
            )
            .documented("Sets supporting text below the control."),
            bool_method("required", FieldOp::Required).documented("Marks the field as required."),
            bool_method("visible", FieldOp::Visible).documented("Controls field visibility."),
            bool_method("labelIndent", FieldOp::LabelIndent)
                .documented("Keeps unlabeled horizontal fields aligned with labeled fields."),
            MethodDescriptor::new(
                "align",
                vec![ArgumentDescriptor::new(
                    "align",
                    ArgumentSchema::Enum(&["start", "center", "end"]),
                )],
                |a| match a {
                    [ComponentArgument::Enum(v)] => match v.as_str() {
                        "start" => Ok(ComponentPayload::new(FieldOp::Align(Align::Start))),
                        "center" => Ok(ComponentPayload::new(FieldOp::Align(Align::Center))),
                        "end" => Ok(ComponentPayload::new(FieldOp::Align(Align::End))),
                        _ => Err(format!("unsupported Field alignment `{v}`")),
                    },
                    _ => Err("Field.align(align) expects an alignment literal".into()),
                },
            )
            .documented("Aligns the label and control within the field."),
            MethodDescriptor::new(
                "colSpan",
                vec![ArgumentDescriptor::new("span", ArgumentSchema::Number)],
                |a| match a {
                    [ComponentArgument::Number(v)] => positive_u16(*v, "Field.colSpan")
                        .map(|v| ComponentPayload::new(FieldOp::ColSpan(v))),
                    _ => Err(
                        "Field.colSpan expects an exactly representable positive integer".into(),
                    ),
                },
            )
            .documented("Sets the field's grid-column span."),
        ],
        typescript: TypeScriptDescriptor::new(
            "A typed form field containing ordinary control children.",
        ),
        materializer: Arc::new(FieldMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "Form",
        constructors: vec![
            ConstructorDescriptor::new("Form", vec![], |_| {
                Ok(ComponentPayload::new(FormPayload::Vertical))
            }),
            ConstructorDescriptor::new("VForm", vec![], |_| {
                Ok(ComponentPayload::new(FormPayload::Vertical))
            }),
            ConstructorDescriptor::new("HForm", vec![], |_| {
                Ok(ComponentPayload::new(FormPayload::Horizontal))
            }),
        ],
        methods: vec![
            MethodDescriptor::new(
                "columns",
                vec![ArgumentDescriptor::new("columns", ArgumentSchema::Number)],
                form_columns,
            )
            .documented("Sets the form grid's column count."),
            MethodDescriptor::new(
                "labelWidth",
                vec![ArgumentDescriptor::new("width", ArgumentSchema::Number)],
                |a| match a {
                    [ComponentArgument::Number(v)] => nonnegative_f32(*v, "Form.labelWidth")
                        .map(|v| ComponentPayload::new(FormOp::LabelWidth(v))),
                    _ => Err("Form.labelWidth(width) expects a nonnegative finite number".into()),
                },
            )
            .documented("Sets the horizontal form label width in pixels."),
            MethodDescriptor::new(
                "size",
                vec![ArgumentDescriptor::new(
                    "size",
                    ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                )],
                |a| match a {
                    [ComponentArgument::Enum(v)] => match v.as_str() {
                        "xsmall" => Ok(ComponentPayload::new(FormOp::Size(Size::XSmall))),
                        "small" => Ok(ComponentPayload::new(FormOp::Size(Size::Small))),
                        "medium" => Ok(ComponentPayload::new(FormOp::Size(Size::Medium))),
                        "large" => Ok(ComponentPayload::new(FormOp::Size(Size::Large))),
                        _ => Err(format!("unsupported Form size `{v}`")),
                    },
                    _ => Err("Form.size(size) expects a size literal".into()),
                },
            )
            .documented("Sets the form density."),
        ],
        typescript: TypeScriptDescriptor::new(
            "A vertical or horizontal form accepting Field children.",
        ),
        materializer: Arc::new(FormMaterializer),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn columns_accept_only_positive_integers() {
        assert!(form_columns(&[ComponentArgument::Number(2.)]).is_ok());
        assert!(form_columns(&[ComponentArgument::Number(u16::MAX as f64 + 1.)]).is_err());
        assert!(form_columns(&[ComponentArgument::Number(-1.)]).is_err());
        assert!(form_columns(&[ComponentArgument::Number(1.5)]).is_err());
        assert!(form_columns(&[ComponentArgument::Number(usize::MAX as f64)]).is_err());
    }

    #[test]
    fn field_span_and_label_width_reject_lossy_ranges() {
        assert!(positive_u16(u16::MAX as f64 + 1.0, "Field.colSpan").is_err());
        assert!(nonnegative_f32((f32::MAX as f64) * 2.0, "Form.labelWidth").is_err());
    }
}

use gpui_component::{Disableable as _, Sizable as _, Size, pagination::Pagination};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;

use super::common::{nonempty_id, nonnegative_usize};
#[derive(Clone)]
struct PaginationPayload(String);
#[derive(Clone)]
enum PaginationOp {
    Current(usize),
    Total(usize),
    Visible(usize),
    Compact,
    Size(Size),
}
struct PaginationMaterializer;
impl ComponentMaterializer for PaginationMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        anyhow::ensure!(
            request.children_len() == 0,
            "Pagination does not accept children"
        );
        let id = &request
            .payload()
            .downcast_ref::<PaginationPayload>()
            .ok_or_else(|| anyhow::anyhow!("Pagination received an incompatible payload"))?
            .0;
        let mut p = Pagination::new(id.clone()).disabled(request.disabled());
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<PaginationOp>())
        {
            p = match op {
                PaginationOp::Current(value) => p.current_page(*value),
                PaginationOp::Total(value) => p.total_pages(*value),
                PaginationOp::Visible(value) => p.visible_pages(*value),
                PaginationOp::Compact => p.compact(),
                PaginationOp::Size(value) => p.with_size(*value),
            }
        }
        let mut wrapper = gpui::div().child(p);
        wrapper.style().refine(&request.take_style());
        Ok(wrapper.into_any_element())
    }
}
fn positive(a: &ComponentArgument, label: &str) -> Result<usize, String> {
    match a {
        ComponentArgument::Number(value) => {
            nonnegative_usize(*value, &format!("Pagination.{label}({label})")).and_then(|value| {
                if value == 0 {
                    Err(format!(
                        "Pagination.{label}({label}) expects a positive integer"
                    ))
                } else {
                    Ok(value)
                }
            })
        }
        _ => Err(format!(
            "Pagination.{label}({label}) expects a positive integer"
        )),
    }
}
pub(super) fn register(r: &mut ComponentRegistry) -> Result<(), RegistryError> {
    let numeric = |name: &'static str, doc: &'static str, wrap: fn(usize) -> PaginationOp| {
        MethodDescriptor::new(
            name,
            vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
            move |arguments| match arguments {
                [value] => positive(value, name).map(|value| ComponentPayload::new(wrap(value))),
                _ => Err(format!("Pagination.{name}({name}) expects one argument")),
            },
        )
        .with_documentation(doc)
    };
    r.register(
        ComponentDescriptor::new("Pagination", Arc::new(PaginationMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Pagination",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => nonempty_id(id, "Pagination")
                        .map(PaginationPayload)
                        .map(ComponentPayload::new),
                    _ => Err("Pagination(id) expects a string id".into()),
                },
            )])
            .with_methods(vec![
                numeric(
                    "currentPage",
                    "Sets the current 1-based page.",
                    PaginationOp::Current,
                ),
                numeric(
                    "totalPages",
                    "Sets the positive page count.",
                    PaginationOp::Total,
                ),
                numeric(
                    "visiblePages",
                    "Sets the maximum visible page buttons.",
                    PaginationOp::Visible,
                ),
                MethodDescriptor::new("compact", vec![], |_| {
                    Ok(ComponentPayload::new(PaginationOp::Compact))
                })
                .with_documentation("Shows only previous and next icon buttons."),
                MethodDescriptor::new(
                    "size",
                    vec![ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(value)] => match value.as_str() {
                            "xsmall" => Ok(ComponentPayload::new(PaginationOp::Size(Size::XSmall))),
                            "small" => Ok(ComponentPayload::new(PaginationOp::Size(Size::Small))),
                            "medium" => Ok(ComponentPayload::new(PaginationOp::Size(Size::Medium))),
                            "large" => Ok(ComponentPayload::new(PaginationOp::Size(Size::Large))),
                            _ => Err(format!("unsupported Pagination size `{value}`")),
                        },
                        _ => Err("Pagination.size(size) expects a size literal".into()),
                    },
                )
                .with_documentation("Sets semantic size."),
            ])
            .with_documentation(
                "Controlled page navigation; disabled common behavior is supported.",
            ),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_integer_validation() {
        assert_eq!(
            positive(&ComponentArgument::Number(3.), "totalPages").unwrap(),
            3
        );
        assert!(positive(&ComponentArgument::Number(0.), "totalPages").is_err());
        assert!(positive(&ComponentArgument::Number(1.5), "totalPages").is_err());
        assert!(positive(&ComponentArgument::Number(usize::MAX as f64), "totalPages").is_err());
        assert!(positive(&ComponentArgument::Number(f64::INFINITY), "totalPages").is_err());
    }
    #[test]
    fn id_rejects_empty_and_whitespace_only_values() {
        assert!(nonempty_id("", "Pagination").is_err());
        assert!(nonempty_id("\n", "Pagination").is_err());
    }
}

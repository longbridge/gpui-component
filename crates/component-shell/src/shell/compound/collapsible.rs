use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow, gpui,
    gpui_component::collapsible::Collapsible,
};
use std::sync::Arc;
#[derive(Clone, Copy)]
struct CollapsiblePayload;
#[derive(Clone)]
enum CollapsibleOp {
    Open(bool),
    MotionId(String),
}
struct CollapsibleMaterializer;
impl ComponentMaterializer for CollapsibleMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<CollapsiblePayload>()
            .ok_or_else(|| anyhow::anyhow!("Collapsible received an incompatible payload"))?;
        let mut component = Collapsible::new();
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<CollapsibleOp>())
        {
            component = match op {
                CollapsibleOp::Open(v) => component.open(*v),
                CollapsibleOp::MotionId(id) => component.motion_id(id.clone()),
            };
        }
        if let Some(content) = request.take_slot("content") {
            component = component.content(content);
        }
        request.finish(component)
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor {
        name: "Collapsible",
        constructors: vec![ConstructorDescriptor::new("Collapsible", vec![], |_| {
            Ok(ComponentPayload::new(CollapsiblePayload))
        })],
        methods: vec![
            MethodDescriptor::new(
                "open",
                vec![ArgumentDescriptor::new("open", ArgumentSchema::Boolean)],
                |a| match a {
                    [ComponentArgument::Boolean(v)] => {
                        Ok(ComponentPayload::new(CollapsibleOp::Open(*v)))
                    }
                    _ => Err("Collapsible.open(open) expects a boolean".into()),
                },
            )
            .documented("Controls whether the content slot is revealed."),
            MethodDescriptor::new(
                "motionId",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |a| match a {
                    [ComponentArgument::String(v)] => {
                        Ok(ComponentPayload::new(CollapsibleOp::MotionId(v.clone())))
                    }
                    _ => Err("Collapsible.motionId(id) expects a string".into()),
                },
            )
            .documented("Adds stable identity for a reversible measured reveal."),
        ],
        typescript: TypeScriptDescriptor::new(
            "A trigger container with optional named `content` reveal content.",
        ),
        materializer: Arc::new(CollapsibleMaterializer),
    })?;
    Ok(())
}

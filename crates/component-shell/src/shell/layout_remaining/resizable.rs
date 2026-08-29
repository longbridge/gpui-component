use super::typed::{Carrier, take};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _, px},
    gpui_component::{
        ResizablePanel, ResizablePanelGroup, h_resizable, resizable_panel, v_resizable,
    },
};
use std::sync::Arc;

#[derive(Clone)]
struct Id(String);
#[derive(Clone, Copy)]
struct Empty;
#[derive(Clone, Copy)]
enum PanelOp {
    Visible(bool),
    Size(f32),
    Range(f32, f32),
}
#[derive(Clone, Copy)]
enum GroupOp {
    Axis(bool),
    CrossSize(f32),
}

fn finite_positive(value: f64, label: &str) -> Result<f32, String> {
    if value.is_finite() && value > 0. && value <= f32::MAX as f64 {
        Ok(value as f32)
    } else {
        Err(format!("{label} expects a positive finite pixel value"))
    }
}

fn require_group_style(style: gpui::StyleRefinement) -> anyhow::Result<()> {
    anyhow::ensure!(
        style == gpui::StyleRefinement::default(),
        "Resizable does not implement Styled; style its parent or panels"
    );
    Ok(())
}

fn require_panel_child(actual: Option<&'static str>) -> anyhow::Result<()> {
    anyhow::ensure!(
        actual == Some("ResizablePanel"),
        "Resizable accepts only ResizablePanel children; received {}",
        actual.unwrap_or("an ordinary element")
    );
    Ok(())
}

struct PanelMaterializer;
impl ComponentMaterializer for PanelMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("ResizablePanel received an incompatible payload"))?;
        let mut panel = resizable_panel();
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<PanelOp>())
        {
            panel = match op {
                PanelOp::Visible(v) => panel.visible(*v),
                PanelOp::Size(v) => panel.size(px(*v)),
                PanelOp::Range(a, b) => panel.size_range(px(*a)..px(*b)),
            };
        }
        panel.style().refine(&request.take_style());
        panel.extend(request.take_children()?);
        Ok(Carrier::new(panel).into_any_element())
    }
}

struct GroupMaterializer;
impl ComponentMaterializer for GroupMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<Id>()
            .ok_or_else(|| anyhow::anyhow!("Resizable received an incompatible payload"))?;
        let mut vertical = false;
        let mut size = None;
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<GroupOp>())
        {
            match op {
                GroupOp::Axis(v) => vertical = *v,
                GroupOp::CrossSize(v) => size = Some(*v),
            }
        }
        let mut group: ResizablePanelGroup = if vertical {
            v_resizable(id.0.clone())
        } else {
            h_resizable(id.0.clone())
        };
        if let Some(value) = size {
            group = group.size(px(value));
        }
        let style = request.take_style();
        require_group_style(style)?;
        for mut child in request.take_typed_children() {
            require_panel_child(child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            group = group.child(take::<ResizablePanel>(&mut element, "ResizablePanel")?);
        }
        Ok(group.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor {
        name: "ResizablePanel",
        constructors: vec![ConstructorDescriptor::new("ResizablePanel", vec![], |_| {
            Ok(ComponentPayload::new(Empty))
        })],
        methods: vec![
            MethodDescriptor::new(
                "visible",
                vec![ArgumentDescriptor::new("visible", ArgumentSchema::Boolean)],
                |a| match a {
                    [ComponentArgument::Boolean(v)] => {
                        Ok(ComponentPayload::new(PanelOp::Visible(*v)))
                    }
                    _ => Err("ResizablePanel.visible expects boolean".into()),
                },
            )
            .documented("Sets panel visibility."),
            MethodDescriptor::new(
                "size",
                vec![ArgumentDescriptor::new("pixels", ArgumentSchema::Number)],
                |a| match a {
                    [ComponentArgument::Number(v)] => finite_positive(*v, "ResizablePanel.size")
                        .map(PanelOp::Size)
                        .map(ComponentPayload::new),
                    _ => Err("ResizablePanel.size expects pixels".into()),
                },
            )
            .documented("Sets initial panel size in pixels."),
            MethodDescriptor::new(
                "sizeRange",
                vec![
                    ArgumentDescriptor::new("minimum", ArgumentSchema::Number),
                    ArgumentDescriptor::new("maximum", ArgumentSchema::Number),
                ],
                |a| match a {
                    [
                        ComponentArgument::Number(min),
                        ComponentArgument::Number(max),
                    ] => {
                        let min = finite_positive(*min, "minimum")?;
                        let max = finite_positive(*max, "maximum")?;
                        if min > max {
                            return Err("sizeRange minimum must not exceed maximum".into());
                        }
                        Ok(ComponentPayload::new(PanelOp::Range(min, max)))
                    }
                    _ => Err("ResizablePanel.sizeRange expects two numbers".into()),
                },
            )
            .documented("Sets the inclusive resize range in pixels."),
        ],
        typescript: TypeScriptDescriptor::new(
            "A typed resizable panel accepting ordinary children and shell style.",
        ),
        materializer: Arc::new(PanelMaterializer),
    })?;
    registry.register(ComponentDescriptor { name:"Resizable", constructors:vec![ConstructorDescriptor::new("Resizable",vec![ArgumentDescriptor::new("id",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(v)]if !v.trim().is_empty()=>Ok(ComponentPayload::new(Id(v.clone()))),_=>Err("Resizable expects a non-empty id".into())})], methods:vec![
        MethodDescriptor::new("axis",vec![ArgumentDescriptor::new("axis",ArgumentSchema::Enum(&["horizontal","vertical"]))],|a|match a{[ComponentArgument::Enum(v)]=>match v.as_str(){"horizontal"=>Ok(ComponentPayload::new(GroupOp::Axis(false))),"vertical"=>Ok(ComponentPayload::new(GroupOp::Axis(true))),_=>Err("unsupported axis".into())},_=>Err("Resizable.axis expects an axis".into())}).documented("Sets the resize axis."),
        MethodDescriptor::new("crossSize",vec![ArgumentDescriptor::new("pixels",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(v)]=>finite_positive(*v,"Resizable.crossSize").map(GroupOp::CrossSize).map(ComponentPayload::new),_=>Err("Resizable.crossSize expects pixels".into())}).documented("Sets the cross-axis size in pixels."),
    ], typescript:TypeScriptDescriptor::new("A typed native resizable group accepting only ResizablePanel children. It owns keyed state internally."), materializer:Arc::new(GroupMaterializer) })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_contracts_are_closed() {
        assert!(finite_positive(1.0, "x").is_ok());
        assert!(finite_positive(0.0, "x").is_err());
        assert!(finite_positive(f64::INFINITY, "x").is_err());
    }
    #[test]
    fn group_rejects_style_and_wrong_children() {
        assert!(require_group_style(gpui::StyleRefinement::default()).is_ok());
        assert!(require_group_style(gpui::StyleRefinement::default().p_2()).is_err());
        assert!(require_panel_child(Some("ResizablePanel")).is_ok());
        assert!(require_panel_child(Some("Textarea")).is_err());
        assert!(
            require_panel_child(None)
                .unwrap_err()
                .to_string()
                .contains("ordinary element")
        );
    }
}

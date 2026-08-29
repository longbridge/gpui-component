//! Native typed settings hierarchy.
//!
//! Value fields and reset callbacks are deferred: native getters accept only
//! `&App`, while shell callbacks currently require live `Window` + `App` authority.

mod typed;

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, Axis, IntoElement as _, ParentElement as _, Refineable as _, Styled as _, px},
    gpui_component::{
        Sizable as _, Size,
        setting::{SelectIndex, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    },
};
use std::sync::Arc;
use typed::{Carrier, take};

#[derive(Clone)]
struct Text(String);
#[derive(Clone, Copy)]
struct Empty;
#[derive(Clone)]
enum TextOp {
    Description(String),
    Title(String),
}
#[derive(Clone, Copy)]
enum BoolOp {
    DefaultOpen(bool),
    Resettable(bool),
}
#[derive(Clone)]
enum ItemOp {
    Description(String),
    Layout(Axis),
    Keywords(Vec<String>),
    Disabled(bool),
}
#[derive(Clone, Copy)]
enum SettingsOp {
    Size(Size),
    SidebarWidth(f32),
    SidebarRange(f32, f32),
    Selected(usize),
}
fn positive(v: f64, label: &str) -> Result<f32, String> {
    if v.is_finite() && v > 0. && v <= f32::MAX as f64 {
        Ok(v as f32)
    } else {
        Err(format!("{label} expects a positive finite pixel value"))
    }
}
fn reject_style(style: gpui::StyleRefinement, name: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        style == gpui::StyleRefinement::default(),
        "{name} does not implement Styled"
    );
    Ok(())
}
fn require(actual: Option<&'static str>, expected: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        actual == Some(expected),
        "expected only {expected} children; received {}",
        actual.unwrap_or("an ordinary element")
    );
    Ok(())
}

struct ItemMat;
impl ComponentMaterializer for ItemMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let title = r
            .payload()
            .downcast_ref::<Text>()
            .ok_or_else(|| anyhow::anyhow!("SettingItem payload"))?
            .0
            .clone();
        let field = r
            .take_slot_factory("content")
            .ok_or_else(|| anyhow::anyhow!("SettingItem requires content(element)"))?;
        anyhow::ensure!(
            r.children_len() == 0,
            "SettingItem does not accept children"
        );
        let mut sf = SettingField::render(move |_, window, cx| match field.build(window, cx) {
            Ok(e) => e,
            Err(e) => gpui::div()
                .child(format!("Failed to render setting field: {e:#}"))
                .into_any_element(),
        });
        sf.style().refine(&r.take_style());
        let mut item = SettingItem::new(title, sf);
        for op in r
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<ItemOp>())
        {
            item = match op {
                ItemOp::Description(v) => item.description(v.clone()),
                ItemOp::Layout(v) => item.layout(*v),
                ItemOp::Keywords(v) => item.keywords(v.clone()),
                ItemOp::Disabled(v) => item.disabled(*v),
            }
        }
        Ok(Carrier::new(item).into_any_element())
    }
}
struct GroupMat;
impl ComponentMaterializer for GroupMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        r.payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("SettingGroup payload"))?;
        let mut group = SettingGroup::new();
        for op in r.methods() {
            if let Some(TextOp::Title(v)) = op.payload().downcast_ref::<TextOp>() {
                group = group.title(v.clone())
            }
            if let Some(TextOp::Description(v)) = op.payload().downcast_ref::<TextOp>() {
                group = group.description(v.clone())
            }
        }
        group.style().refine(&r.take_style());
        for mut child in r.take_typed_children() {
            require(child.component_name(), "SettingItem")?;
            let mut e = r.materialize_child(&mut child)?;
            group = group.item(take::<SettingItem>(&mut e, "SettingItem")?)
        }
        Ok(Carrier::new(group).into_any_element())
    }
}
struct PageMat;
impl ComponentMaterializer for PageMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let title = r
            .payload()
            .downcast_ref::<Text>()
            .ok_or_else(|| anyhow::anyhow!("SettingPage payload"))?
            .0
            .clone();
        let mut page = SettingPage::new(title);
        for op in r.methods() {
            if let Some(TextOp::Description(v)) = op.payload().downcast_ref::<TextOp>() {
                page = page.description(v.clone())
            }
            if let Some(BoolOp::DefaultOpen(v)) = op.payload().downcast_ref::<BoolOp>() {
                page = page.default_open(*v)
            }
            if let Some(BoolOp::Resettable(v)) = op.payload().downcast_ref::<BoolOp>() {
                page = page.resettable(*v)
            }
        }
        if let Some(factory) = r.take_slot_factory("content") {
            page = page.title_suffix(move |window, cx| match factory.build(window, cx) {
                Ok(e) => e,
                Err(e) => gpui::div()
                    .child(format!("Failed to render title suffix: {e:#}"))
                    .into_any_element(),
            })
        }
        reject_style(r.take_style(), "SettingPage")?;
        for mut child in r.take_typed_children() {
            require(child.component_name(), "SettingGroup")?;
            let mut e = r.materialize_child(&mut child)?;
            page = page.group(take::<SettingGroup>(&mut e, "SettingGroup")?)
        }
        Ok(Carrier::new(page).into_any_element())
    }
}
struct SettingsMat;
impl ComponentMaterializer for SettingsMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = r
            .payload()
            .downcast_ref::<Text>()
            .ok_or_else(|| anyhow::anyhow!("Settings payload"))?
            .0
            .clone();
        let mut settings = Settings::new(id);
        for op in r
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<SettingsOp>())
        {
            settings = match op {
                SettingsOp::Size(v) => settings.with_size(*v),
                SettingsOp::SidebarWidth(v) => settings.sidebar_width(px(*v)),
                SettingsOp::SidebarRange(a, b) => settings.sidebar_size_range(px(*a)..px(*b)),
                SettingsOp::Selected(v) => settings.default_selected_index(SelectIndex {
                    page_ix: *v,
                    group_ix: None,
                }),
            }
        }
        reject_style(r.take_style(), "Settings")?;
        for mut child in r.take_typed_children() {
            require(child.component_name(), "SettingPage")?;
            let mut e = r.materialize_child(&mut child)?;
            settings = settings.page(take::<SettingPage>(&mut e, "SettingPage")?)
        }
        Ok(settings.into_any_element())
    }
}

fn text_method(name: &'static str, op: fn(String) -> TextOp) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("text", ArgumentSchema::String)],
        move |a| match a {
            [ComponentArgument::String(v)] if !v.trim().is_empty() => {
                Ok(ComponentPayload::new(op(v.clone())))
            }
            _ => Err(format!("{name} expects non-empty text")),
        },
    )
}
fn bool_method(name: &'static str, op: fn(bool) -> BoolOp) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("value", ArgumentSchema::Boolean)],
        move |a| match a {
            [ComponentArgument::Boolean(v)] => Ok(ComponentPayload::new(op(*v))),
            _ => Err(format!("{name} expects boolean")),
        },
    )
}
pub(super) fn register(reg: &mut ComponentRegistry) -> Result<(), RegistryError> {
    reg.register(ComponentDescriptor{name:"SettingItem",constructors:vec![ConstructorDescriptor::new("SettingItem",vec![ArgumentDescriptor::new("title",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(v)]if !v.trim().is_empty()=>Ok(ComponentPayload::new(Text(v.clone()))),_=>Err("SettingItem expects non-empty title".into())})],methods:vec![MethodDescriptor::new("description",vec![ArgumentDescriptor::new("text",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(v)]=>Ok(ComponentPayload::new(ItemOp::Description(v.clone()))),_=>Err("description expects text".into())}),MethodDescriptor::new("layout",vec![ArgumentDescriptor::new("axis",ArgumentSchema::Enum(&["horizontal","vertical"]))],|a|match a{[ComponentArgument::Enum(v)]if v=="horizontal"=>Ok(ComponentPayload::new(ItemOp::Layout(Axis::Horizontal))),[ComponentArgument::Enum(v)]if v=="vertical"=>Ok(ComponentPayload::new(ItemOp::Layout(Axis::Vertical))),_=>Err("layout expects horizontal or vertical".into())}),MethodDescriptor::new("keywords",vec![ArgumentDescriptor::new("keywords",ArgumentSchema::Array(Box::new(ArgumentSchema::String)))],|a|match a{[ComponentArgument::Array(v)]=>v.iter().map(|v|match v{ComponentArgument::String(s)=>Ok(s.clone()),_=>Err("keywords expects strings".into())}).collect::<Result<Vec<_>,String>>().map(ItemOp::Keywords).map(ComponentPayload::new),_=>Err("keywords expects string array".into())}),MethodDescriptor::new("disabled",vec![ArgumentDescriptor::new("disabled",ArgumentSchema::Boolean)],|a|match a{[ComponentArgument::Boolean(v)]=>Ok(ComponentPayload::new(ItemOp::Disabled(*v))),_=>Err("disabled expects boolean".into())})],typescript:TypeScriptDescriptor::new("A typed native setting item requiring lazy content(element); style applies to the field."),materializer:Arc::new(ItemMat)})?;
    reg.register(ComponentDescriptor {
        name: "SettingGroup",
        constructors: vec![ConstructorDescriptor::new("SettingGroup", vec![], |_| {
            Ok(ComponentPayload::new(Empty))
        })],
        methods: vec![
            text_method("title", TextOp::Title),
            text_method("description", TextOp::Description),
        ],
        typescript: TypeScriptDescriptor::new(
            "A styled native setting group accepting only SettingItem children.",
        ),
        materializer: Arc::new(GroupMat),
    })?;
    reg.register(ComponentDescriptor{name:"SettingPage",constructors:vec![ConstructorDescriptor::new("SettingPage",vec![ArgumentDescriptor::new("title",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(v)]if !v.trim().is_empty()=>Ok(ComponentPayload::new(Text(v.clone()))),_=>Err("SettingPage expects non-empty title".into())})],methods:vec![text_method("description",TextOp::Description),bool_method("defaultOpen",BoolOp::DefaultOpen),bool_method("resettable",BoolOp::Resettable)],typescript:TypeScriptDescriptor::new("A typed native setting page accepting SettingGroup children and lazy content(element) as its title suffix; style is rejected."),materializer:Arc::new(PageMat)})?;
    reg.register(ComponentDescriptor{name:"Settings",constructors:vec![ConstructorDescriptor::new("Settings",vec![ArgumentDescriptor::new("id",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(v)]if !v.trim().is_empty()=>Ok(ComponentPayload::new(Text(v.clone()))),_=>Err("Settings expects non-empty id".into())})],methods:vec![MethodDescriptor::new("size",vec![ArgumentDescriptor::new("size",ArgumentSchema::Enum(&["xsmall","small","medium","large"]))],|a|match a{[ComponentArgument::Enum(v)]=>match v.as_str(){"xsmall"=>Ok(Size::XSmall),"small"=>Ok(Size::Small),"medium"=>Ok(Size::Medium),"large"=>Ok(Size::Large),_=>Err("unsupported size".into())}.map(SettingsOp::Size).map(ComponentPayload::new),_=>Err("size expects semantic size".into())}),MethodDescriptor::new("sidebarWidth",vec![ArgumentDescriptor::new("pixels",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(v)]=>positive(*v,"sidebarWidth").map(SettingsOp::SidebarWidth).map(ComponentPayload::new),_=>Err("sidebarWidth expects number".into())}),MethodDescriptor::new("sidebarSizeRange",vec![ArgumentDescriptor::new("minimum",ArgumentSchema::Number),ArgumentDescriptor::new("maximum",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(min),ComponentArgument::Number(max)]=>{let min=positive(*min,"minimum")?;let max=positive(*max,"maximum")?;if min>max{return Err("minimum must not exceed maximum".into())}Ok(ComponentPayload::new(SettingsOp::SidebarRange(min,max)))},_=>Err("sidebarSizeRange expects two numbers".into())}),MethodDescriptor::new("defaultSelectedPage",vec![ArgumentDescriptor::new("index",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(v)]if v.is_finite()&&*v>=0.&&v.fract()==0.&&*v<=usize::MAX as f64=>Ok(ComponentPayload::new(SettingsOp::Selected(*v as usize))),_=>Err("defaultSelectedPage expects a nonnegative integer".into())})],typescript:TypeScriptDescriptor::new("A native keyed-state settings container accepting only SettingPage children; style is rejected."),materializer:Arc::new(SettingsMat)})?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_and_structural_contracts_are_closed() {
        assert_eq!(positive(1., "width").unwrap(), 1.);
        assert!(positive(0., "width").is_err());
        assert!(positive(f64::INFINITY, "width").is_err());
        assert!(require(Some("SettingPage"), "SettingPage").is_ok());
        assert!(require(Some("SettingGroup"), "SettingPage").is_err());
        assert!(require(None, "SettingPage").is_err());
        assert!(reject_style(gpui::StyleRefinement::default(), "Settings").is_ok());
        assert!(reject_style(gpui::StyleRefinement::default().p_2(), "Settings").is_err());
    }
}

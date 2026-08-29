use gpui_shell::{
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
    gpui_component::status_bar::StatusBar,
};
use std::sync::Arc;
#[derive(Clone)]
struct StatusBarPayload;
struct StatusBarMaterializer;
impl StatusBarMaterializer {
    fn component(payload: &ComponentPayload) -> anyhow::Result<StatusBar> {
        payload
            .downcast_ref::<StatusBarPayload>()
            .ok_or_else(|| anyhow::anyhow!("StatusBar received an incompatible payload"))?;
        Ok(StatusBar::new())
    }
}
impl ComponentMaterializer for StatusBarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut component = Self::component(request.payload())?;
        if let Some(left) = request.take_slot("left") {
            component = component.left(left)
        }
        if let Some(right) = request.take_slot("right") {
            component = component.right(right)
        }
        component.style().refine(&request.take_style());
        component.extend(request.take_children()?);
        Ok(component.into_any_element())
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor{name:"StatusBar",constructors:vec![ConstructorDescriptor::new("StatusBar",Vec::new(),|_|Ok(ComponentPayload::new(StatusBarPayload)))],methods:Vec::new(),typescript:TypeScriptDescriptor::new("A three-region status bar; ordinary children fill the center and named left/right slots pin content to each edge."),materializer:Arc::new(StatusBarMaterializer)})?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_real_status_bar() {
        drop(
            StatusBarMaterializer::component(&ComponentPayload::new(StatusBarPayload))
                .unwrap()
                .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            StatusBarMaterializer::component(&ComponentPayload::new(()))
                .err()
                .unwrap()
                .to_string(),
            "StatusBar received an incompatible payload"
        );
    }
}

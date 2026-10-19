//! Live NSH child composition. These handles retain NSH node identity rather
//! than serializing component trees or introducing another script runtime.
use gpui_kit::{AnyElement, IntoElement};
use nsh_platform_gpui::ComponentContext;

/// Current unnamed children, preserving NSH keyed identity and reactive owners.
pub fn children(context: &ComponentContext) -> Vec<AnyElement> {
    context
        .children()
        .into_iter()
        .map(IntoElement::into_any_element)
        .collect()
}

/// Current named JSX slot, if it is mounted.
pub fn slot(context: &ComponentContext, name: &str) -> Option<AnyElement> {
    context.slot(name).map(IntoElement::into_any_element)
}

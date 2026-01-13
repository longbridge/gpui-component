use crate::{
    Collapsible,
    sidebar::{SidebarGroup, SidebarMenu, SidebarSection},
};
use gpui::{AnyElement, IntoElement};

/// Convenience alias for [`SidebarContent`] using [`SidebarMenu`] as children.
pub type DefaultSidebarContent = SidebarContent<SidebarMenu>;

/// A polymorphic container for sidebar content.
///
/// Allows mixing [`SidebarGroup`] and [`SidebarSection`] in the same sidebar.
pub enum SidebarContent<E: Collapsible + IntoElement + 'static> {
    Labeled(Box<SidebarGroup<E>>),
    Section(Box<SidebarSection<E>>),
}

impl<E: Collapsible + IntoElement + 'static> Collapsible for SidebarContent<E> {
    fn is_collapsed(&self) -> bool {
        match self {
            SidebarContent::Labeled(g) => g.is_collapsed(),
            SidebarContent::Section(g) => g.is_collapsed(),
        }
    }

    fn collapsed(self, collapsed: bool) -> Self {
        match self {
            SidebarContent::Labeled(g) => SidebarContent::Labeled(Box::new(g.collapsed(collapsed))),
            SidebarContent::Section(g) => SidebarContent::Section(Box::new(g.collapsed(collapsed))),
        }
    }
}

impl<E: Collapsible + IntoElement + 'static> IntoElement for SidebarContent<E> {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        match self {
            SidebarContent::Labeled(g) => g.into_any_element(),
            SidebarContent::Section(g) => g.into_any_element(),
        }
    }
}

/// Converts a [`SidebarGroup`] into [`SidebarContent::Labeled`].
impl<E: Collapsible + IntoElement + 'static> From<SidebarGroup<E>> for SidebarContent<E> {
    fn from(group: SidebarGroup<E>) -> Self {
        SidebarContent::Labeled(Box::new(group))
    }
}

/// Converts a [`SidebarSection`] into [`SidebarContent::Section`].
impl<E: Collapsible + IntoElement + 'static> From<SidebarSection<E>> for SidebarContent<E> {
    fn from(group: SidebarSection<E>) -> Self {
        SidebarContent::Section(Box::new(group))
    }
}

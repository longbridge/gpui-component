use crate::{ActiveTheme, Collapsible, Icon, IconName, h_flex, skeleton::Skeleton, v_flex};
use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled as _, Window, div,
    prelude::FluentBuilder as _,
};

/// Controls what is displayed when the sidebar is collapsed.
#[derive(Default)]
pub enum SectionCollapsedDisplay {
    /// Show nothing.
    Hidden,
    /// Show the section [`SidebarSection::icon`].
    Icon,
    /// Show the section header content.
    Header,
    /// Show the section action element.
    Action,
    /// Show the section children in collapsed mode.
    #[default]
    Children,
    /// Show a custom element.
    Custom(AnyElement),
}

/// A section in a [`super::Sidebar`] with optional header, children, and states.
#[derive(IntoElement)]
pub struct SidebarSection<E: Collapsible + IntoElement + 'static> {
    icon: Option<IconName>,
    header: Option<AnyElement>,
    collapsed_header: SectionCollapsedDisplay,
    action: Option<AnyElement>,

    loading: bool,
    empty_state: Option<AnyElement>,

    separator_before: bool,
    separator_after: bool,

    children: Vec<E>,
    collapsed: bool,
}

impl<E: Collapsible + IntoElement> SidebarSection<E> {
    pub fn new() -> Self {
        Self {
            icon: None,
            header: None,
            collapsed_header: SectionCollapsedDisplay::default(),
            action: None,
            loading: false,
            empty_state: None,
            separator_before: false,
            separator_after: false,
            children: Vec::new(),
            collapsed: false,
        }
    }

    /// Sets the section icon, displayed before the header.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the header content.
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Sets what to display when the sidebar is collapsed.
    ///
    /// See [`SectionCollapsedDisplay`] for available options.
    pub fn collapsed_header(mut self, display: SectionCollapsedDisplay) -> Self {
        self.collapsed_header = display;
        self
    }

    /// Sets a custom element to display when collapsed.
    ///
    /// Shorthand for `.collapsed_header(CollapsedDisplay::Custom(...))`.
    pub fn collapsed_header_custom(mut self, content: impl IntoElement) -> Self {
        self.collapsed_header = SectionCollapsedDisplay::Custom(content.into_any_element());
        self
    }

    /// Sets an action element, displayed at the end of the header.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }

    /// Shows a loading state with [`Skeleton`] placeholders.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Sets the element to display when there are no children.
    pub fn empty_state(mut self, empty_state: impl IntoElement) -> Self {
        self.empty_state = Some(empty_state.into_any_element());
        self
    }

    /// Adds a separator line before this section.
    pub fn separator_before(mut self, separator: bool) -> Self {
        self.separator_before = separator;
        self
    }

    /// Adds a separator line after this section.
    pub fn separator_after(mut self, separator: bool) -> Self {
        self.separator_after = separator;
        self
    }

    /// Adds a child element.
    pub fn child(mut self, child: E) -> Self {
        self.children.push(child);
        self
    }

    /// Adds multiple child elements.
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.children.extend(children);
        self
    }
}

impl<E: Collapsible + IntoElement> Collapsible for SidebarSection<E> {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl<E: Collapsible + IntoElement> SidebarSection<E> {
    fn build_header(&mut self, cx: &App) -> AnyElement {
        if self.collapsed {
            let content = match std::mem::take(&mut self.collapsed_header) {
                SectionCollapsedDisplay::Hidden | SectionCollapsedDisplay::Children => None,
                SectionCollapsedDisplay::Icon => self
                    .icon
                    .take()
                    .map(|icon| Icon::new(icon).size_4().into_any_element()),
                SectionCollapsedDisplay::Header => self.header.take(),
                SectionCollapsedDisplay::Action => self.action.take(),
                SectionCollapsedDisplay::Custom(element) => Some(element),
            };

            match content {
                Some(el) => h_flex()
                    .w_full()
                    .h_8()
                    .items_center()
                    .justify_center()
                    .child(el)
                    .into_any_element(),
                None => div().into_any_element(),
            }
        } else {
            h_flex()
                .w_full()
                .items_center()
                .gap_1()
                .px_2()
                .h_8()
                .text_xs()
                .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                .when_some(self.icon.take(), |this, icon| {
                    this.child(Icon::new(icon).size_4())
                })
                .when_some(self.header.take(), |this, header| {
                    this.child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(header),
                    )
                })
                .when_some(self.action.take(), |this, action| this.child(action))
                .into_any_element()
        }
    }

    fn build_loading_state() -> AnyElement {
        v_flex()
            .gap_2()
            .px_2()
            .child(Skeleton::new().h_8())
            .child(Skeleton::new().h_8())
            .child(Skeleton::new().h_8().w_2_3())
            .into_any_element()
    }
}

impl<E: Collapsible + IntoElement> RenderOnce for SidebarSection<E> {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_children = !self.children.is_empty();
        let show_children_when_collapsed =
            matches!(self.collapsed_header, SectionCollapsedDisplay::Children);

        let has_header = self.header.is_some()
            || self.icon.is_some()
            || !matches!(
                self.collapsed_header,
                SectionCollapsedDisplay::Hidden | SectionCollapsedDisplay::Children
            );

        let header_element = if has_header {
            Some(self.build_header(cx))
        } else {
            None
        };

        let children_element = if has_children {
            Some(
                div()
                    .gap_2()
                    .flex_col()
                    .children(
                        self.children
                            .into_iter()
                            .map(|child| child.collapsed(self.collapsed)),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let content_element = if !self.collapsed {
            if self.loading {
                Some(Self::build_loading_state())
            } else if !has_children && self.empty_state.is_some() {
                Some(
                    div()
                        .px_2()
                        .py_4()
                        .when_some(self.empty_state.take(), |this, empty| this.child(empty))
                        .into_any_element(),
                )
            } else {
                children_element
            }
        } else if show_children_when_collapsed {
            children_element
        } else {
            None
        };

        v_flex()
            .relative()
            .when(self.separator_before && !self.collapsed, |this| {
                this.child(div().h_px().mx_2().my_2().bg(cx.theme().sidebar_border))
            })
            .when_some(header_element, |this, header| this.child(header))
            .when_some(content_element, |this, content| this.child(content))
            .when(self.separator_after && !self.collapsed, |this| {
                this.child(div().h_px().mx_2().my_2().bg(cx.theme().sidebar_border))
            })
    }
}

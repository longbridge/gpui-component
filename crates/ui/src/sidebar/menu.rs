use crate::{h_flex, v_flex, ActiveTheme as _, Collapsible, Icon, IconName, StyledExt};
use gpui::{
    div, percentage, prelude::FluentBuilder as _, App, ClickEvent, InteractiveElement as _,
    IntoElement, ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window,
};
use std::rc::Rc;

#[derive(IntoElement)]
pub struct SidebarMenu {
    collapsed: bool,
    items: Vec<SidebarMenuItem>,
}

impl SidebarMenu {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            collapsed: false,
        }
    }

    pub fn child(mut self, child: impl Into<SidebarMenuItem>) -> Self {
        self.items.push(child.into());
        self
    }

    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<SidebarMenuItem>>,
    ) -> Self {
        self.items = children.into_iter().map(Into::into).collect();
        self
    }
}
impl Collapsible for SidebarMenu {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}
impl RenderOnce for SidebarMenu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        v_flex().gap_2().children(
            self.items
                .into_iter()
                .map(|item| item.collapsed(self.collapsed)),
        )
    }
}

/// A sidebar menu item
#[derive(IntoElement)]
pub struct SidebarMenuItem {
    icon: Option<Icon>,
    label: SharedString,
    handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    active: bool,
    collapsed: bool,
    children: Vec<Self>,
}

impl SidebarMenuItem {
    /// Create a new SidebarMenuItem with a label
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            icon: None,
            label: label.into(),
            handler: Rc::new(|_, _, _| {}),
            active: false,
            collapsed: false,
            children: Vec::new(),
        }
    }

    /// Set the icon for the menu item
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the active state of the menu item
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Add a click handler to the menu item
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Rc::new(handler);
        self
    }

    /// Set the collapsed state of the menu item
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Self>>) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    fn is_submenu(&self) -> bool {
        self.children.len() > 0
    }

    fn is_open(&self) -> bool {
        if self.is_submenu() {
            self.active
        } else {
            false
        }
    }

    fn render_menu_item(&self, _: &Window, cx: &App) -> impl IntoElement {
        let handler = self.handler.clone();
        let is_collapsed = self.collapsed;
        let is_active = self.active;
        let is_open = self.is_open();
        let is_submenu = self.is_submenu();

        h_flex()
            .id("sidebar-menu-item")
            .overflow_hidden()
            .flex_shrink_0()
            .p_2()
            .gap_2()
            .items_center()
            .rounded(cx.theme().radius)
            .text_sm()
            .cursor_pointer()
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when(is_active && !is_submenu, |this| {
                this.font_medium()
                    .bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when_some(self.icon.clone(), |this, icon| this.child(icon.size_4()))
            .when(is_collapsed, |this| {
                this.justify_center().size_7().mx_auto()
            })
            .when(!is_collapsed, |this| {
                this.h_7()
                    .child(div().flex_1().child(self.label.clone()))
                    .when(is_submenu, |this| {
                        this.child(
                            Icon::new(IconName::ChevronRight)
                                .size_4()
                                .when(is_open, |this| this.rotate(percentage(90. / 360.))),
                        )
                    })
            })
            .on_click(move |ev, window, cx| handler(ev, window, cx))
    }
}

impl RenderOnce for SidebarMenuItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_submenu = self.is_submenu();
        let is_open = self.is_open();
        let is_collapsed = self.collapsed;

        div()
            .w_full()
            .child(self.render_menu_item(window, cx))
            .when(is_submenu && is_open && !is_collapsed, |this| {
                this.child(
                    v_flex()
                        .border_l_1()
                        .border_color(cx.theme().sidebar_border)
                        .gap_1()
                        .mx_3p5()
                        .px_2p5()
                        .py_0p5()
                        .children(self.children),
                )
            })
    }
}

use crate::{h_flex, v_flex, ActiveTheme as _, Collapsible, Icon, IconName, StyledExt};
use gpui::{
    div, percentage, prelude::FluentBuilder as _, App, ClickEvent, InteractiveElement as _,
    IntoElement, ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window,
};
use std::rc::Rc;

#[derive(IntoElement)]
pub struct SidebarMenu {
    is_collapsed: bool,
    items: Vec<SidebarMenuItem>,
}

impl SidebarMenu {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            is_collapsed: false,
        }
    }

    pub fn menu(
        mut self,
        label: impl Into<SharedString>,
        icon: Option<Icon>,
        active: bool,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(SidebarMenuItem::Item {
            icon,
            label: label.into(),
            handler: Rc::new(handler),
            active,
            is_collapsed: self.is_collapsed,
        });
        self
    }

    pub fn submenu(
        mut self,
        label: impl Into<SharedString>,
        icon: Option<Icon>,
        open: bool,
        items: impl FnOnce(SidebarMenu) -> Self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        let menu = SidebarMenu::new();
        let menu = items(menu);
        self.items.push(SidebarMenuItem::Submenu {
            icon,
            label: label.into(),
            items: menu.items,
            is_open: open,
            is_collapsed: self.is_collapsed,
            handler: Rc::new(handler),
        });
        self
    }
}
impl Collapsible for SidebarMenu {
    fn is_collapsed(&self) -> bool {
        self.is_collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.is_collapsed = collapsed;
        self
    }
}
impl RenderOnce for SidebarMenu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(self.items.into_iter().map(|mut item| {
                match &mut item {
                    SidebarMenuItem::Item { is_collapsed, .. } => *is_collapsed = self.is_collapsed,
                    SidebarMenuItem::Submenu { is_collapsed, .. } => {
                        *is_collapsed = self.is_collapsed
                    }
                }
                item
            }))
    }
}

/// A sidebar menu item
#[derive(IntoElement)]
enum SidebarMenuItem {
    Item {
        icon: Option<Icon>,
        label: SharedString,
        handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
        active: bool,
        is_collapsed: bool,
    },
    Submenu {
        icon: Option<Icon>,
        label: SharedString,
        handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
        items: Vec<SidebarMenuItem>,
        is_open: bool,
        is_collapsed: bool,
    },
}

impl SidebarMenuItem {
    fn is_submenu(&self) -> bool {
        matches!(self, SidebarMenuItem::Submenu { .. })
    }

    fn icon(&self) -> Option<Icon> {
        match self {
            SidebarMenuItem::Item { icon, .. } => icon.clone(),
            SidebarMenuItem::Submenu { icon, .. } => icon.clone(),
        }
    }

    fn label(&self) -> SharedString {
        match self {
            SidebarMenuItem::Item { label, .. } => label.clone(),
            SidebarMenuItem::Submenu { label, .. } => label.clone(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            SidebarMenuItem::Item { active, .. } => *active,
            SidebarMenuItem::Submenu { .. } => false,
        }
    }

    fn is_open(&self) -> bool {
        match self {
            SidebarMenuItem::Item { .. } => false,
            SidebarMenuItem::Submenu { is_open, items, .. } => {
                *is_open || items.iter().any(|item| item.is_active())
            }
        }
    }

    fn is_collapsed(&self) -> bool {
        match self {
            SidebarMenuItem::Item { is_collapsed, .. } => *is_collapsed,
            SidebarMenuItem::Submenu { is_collapsed, .. } => *is_collapsed,
        }
    }

    fn render_menu_item(
        &self,
        is_submenu: bool,
        is_active: bool,
        is_open: bool,
        _: &Window,
        cx: &App,
    ) -> impl IntoElement {
        let handler = match &self {
            SidebarMenuItem::Item { handler, .. } => Some(handler.clone()),
            SidebarMenuItem::Submenu { handler, .. } => Some(handler.clone()),
        };
        let is_collapsed = self.is_collapsed();

        h_flex()
            .id("sidebar-menu-item")
            .overflow_hidden()
            .flex_shrink_0()
            .p_2()
            .gap_2()
            .items_center()
            .rounded_md()
            .text_sm()
            .cursor_pointer()
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when(is_active, |this| {
                this.font_medium()
                    .bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when_some(self.icon(), |this, icon| this.child(icon.size_4()))
            .when(is_collapsed, |this| {
                this.justify_center().size_7().mx_auto()
            })
            .when(!is_collapsed, |this| {
                this.h_7()
                    .child(div().flex_1().child(self.label()))
                    .when(is_submenu, |this| {
                        this.child(
                            Icon::new(IconName::ChevronRight)
                                .size_4()
                                .when(is_open, |this| this.rotate(percentage(90. / 360.))),
                        )
                    })
            })
            .when_some(handler, |this, handler| {
                this.on_click(move |ev, window, cx| handler(ev, window, cx))
            })
    }
}

impl RenderOnce for SidebarMenuItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_submenu = self.is_submenu();
        let is_active = self.is_active();
        let is_open = self.is_open();

        div()
            .w_full()
            .child(self.render_menu_item(is_submenu, is_active, is_open, window, cx))
            .when(is_open, |this| {
                this.map(|this| match self {
                    SidebarMenuItem::Submenu {
                        items,
                        is_collapsed,
                        ..
                    } => {
                        if is_collapsed {
                            this
                        } else {
                            this.child(
                                v_flex()
                                    .border_l_1()
                                    .border_color(cx.theme().sidebar_border)
                                    .gap_1()
                                    .mx_3p5()
                                    .px_2p5()
                                    .py_0p5()
                                    .children(items),
                            )
                        }
                    }
                    _ => this,
                })
            })
    }
}

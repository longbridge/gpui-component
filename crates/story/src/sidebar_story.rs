use gpui::{
    div, impl_internal_actions, prelude::FluentBuilder, relative, App, AppContext, ClickEvent,
    Context, ElementId, Entity, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Window,
};

use gpui_component::{
    blue_500,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    divider::Divider,
    h_flex,
    popup_menu::PopupMenuExt,
    sidebar::{
        Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
        SidebarToggleButton,
    },
    switch::Switch,
    v_flex, white, ActiveTheme, Collapsible, Icon, IconName, Side,
};
use serde::Deserialize;

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectCompany(SharedString);

impl_internal_actions!(sidebar_story, [SelectCompany]);

pub struct SidebarStory {
    active_item: Item,
    active_subitem: Option<SubItem>,
    collapsed: bool,
    side: Side,
    focus_handle: gpui::FocusHandle,
}

impl SidebarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            active_item: Item::Playground,
            active_subitem: None,
            collapsed: false,
            side: Side::Left,
            focus_handle: cx.focus_handle(),
        }
    }

    fn render_content(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().child(
            h_flex().gap_2().child(
                Switch::new("side")
                    .label("Placement Right")
                    .checked(self.side.is_right())
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.side = if *checked { Side::Right } else { Side::Left };
                        cx.notify();
                    })),
            ),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Playground,
    Models,
    Documentation,
    Settings,
    DesignEngineering,
    SalesAndMarketing,
    Travel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubItem {
    History,
    Starred,
    General,
    Team,
    Billing,
    Limits,
    Settings,
    Genesis,
    Explorer,
    Quantum,
    Introduction,
    GetStarted,
    Tutorial,
    Changelog,
}

impl Item {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::Models => "Models",
            Self::Documentation => "Documentation",
            Self::Settings => "Settings",
            Self::DesignEngineering => "Design Engineering",
            Self::SalesAndMarketing => "Sales and Marketing",
            Self::Travel => "Travel",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Playground => IconName::SquareTerminal,
            Self::Models => IconName::Bot,
            Self::Documentation => IconName::BookOpen,
            Self::Settings => IconName::Settings2,
            Self::DesignEngineering => IconName::Frame,
            Self::SalesAndMarketing => IconName::ChartPie,
            Self::Travel => IconName::Map,
        }
    }

    pub fn handler(
        &self,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *self;
        move |this, _, _, cx| {
            this.active_item = item;
            this.active_subitem = None;
            cx.notify();
        }
    }

    pub fn items(&self) -> Vec<SubItem> {
        match self {
            Self::Playground => vec![SubItem::History, SubItem::Starred, SubItem::Settings],
            Self::Models => vec![SubItem::Genesis, SubItem::Explorer, SubItem::Quantum],
            Self::Documentation => vec![
                SubItem::Introduction,
                SubItem::GetStarted,
                SubItem::Tutorial,
                SubItem::Changelog,
            ],
            Self::Settings => vec![
                SubItem::General,
                SubItem::Team,
                SubItem::Billing,
                SubItem::Limits,
            ],
            _ => Vec::new(),
        }
    }
}

impl SubItem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::History => "History",
            Self::Starred => "Starred",
            Self::Settings => "Settings",
            Self::Genesis => "Genesis",
            Self::Explorer => "Explorer",
            Self::Quantum => "Quantum",
            Self::Introduction => "Introduction",
            Self::GetStarted => "Get Started",
            Self::Tutorial => "Tutorial",
            Self::Changelog => "Changelog",
            Self::Team => "Team",
            Self::Billing => "Billing",
            Self::Limits => "Limits",
            Self::General => "General",
        }
    }

    pub fn handler(
        &self,
        item: &Item,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *item;
        let subitem = *self;
        move |this, _, _, cx| {
            this.active_item = item;
            this.active_subitem = Some(subitem);
            cx.notify();
        }
    }
}

impl super::Story for SidebarStory {
    fn title() -> &'static str {
        "Sidebar"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for SidebarStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarStory {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let groups: [Vec<Item>; 2] = [
            vec![
                Item::Playground,
                Item::Models,
                Item::Documentation,
                Item::Settings,
            ],
            vec![
                Item::DesignEngineering,
                Item::SalesAndMarketing,
                Item::Travel,
            ],
        ];

        let sidebar = if self.side.is_left() {
            Sidebar::left(&cx.entity())
        } else {
            Sidebar::right(&cx.entity())
        };

        h_flex()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .when(self.side.is_right(), |this| this.flex_row_reverse())
            .child(
                sidebar
                    .collapsed(self.collapsed)
                    .header(
                        SidebarHeader::new()
                            .collapsed(self.collapsed)
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(cx.theme().radius)
                                    .bg(blue_500())
                                    .text_color(white())
                                    .size_8()
                                    .flex_shrink_0()
                                    .when(!self.collapsed, |this| {
                                        this.child(Icon::new(IconName::GalleryVerticalEnd).size_4())
                                    })
                                    .when(self.collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                            .child(Icon::new(IconName::GalleryVerticalEnd).size_5())
                                    }),
                            )
                            .when(!self.collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .gap_0()
                                        .text_sm()
                                        .flex_1()
                                        .line_height(relative(1.25))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child("Company Name")
                                        .child(div().child("Enterprise").text_xs()),
                                )
                            })
                            .when(!self.collapsed, |this| {
                                this.child(
                                    Icon::new(IconName::ChevronsUpDown).size_4().flex_shrink_0(),
                                )
                            })
                            .popup_menu(|menu, _, _| {
                                menu.menu(
                                    "Twitter Inc.",
                                    Box::new(SelectCompany(SharedString::from("twitter"))),
                                )
                                .menu(
                                    "Meta Platforms",
                                    Box::new(SelectCompany(SharedString::from("meta"))),
                                )
                                .menu(
                                    "Google Inc.",
                                    Box::new(SelectCompany(SharedString::from("google"))),
                                )
                            }),
                    )
                    .footer(
                        SidebarFooter::new()
                            .collapsed(self.collapsed)
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(IconName::CircleUser)
                                    .when(!self.collapsed, |this| this.child("Jason Lee")),
                            )
                            .when(!self.collapsed, |this| {
                                this.child(
                                    Icon::new(IconName::ChevronsUpDown).size_4().flex_shrink_0(),
                                )
                            }),
                    )
                    .child(
                        SidebarGroup::new("Platform").child(SidebarMenu::new().children({
                            let mut items = Vec::with_capacity(groups[0].len());
                            for (ix, item) in groups[0].iter().enumerate() {
                                let item = *item;
                                let id =
                                    ElementId::Name(SharedString::new(format!("item-{}", ix,)));

                                items.push(
                                    SidebarMenuItem::new(item.label())
                                        .id(id)
                                        .icon(item.icon().into())
                                        .active(self.active_item == item)
                                        .children({
                                            let mut sub_items =
                                                Vec::with_capacity(item.items().len());
                                            for (ix, sub_item) in
                                                item.items().into_iter().enumerate()
                                            {
                                                let id = ElementId::Name(SharedString::new(
                                                    format!("sub-item-{}", ix),
                                                ));

                                                sub_items.push(
                                                    SidebarMenuItem::new(sub_item.label())
                                                        .id(id)
                                                        .active(
                                                            self.active_subitem == Some(sub_item),
                                                        )
                                                        .on_click(
                                                            cx.listener(sub_item.handler(&item)),
                                                        ),
                                                );
                                            }
                                            sub_items
                                        })
                                        .on_click(cx.listener(item.handler())),
                                );
                            }
                            items
                        })),
                    )
                    .child(
                        SidebarGroup::new("Projects").child(SidebarMenu::new().children({
                            let mut items = Vec::with_capacity(groups[1].len());
                            for (ix, item) in groups[1].iter().enumerate() {
                                let id =
                                    ElementId::Name(SharedString::new(format!("project-{}", ix)));

                                items.push(
                                    SidebarMenuItem::new(item.label())
                                        .id(id)
                                        .icon(item.icon().into())
                                        .active(self.active_item == *item)
                                        .on_click(cx.listener(item.handler())),
                                );
                            }
                            items
                        })),
                    ),
            )
            .child(
                v_flex()
                    .size_full()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .when(self.side.is_right(), |this| {
                                this.flex_row_reverse().justify_between()
                            })
                            .child(
                                SidebarToggleButton::left()
                                    .side(self.side)
                                    .collapsed(self.collapsed)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.collapsed = !this.collapsed;
                                        cx.notify();
                                    })),
                            )
                            .child(Divider::vertical().h_4())
                            .child(
                                Breadcrumb::new()
                                    .item(BreadcrumbItem::new("0", "Home").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.active_item = Item::Playground;
                                            cx.notify();
                                        },
                                    )))
                                    .item(
                                        BreadcrumbItem::new("1", self.active_item.label())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.active_subitem = None;
                                                cx.notify();
                                            })),
                                    )
                                    .when_some(self.active_subitem, |this, subitem| {
                                        this.item(BreadcrumbItem::new("2", subitem.label()))
                                    }),
                            ),
                    )
                    .child(self.render_content(window, cx)),
            )
    }
}

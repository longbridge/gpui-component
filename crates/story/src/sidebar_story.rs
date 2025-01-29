use gpui::{
    div, impl_internal_actions, relative, App, AppContext, ClickEvent, Context, Entity, Focusable,
    ParentElement, Render, SharedString, Styled, Window,
};

use serde::Deserialize;
use ui::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    divider::Divider,
    h_flex,
    popup_menu::PopupMenuExt,
    prelude::FluentBuilder,
    sidebar::{
        Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarToggleButton,
    },
    v_flex, ActiveTheme, Collapsible, Icon, IconName,
};

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectCompany(SharedString);

impl_internal_actions!(sidebar_story, [SelectCompany]);

pub struct SidebarStory {
    active_item: Item,
    active_subitem: Option<SubItem>,
    is_collapsed: bool,
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
            is_collapsed: false,
            focus_handle: cx.focus_handle(),
        }
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
        _: &mut gpui::Window,
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

        h_flex()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .child(
                Sidebar::left(&cx.entity())
                    .collapsed(self.is_collapsed)
                    .header(
                        SidebarHeader::new()
                            .collapsed(self.is_collapsed)
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .bg(ui::blue_500())
                                    .text_color(ui::white())
                                    .size_8()
                                    .flex_shrink_0()
                                    .child(Icon::new(IconName::GalleryVerticalEnd).size_4())
                                    .when(self.is_collapsed, |this| {
                                        this.size_4().bg(cx.theme().transparent)
                                    }),
                            )
                            .when(!self.is_collapsed, |this| {
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
                            .when(!self.is_collapsed, |this| {
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
                            .collapsed(self.is_collapsed)
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(IconName::CircleUser)
                                    .when(!self.is_collapsed, |this| this.child("Jason Lee")),
                            )
                            .when(!self.is_collapsed, |this| {
                                this.child(
                                    Icon::new(IconName::ChevronsUpDown).size_4().flex_shrink_0(),
                                )
                            }),
                    )
                    .child(SidebarGroup::new("Platform").child(SidebarMenu::new().map(
                        |mut menu| {
                            for item in groups[0].iter() {
                                let item = *item;
                                menu = menu.submenu(
                                    item.label(),
                                    Some(item.icon().into()),
                                    self.active_item == item,
                                    |mut submenu| {
                                        for subitem in item.items() {
                                            submenu = submenu.menu(
                                                subitem.label(),
                                                None,
                                                self.active_subitem == Some(subitem),
                                                cx.listener(subitem.handler(&item)),
                                            );
                                        }
                                        submenu
                                    },
                                    cx.listener(move |this, _, _, cx| {
                                        this.active_item = item;
                                        cx.notify();
                                    }),
                                );
                            }
                            menu
                        },
                    )))
                    .child(SidebarGroup::new("Projects").child(SidebarMenu::new().map(
                        |mut menu| {
                            for item in groups[1].iter() {
                                menu = menu.menu(
                                    item.label(),
                                    Some(item.icon().into()),
                                    self.active_item == *item,
                                    cx.listener(item.handler()),
                                );
                            }
                            menu
                        },
                    ))),
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
                            .child(
                                SidebarToggleButton::left()
                                    .collapsed(self.is_collapsed)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.is_collapsed = !this.is_collapsed;
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
                    .child("This content"),
            )
    }
}

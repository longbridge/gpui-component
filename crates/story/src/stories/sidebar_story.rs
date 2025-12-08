use std::collections::HashMap;

use gpui::{
    Action, App, AppContext, ClickEvent, Context, Entity, Focusable, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, div, prelude::FluentBuilder, px, relative,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, Side, Sizable,
    badge::Badge,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    divider::Divider,
    h_flex,
    menu::DropdownMenu,
    sidebar::{
        Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
        SidebarToggleButton,
    },
    switch::Switch,
    v_flex,
};
use serde::Deserialize;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sidebar_story, no_json)]
pub struct SelectCompany(SharedString);

pub struct SidebarStory {
    active_items: HashMap<Item, bool>,
    last_active_item: Item,
    active_subitem: Option<SubItem>,
    collapsed: bool,
    side: Side,
    click_to_open_submenu: bool,
    focus_handle: gpui::FocusHandle,
    checked: bool,
}

impl SidebarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut active_items = HashMap::new();
        active_items.insert(Item::Playground, true);

        Self {
            active_items,
            last_active_item: Item::Playground,
            active_subitem: None,
            collapsed: false,
            side: Side::Left,
            focus_handle: cx.focus_handle(),
            checked: false,
            click_to_open_submenu: false,
        }
    }

    fn render_content(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            h_flex()
                .gap_3()
                .child(
                    Switch::new("side")
                        .label("Placement Right")
                        .checked(self.side.is_right())
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.side = if *checked { Side::Right } else { Side::Left };
                            cx.notify();
                        })),
                )
                .child(
                    Switch::new("click-to-open")
                        .checked(self.click_to_open_submenu)
                        .label("Click to open submenu")
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.click_to_open_submenu = *checked;
                            cx.notify();
                        })),
                ),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Travel => true,
            _ => false,
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
            if this.active_items.contains_key(&item) {
                this.active_items.remove(&item);
            } else {
                this.active_items.insert(item, true);
            }

            this.last_active_item = item;
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

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Quantum => true,
            _ => false,
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
            println!(
                "Clicked on item: {}, child: {}",
                item.label(),
                subitem.label()
            );
            this.active_items.insert(item, true);
            this.last_active_item = item;
            this.active_subitem = Some(subitem);
            cx.notify();
        }
    }
}

impl super::Story for SidebarStory {
    fn title() -> &'static str {
        "Sidebar"
    }

    fn description() -> &'static str {
        "A composable, themeable and customizable sidebar component."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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

        h_flex()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .when(self.side.is_right(), |this| this.flex_row_reverse())
            .child(
                Sidebar::new(self.side)
                    .collapsed(self.collapsed)
                    .w(px(220.))
                    .gap_0()
                    .header(
                        SidebarHeader::new()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().success)
                                    .text_color(cx.theme().success_foreground)
                                    .size_8()
                                    .flex_shrink_0()
                                    .when(!self.collapsed, |this| {
                                        this.child(Icon::new(IconName::GalleryVerticalEnd))
                                    })
                                    .when(self.collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                            .child(Icon::new(IconName::GalleryVerticalEnd))
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
                            .dropdown_menu(|menu, _, _| {
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
                    .child(
                        SidebarGroup::new("Platform").child(SidebarMenu::new().children(
                            groups[0].iter().enumerate().map(|(ix, item)| {
                                let is_active =
                                    self.last_active_item == *item && self.active_subitem == None;
                                SidebarMenuItem::new(item.label())
                                    .icon(item.icon())
                                    .active(is_active)
                                    .default_open(ix == 0)
                                    .click_to_open(self.click_to_open_submenu)
                                    .children(item.items().into_iter().enumerate().map(
                                        |(ix, sub_item)| {
                                            SidebarMenuItem::new(sub_item.label())
                                                .active(self.active_subitem == Some(sub_item))
                                                .disable(sub_item.is_disabled())
                                                .when(ix == 0, |this| {
                                                    this.suffix(
                                                        Switch::new("switch")
                                                            .xsmall()
                                                            .checked(self.checked)
                                                            .on_click(cx.listener(
                                                                |this, checked, _, _| {
                                                                    this.checked = *checked
                                                                },
                                                            )),
                                                    )
                                                })
                                                .on_click(cx.listener(sub_item.handler(&item)))
                                        },
                                    ))
                                    .on_click(cx.listener(item.handler()))
                            }),
                        )),
                    )
                    .child(
                        SidebarGroup::new("Projects").child(SidebarMenu::new().children(
                            groups[1].iter().enumerate().map(|(ix, item)| {
                                let is_active =
                                    self.last_active_item == *item && self.active_subitem == None;
                                SidebarMenuItem::new(item.label())
                                    .icon(item.icon())
                                    .active(is_active)
                                    .disable(item.is_disabled())
                                    .click_to_open(self.click_to_open_submenu)
                                    .when(ix == 0, |this| {
                                        this.suffix(
                                            Badge::new().dot().count(1).child(
                                                div().p_0p5().child(Icon::new(IconName::Bell)),
                                            ),
                                        )
                                    })
                                    .when(ix == 1, |this| this.suffix(IconName::Settings2))
                                    .on_click(cx.listener(item.handler()))
                            }),
                        )),
                    )
                    .footer(
                        SidebarFooter::new()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(IconName::CircleUser)
                                    .when(!self.collapsed, |this| this.child("Jason Lee")),
                            )
                            .when(!self.collapsed, |this| {
                                this.child(Icon::new(IconName::ChevronsUpDown).size_4())
                            }),
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
                                    .child("Breadcrumb")
                                    .child(BreadcrumbItem::new("Home").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.last_active_item = Item::Playground;
                                            cx.notify();
                                        },
                                    )))
                                    .child(
                                        BreadcrumbItem::new(self.last_active_item.label())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.active_subitem = None;
                                                cx.notify();
                                            })),
                                    )
                                    .when_some(self.active_subitem, |this, subitem| {
                                        this.child(BreadcrumbItem::new(subitem.label()))
                                    }),
                            ),
                    )
                    .child(self.render_content(window, cx)),
            )
    }
}

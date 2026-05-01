use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem, SidebarToggleButton,
    },
    *,
};
use gpui_component_assets::Assets;

pub struct Example {
    collapsed: bool,
}

impl Example {
    fn new() -> Self {
        Self { collapsed: false }
    }

    fn menu() -> SidebarMenu {
        SidebarMenu::new().children([
            SidebarMenuItem::new("Dashboard")
                .icon(IconName::LayoutDashboard)
                .active(true),
            SidebarMenuItem::new("Inbox").icon(IconName::Inbox),
            SidebarMenuItem::new("Calendar").icon(IconName::Calendar),
            SidebarMenuItem::new("Projects")
                .icon(IconName::Folder)
                .default_open(true)
                .click_to_toggle(true)
                .children([
                    SidebarMenuItem::new("Design"),
                    SidebarMenuItem::new("Engineering"),
                    SidebarMenuItem::new("Marketing"),
                ]),
            SidebarMenuItem::new("Settings").icon(IconName::Settings),
        ])
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsible = SidebarCollapsible::Icon;
        let icon_collapsed = self.collapsed && collapsible == SidebarCollapsible::Icon;

        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                Sidebar::new("sidebar-icon-example")
                    .collapsible(collapsible)
                    .collapsed(self.collapsed)
                    .w(px(240.))
                    .header(
                        SidebarHeader::new()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size_8()
                                    .flex_shrink_0()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().sidebar_primary)
                                    .text_color(cx.theme().sidebar_primary_foreground)
                                    .when(icon_collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                    })
                                    .child(Icon::new(IconName::GalleryVerticalEnd)),
                            )
                            .when(!icon_collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .flex_1()
                                        .overflow_hidden()
                                        .child("Acme Inc")
                                        .child(div().text_xs().child("Enterprise")),
                                )
                            }),
                    )
                    .child(SidebarGroup::new("Application").child(Self::menu()))
                    .footer(
                        SidebarFooter::new().child(
                            h_flex()
                                .gap_2()
                                .child(IconName::CircleUser)
                                .when(!icon_collapsed, |this| this.child("Jason Lee")),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                SidebarToggleButton::new().collapsed(icon_collapsed).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.collapsed = !this.collapsed;
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(div().font_bold().child("Icon collapsible sidebar")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .p_5()
                            .child("The sidebar collapses to icon width, matching shadcn's collapsible=\"icon\" behavior."),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(620.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example::new());
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}

use super::{
    introduction::Introduction, llm_provider::LlmProvider, mcp_provider::McpProvider,
    profile::Profile, user_guide::UserGuide,
};
use crate::app::AppExt;
use crate::app::AppState;
#[cfg(target_os = "windows")]
use crate::app::WindowExt;
use crate::{
    app::{Quit, ToggleSearch},
    ui::components::container::Container,
};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    resizable::{h_resizable, resizable_panel, ResizableState},
    sidebar::{Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    white, *,
};
use serde::Deserialize;

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectCompany(SharedString);

impl_internal_actions!(sidebar_story, [SelectCompany]);

pub struct Settings {
    stories: Vec<(&'static str, Vec<Entity<Container>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    collapsed: bool,
    // search_input: Entity<InputState>,
    sidebar_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,
    side: Side,
}

impl Settings {
    pub fn init(cx: &mut App) {
        // 绑定键盘快捷键
        cx.bind_keys([
            KeyBinding::new("/", ToggleSearch, None), // 斜杠键切换搜索
            KeyBinding::new("cmd-q", Quit, None),     // Cmd+Q 退出
        ]);
    }
}

impl Settings {
    pub fn open(init_view: Option<&str>, parent: &mut Window, cx: &mut App) {
        cx.activate(true);
        let window_size = size(px(1024.0), px(920.0));
        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };
        let init_view = if let Some(init_view) = init_view {
            init_view.to_string()
        } else {
            "个人资料".to_string()
        };

        let parent_handle = parent.window_handle();
        cx.create_normal_window("xTo-Do 设置", options, move |window, cx| {
            cx.new(|cx| Self::new(&init_view, parent_handle, window, cx))
        });
        // #[cfg(target_os = "windows")]
        // parent.enable_window(false);
       
    }

    fn new(
        init_view: &str,
        parent: AnyWindowHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // window.topmost_window();
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let sub = cx.on_window_closed(move |cx| {
            parent
                .update(cx, |_, window, _cx| {
                    window.activate_window();
                    #[cfg(target_os = "windows")]
                    window.enable_window(true);
                })
                .ok();
        });
        let _subscriptions = vec![
            cx.subscribe(&search_input, |this, _, e, cx| match e {
                InputEvent::Change(_) => {
                    this.active_group_index = Some(0);
                    this.active_index = Some(0);
                    cx.notify()
                }
                _ => {}
            }),
            sub,
        ];

        let stories = vec![
            ("个人资料", vec![Container::panel::<Profile>(window, cx)]),
            (
                "入门指南",
                vec![
                    Container::panel::<Introduction>(window, cx),
                    Container::panel::<UserGuide>(window, cx),
                ],
            ),
            (
                "设置",
                vec![
                    Container::panel::<LlmProvider>(window, cx),
                    Container::panel::<McpProvider>(window, cx),
                    // Container::panel::<LlmProvider>(window, cx),
                    // Container::panel::<LlmProvider>(window, cx),
                ],
            ),
        ];

        let mut this = Self {
            // search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            collapsed: false,
            sidebar_state: ResizableState::new(cx),
            _subscriptions,
            side: Side::Left,
        };

        this.set_active_story(init_view, cx);
        this
    }

    fn set_active_story(&mut self, name: &str, cx: &mut App) {
        // let group_index = 1;
        // let Some(story_index) = self
        //     .stories
        //     .get(group_index)
        //     .and_then(|(_, stories)| stories.iter().position(|story| story.read(cx).name == name))
        // else {
        //     return;
        // };

        // self.active_group_index = Some(group_index);
        // self.active_index = Some(story_index);
        for (group_index, (_, stories)) in self.stories.iter().enumerate() {
            if let Some(story_index) = stories.iter().position(|story| story.read(cx).name == name)
            {
                self.active_group_index = Some(group_index);
                self.active_index = Some(story_index);
                return;
            }
        }
    }
}

impl Render for Settings {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let active_group = self
            .active_group_index
            .and_then(|index| self.stories.get(index));
        let active_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()));
        let (story_name, description) =
            if let Some(story) = active_story.as_ref().map(|story| story.read(cx)) {
                (story.name.clone(), story.description.clone())
            } else {
                ("".into(), "".into())
            };

        h_resizable("settings-container", self.sidebar_state.clone())
            .child(
                resizable_panel()
                    .size(px(220.))
                    .size_range(px(220.)..px(220.))
                    .child(
                        Sidebar::new(self.side)
                            .width(relative(1.))
                            .border_width(px(0.))
                            .collapsed(self.collapsed)
                            .header(
                                SidebarHeader::new()
                                    .justify_between()
                                    .selected(
                                        self.active_group_index == Some(0)
                                            && self.active_index == Some(0),
                                    )
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.active_group_index = Some(0);
                                        this.active_index = Some(0);
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .id("profile-item")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            // .rounded(cx.theme().radius)
                                            // .bg(orange_500())
                                            // .text_color(white())
                                            // .size_8()
                                            .flex_shrink_0()
                                            .when(!self.collapsed, |this| {
                                                this.child(
                                                    Icon::new(IconName::CircleUser).size(px(36.)),
                                                )
                                            })
                                            .when(self.collapsed, |this| {
                                                this.size_4()
                                                    .bg(cx.theme().transparent)
                                                    .text_color(cx.theme().foreground)
                                                    .child(
                                                        Icon::new(IconName::CircleUser)
                                                            .size(px(36.)),
                                                    )
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
                                                .child(
                                                    AppState::state(cx)
                                                        .profile_manager
                                                        .profile
                                                        .name
                                                        .clone(),
                                                )
                                                .child(
                                                    div()
                                                        .child(
                                                            AppState::state(cx)
                                                                .profile_manager
                                                                .profile
                                                                .department
                                                                .clone(),
                                                        )
                                                        .text_xs(),
                                                ),
                                        )
                                    }),
                            )
                            .children(self.stories.clone().into_iter().skip(1).enumerate().map(
                                |(group_ix, (group_name, sub_stories))| {
                                    SidebarGroup::new(group_name.to_string()).child(
                                        SidebarMenu::new().children(
                                            sub_stories.iter().enumerate().map(|(ix, story)| {
                                                SidebarMenuItem::new(story.read(cx).name.clone())
                                                    .when_some(
                                                        story.read(cx).icon.clone(),
                                                        |item, icon| item.icon(icon),
                                                    )
                                                    .active(
                                                        self.active_group_index
                                                            == Some(group_ix + 1)
                                                            && self.active_index == Some(ix),
                                                    )
                                                    .on_click(cx.listener(
                                                        move |this, _: &ClickEvent, _, cx| {
                                                            this.active_group_index =
                                                                Some(group_ix + 1);
                                                            this.active_index = Some(ix);
                                                            cx.notify();
                                                        },
                                                    ))
                                            }),
                                        ),
                                    )
                                },
                            ))
                            .footer(
                                SidebarFooter::new()
                                    .justify_between()
                                    .p_0()
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        cx.open_url("https://www.shouqianba.com");
                                        // cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(cx.theme().radius)
                                            .bg(orange_300())
                                            .text_color(white())
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
                                                .child("收钱吧")
                                                .child(
                                                    div()
                                                        .child("服务千万商家 全能生意帮手")
                                                        .text_xs(),
                                                ),
                                        )
                                    })
                                    .when(self.collapsed, |this| {
                                        this.child(
                                            Icon::new(IconName::GalleryVerticalEnd)
                                                .size_4()
                                                .flex_shrink_0(),
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_x_hidden()
                    .child(
                        h_flex()
                            .id("header")
                            .p_4()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .justify_between()
                            .items_start()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xl().child(story_name))
                                    .child(
                                        div()
                                            .text_color(cx.theme().muted_foreground)
                                            .text_sm()
                                            .child(description),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("story")
                            .flex_1()
                            .overflow_y_scroll()
                            .when_some(active_story, |this, active_story| {
                                this.child(active_story.clone())
                            }),
                    )
                    .into_any_element(),
            )
    }
}

//! 团队管理面板
//!
//! 提供团队 CRUD、成员管理和团队密钥管理功能。

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::label::Label;
use gpui_component::{
    ActiveTheme, Disableable, Icon, Sizable,
    button::ButtonVariants as _,
    h_flex,
    input::{Input, InputState},
    v_flex, IconName,
};
use one_core::cloud_sync::{CloudApiClient, CloudSyncService, GlobalCloudUser, Team, TeamMember, TeamRole};
use one_core::cloud_sync::supabase::SupabaseClient;
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;
use std::sync::Arc;
use one_core::storage::now;
use crate::auth;

/// 团队管理面板
pub struct TeamManagementPanel {
    focus_handle: FocusHandle,
    /// 团队列表
    teams: Vec<Team>,
    /// 当前选中的团队索引
    selected_team_idx: Option<usize>,
    /// 选中团队的成员列表
    team_members: Vec<TeamMember>,
    /// 是否正在加载
    loading: bool,
    /// 错误信息
    error: Option<String>,
    /// 云同步服务（用于团队密钥管理）
    cloud_sync_service: Arc<std::sync::RwLock<CloudSyncService>>,
    /// 云端 API 客户端
    cloud_client: Arc<SupabaseClient>,
    /// 新团队名称输入
    new_team_name_input: Entity<InputState>,
    /// 新团队描述输入
    new_team_desc_input: Entity<InputState>,
    /// 添加成员邮箱输入
    add_member_email_input: Entity<InputState>,
    /// 团队密钥输入
    team_key_input: Entity<InputState>,
    /// 是否正在创建团队
    creating: bool,
    /// 成功提示
    success_message: Option<String>,
    /// 事件订阅
    _subscriptions: Vec<Subscription>,
}

impl TeamManagementPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let cloud_sync_service =
            Arc::new(std::sync::RwLock::new(CloudSyncService::new()));

        let auth_service = auth::get_auth_service(cx);
        let cloud_client = auth_service.cloud_client();

        let new_team_name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.team_name"))
        });
        let new_team_desc_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.team_description"))
        });
        let add_member_email_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.member_email"))
        });
        let team_key_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.enter_team_key"))
        });

        // 不需要订阅 InputEvent::Change，直接在操作时读取 value
        let mut panel = Self {
            focus_handle,
            teams: Vec::new(),
            selected_team_idx: None,
            team_members: Vec::new(),
            loading: false,
            error: None,
            cloud_sync_service,
            cloud_client,
            new_team_name_input,
            new_team_desc_input,
            add_member_email_input,
            team_key_input,
            creating: false,
            success_message: None,
            _subscriptions: Vec::new(),
        };

        panel.load_teams(cx);
        panel
    }

    /// 加载团队列表
    fn load_teams(&mut self, cx: &mut Context<Self>) {
        if !GlobalCloudUser::is_logged_in(cx) {
            self.loading = false;
            self.error = Some(t!("TeamManagement.not_logged_in").to_string());
            cx.notify();
            return;
        }

        self.loading = true;
        self.error = None;
        cx.notify();

        let client = self.cloud_client.clone();

        cx.spawn(async move |this, cx| {
            match client.list_teams().await {
                Ok(teams) => {
                    this.update(cx, |this, cx| {
                        this.teams = teams;
                        this.loading = false;
                        if this.teams.is_empty() {
                            this.selected_team_idx = None;
                        } else if this.selected_team_idx.is_none() {
                            this.selected_team_idx = Some(0);
                            this.load_members_for_selected(cx);
                        }
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.loading = false;
                        this.error = Some(format!("{}: {}", t!("TeamManagement.load_failed"), e));
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// 加载选中团队的成员
    fn load_members_for_selected(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.selected_team_idx else {
            return;
        };
        let Some(team) = self.teams.get(idx) else {
            return;
        };
        let team_id = team.id.clone();
        let client = self.cloud_client.clone();

        cx.spawn(async move |this, cx| {
            match client.list_team_members(&team_id).await {
                Ok(members) => {
                    this.update(cx, |this, cx| {
                        this.team_members = members;
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.error =
                            Some(format!("{}: {}", t!("TeamManagement.load_members_failed"), e));
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// 创建团队
    fn create_team(&mut self, cx: &mut Context<Self>) {
        let name = self.new_team_name_input.read(cx).text().to_string();
        if name.trim().is_empty() {
            return;
        }

        if !GlobalCloudUser::is_logged_in(cx) {
            self.error = Some(t!("TeamManagement.not_logged_in").to_string());
            cx.notify();
            return;
        }

        let Some(user_id) = self.current_user_id(cx) else {
            self.error = Some(t!("TeamManagement.not_logged_in").to_string());
            cx.notify();
            return;
        };

        if user_id.trim().is_empty() {
            self.error = Some(t!("TeamManagement.not_logged_in").to_string());
            cx.notify();
            return;
        }

        self.creating = true;
        self.error = None;
        cx.notify();

        let desc_text = self.new_team_desc_input.read(cx).text().to_string();
        let description = if desc_text.trim().is_empty() {
            None
        } else {
            Some(desc_text)
        };

        let team = Team {
            id: String::new(), // 由服务端生成
            name,
            owner_id: user_id,
            description,
            key_verification: None,
            key_version: 0,
            created_at: now(),
            updated_at: now(),
        };

        let client = self.cloud_client.clone();

        cx.spawn(async move |this, cx| {
            match client.create_team(&team).await {
                Ok(_team) => {
                    this.update(cx, |this, cx| {
                        this.creating = false;
                        this.success_message =
                            Some(t!("TeamManagement.create_success").to_string());
                        this.load_teams(cx);
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.creating = false;
                        this.error =
                            Some(format!("{}: {}", t!("TeamManagement.create_failed"), e));
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// 添加成员
    fn add_member(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.selected_team_idx else {
            return;
        };
        let Some(team) = self.teams.get(idx) else {
            return;
        };

        let email = self.add_member_email_input.read(cx).text().to_string();
        if email.trim().is_empty() {
            return;
        }

        let team_id = team.id.clone();
        let client = self.cloud_client.clone();

        cx.spawn(async move |this, cx| {
            match client.add_team_member_by_email(&team_id, &email).await {
                Ok(_) => {
                    this.update(cx, |this, cx| {
                        this.success_message =
                            Some(t!("TeamManagement.add_member_success").to_string());
                        this.load_members_for_selected(cx);
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.error =
                            Some(format!("{}: {}", t!("TeamManagement.add_member_failed"), e));
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// 移除成员
    fn remove_member(&mut self, member_id: String, cx: &mut Context<Self>) {
        let client = self.cloud_client.clone();

        cx.spawn(async move |this, cx| {
            match client.remove_team_member(&member_id).await {
                Ok(_) => {
                    this.update(cx, |this, cx| {
                        this.success_message =
                            Some(t!("TeamManagement.remove_member_success").to_string());
                        this.load_members_for_selected(cx);
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.error = Some(format!(
                            "{}: {}",
                            t!("TeamManagement.remove_member_failed"),
                            e
                        ));
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// 解锁团队密钥（本地验证并缓存）
    fn unlock_team_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(idx) = self.selected_team_idx else {
            return;
        };
        let Some(team) = self.teams.get(idx) else {
            return;
        };

        let key = self.team_key_input.read(cx).text().to_string();
        if key.trim().is_empty() {
            return;
        }

        let team_id = team.id.clone();

        if let Ok(mut service) = self.cloud_sync_service.write() {
            service.set_team_key(&team_id, key);
            self.team_key_input.update(cx, |input, cx| {
                input.set_value("", window, cx);
            });
            self.success_message = Some(t!("TeamManagement.key_unlocked").to_string());
            cx.notify();
        }
    }

    /// 获取当前用户 ID
    fn current_user_id(&self, cx: &App) -> Option<String> {
        GlobalCloudUser::get_user(cx).map(|u| u.id)
    }

    /// 判断当前用户是否为选中团队的 owner
    fn is_current_user_owner(&self, cx: &App) -> bool {
        let Some(user_id) = self.current_user_id(cx) else {
            return false;
        };
        let Some(idx) = self.selected_team_idx else {
            return false;
        };
        let Some(team) = self.teams.get(idx) else {
            return false;
        };
        team.owner_id == user_id
    }

    /// 判断团队密钥是否已解锁
    fn is_team_unlocked(&self) -> bool {
        let Some(idx) = self.selected_team_idx else {
            return false;
        };
        let Some(team) = self.teams.get(idx) else {
            return false;
        };
        if let Ok(service) = self.cloud_sync_service.read() {
            service.is_team_unlocked(&team.id)
        } else {
            false
        }
    }

    /// 渲染团队列表（左侧）
    fn render_team_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_idx = self.selected_team_idx;

        v_flex()
            .w(px(220.0))
            .border_r_1()
            .border_color(cx.theme().border)
            .p_2()
            .gap_1()
            .child(
                Label::new(t!("TeamManagement.title"))
                    .text_base()
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                h_flex().gap_1().child(
                    Button::new("refresh_teams")
                        .icon(IconName::Refresh)
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.load_teams(cx);
                        })),
                ),
            )
            .children(self.teams.iter().enumerate().map(|(idx, team)| {
                let is_selected = selected_idx == Some(idx);
                let team_name = team.name.clone();

                div()
                    .id(ElementId::Name(format!("team-{}", idx).into()))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm()
                    .when(is_selected, |this| {
                        this.bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                    })
                    .when(!is_selected, |this| {
                        this.hover(|this| this.bg(cx.theme().muted))
                    })
                    .child(team_name)
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.selected_team_idx = Some(idx);
                        this.load_members_for_selected(cx);
                        cx.notify();
                    }))
            }))
    }

    /// 渲染团队详情（右侧）
    fn render_team_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(idx) = self.selected_team_idx else {
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    Label::new(t!("TeamManagement.select_team"))
                        .text_color(cx.theme().muted_foreground),
                )
                .into_any_element();
        };

        let Some(team) = self.teams.get(idx) else {
            return v_flex().flex_1().into_any_element();
        };

        let is_owner = self.is_current_user_owner(cx);
        let is_unlocked = self.is_team_unlocked();
        let team_name = team.name.clone();
        let team_desc = team.description.clone().unwrap_or_default();

        v_flex()
            .id("team_info")
            .flex_1()
            .p_4()
            .gap_4()
            .overflow_y_scroll()
            // 团队信息
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        Label::new(team_name)
                            .text_lg()
                            .font_weight(FontWeight::BOLD),
                    )
                    .when(!team_desc.is_empty(), |this| {
                        this.child(
                            Label::new(team_desc).text_color(cx.theme().muted_foreground),
                        )
                    })
                    .child(
                        h_flex().gap_2().child(
                            Label::new(if is_owner {
                                t!("TeamManagement.role_owner").to_string()
                            } else {
                                t!("TeamManagement.role_member").to_string()
                            })
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                        ),
                    ),
            )
            // 团队密钥区域
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        Label::new(t!("TeamManagement.team_key"))
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .when(is_unlocked, |this| {
                        this.child(
                            Label::new(t!("TeamManagement.key_unlocked"))
                                .text_sm()
                                .text_color(cx.theme().success),
                        )
                    })
                    .when(!is_unlocked, |this| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Input::new(&self.team_key_input)
                                        .small()
                                        .w(px(200.0)),
                                )
                                .child(
                                    Button::new("unlock_key")
                                        .label(t!("TeamManagement.unlock"))
                                        .small()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.unlock_team_key(window, cx);
                                        })),
                                ),
                        )
                    }),
            )
            // 成员列表
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        Label::new(t!("TeamManagement.members"))
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .children(self.team_members.iter().map(|member| {
                        let member_id = member.id.clone();
                        let role_text = match member.role {
                            TeamRole::Owner => t!("TeamManagement.role_owner").to_string(),
                            TeamRole::Member => t!("TeamManagement.role_member").to_string(),
                        };
                        let is_member_removable = is_owner && member.role != TeamRole::Owner;

                        h_flex()
                            .gap_2()
                            .py_1()
                            .child(Label::new(member.user_id.clone()).text_sm())
                            .child(
                                Label::new(role_text)
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .when(is_member_removable, |this| {
                                this.child(
                                    Button::new(SharedString::from(format!(
                                        "remove-{}",
                                        member_id
                                    )))
                                    .icon(IconName::Close)
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(
                                        move |this, _, _window, cx| {
                                            this.remove_member(member_id.clone(), cx);
                                        },
                                    )),
                                )
                            })
                    }))
                    // 添加成员
                    .when(is_owner, |this| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .mt_2()
                                .child(
                                    Input::new(&self.add_member_email_input)
                                        .small()
                                        .w(px(200.0)),
                                )
                                .child(
                                    Button::new("add_member")
                                        .label(t!("TeamManagement.add_member"))
                                        .small()
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.add_member(cx);
                                        })),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }
}

impl Render for TeamManagementPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error = self.error.clone();
        let success = self.success_message.take();

        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            // 错误/成功提示
            .when_some(error, |this, err| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(hsla(0.0, 0.7, 0.5, 0.1))
                        .child(Label::new(err).text_sm().text_color(cx.theme().danger)),
                )
            })
            .when_some(success, |this, msg| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(hsla(0.33, 0.7, 0.5, 0.1))
                        .child(Label::new(msg).text_sm().text_color(cx.theme().success)),
                )
            })
            // 创建团队区域
            .child(
                h_flex()
                    .px_4()
                    .py_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Input::new(&self.new_team_name_input)
                            .small()
                            .w(px(160.0)),
                    )
                    .child(
                        Input::new(&self.new_team_desc_input)
                            .small()
                            .w(px(200.0)),
                    )
                    .child(
                        Button::new("create_team")
                            .label(t!("TeamManagement.create_team"))
                            .small()
                            .disabled(self.creating)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.create_team(cx);
                            })),
                    ),
            )
            // 主内容区域
            .child(
                h_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_team_list(cx))
                    .child(self.render_team_detail(cx)),
            )
    }
}

impl Focusable for TeamManagementPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabContentEvent> for TeamManagementPanel {}

impl TabContent for TeamManagementPanel {
    fn content_key(&self) -> &'static str {
        "TeamManagement"
    }

    fn title(&self, _cx: &App) -> SharedString {
        SharedString::from(t!("TeamManagement.title"))
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::Building2.into())
    }
}

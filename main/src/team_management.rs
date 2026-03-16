//! 团队管理面板
//!
//! 提供团队 CRUD、成员管理和团队密钥管理功能。

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::label::Label;
use gpui_component::{
    ActiveTheme, Icon, Sizable, WindowExt,
    button::ButtonVariants as _,
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputState},
    v_flex, IconName,
};
use one_core::cloud_sync::{
    CloudApiClient, CloudSyncService, GlobalCloudUser, Team, TeamMember, TeamRole,
};
use one_core::cloud_sync::supabase::SupabaseClient;
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;
use std::sync::Arc;
use one_core::storage::now;
use crate::auth;

/// UUID 截断显示：前 8 位 + "..." + 后 4 位
fn truncate_uuid(uuid: &str) -> String {
    if uuid.len() <= 16 {
        return uuid.to_string();
    }
    format!("{}...{}", &uuid[..8], &uuid[uuid.len() - 4..])
}

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
    /// 云同步服务（用于团队密钥管理）
    cloud_sync_service: Arc<std::sync::RwLock<CloudSyncService>>,
    /// 云端 API 客户端
    cloud_client: Arc<SupabaseClient>,
    /// 添加成员邮箱输入
    add_member_email_input: Entity<InputState>,
    /// 团队密钥输入
    team_key_input: Entity<InputState>,
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

        let add_member_email_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.member_email"))
        });
        let team_key_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.enter_team_key"))
        });

        let mut panel = Self {
            focus_handle,
            teams: Vec::new(),
            selected_team_idx: None,
            team_members: Vec::new(),
            loading: false,
            cloud_sync_service,
            cloud_client,
            add_member_email_input,
            team_key_input,
            _subscriptions: Vec::new(),
        };

        panel.load_teams(cx);
        panel
    }

    /// 加载团队列表
    fn load_teams(&mut self, cx: &mut Context<Self>) {
        if !GlobalCloudUser::is_logged_in(cx) {
            self.loading = false;
            cx.notify();
            return;
        }

        self.loading = true;
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
                    let msg = format!("{}: {}", t!("TeamManagement.load_failed"), e);
                    this.update(cx, |this, cx| {
                        this.loading = false;
                        cx.notify();
                    })
                    .ok();
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
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
                    let msg = format!("{}: {}", t!("TeamManagement.load_members_failed"), e);
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
                }
            }
        })
        .detach();
    }

    /// 打开创建团队 Dialog
    fn open_create_team_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.team_name"))
        });
        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("TeamManagement.team_description"))
        });

        let name_for_ok = name_input.clone();
        let desc_for_ok = desc_input.clone();
        let view = cx.entity().clone();

        window.open_dialog(cx, move |dialog, _, _| {
            let name_ok = name_for_ok.clone();
            let desc_ok = desc_for_ok.clone();
            let view_ok = view.clone();

            dialog
                .title(t!("TeamManagement.create_team").to_string())
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    Label::new(t!("TeamManagement.team_name_label"))
                                        .text_sm(),
                                )
                                .child(Input::new(&name_input)),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    Label::new(t!("TeamManagement.team_desc_label"))
                                        .text_sm(),
                                )
                                .child(Input::new(&desc_input)),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("TeamManagement.create_team")),
                )
                .on_ok(move |_, window, cx| {
                    let name = name_ok.read(cx).value().to_string();
                    if name.trim().is_empty() {
                        window.push_notification(
                            t!("TeamManagement.name_required").to_string(),
                            cx,
                        );
                        return false;
                    }

                    let desc_text = desc_ok.read(cx).value().to_string();
                    let description = if desc_text.trim().is_empty() {
                        None
                    } else {
                        Some(desc_text)
                    };

                    view_ok.update(cx, |this, cx| {
                        this.do_create_team(name, description, cx);
                    });

                    true
                })
        });
    }

    /// 执行创建团队的异步逻辑
    fn do_create_team(
        &mut self,
        name: String,
        description: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !GlobalCloudUser::is_logged_in(cx) {
            return;
        }

        let Some(user_id) = self.current_user_id(cx) else {
            return;
        };

        if user_id.trim().is_empty() {
            return;
        }

        let team = Team {
            id: String::new(),
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
                    let msg = t!("TeamManagement.create_success").to_string();
                    this.update(cx, |this, cx| {
                        this.load_teams(cx);
                    })
                    .ok();
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
                }
                Err(e) => {
                    let msg = format!("{}: {}", t!("TeamManagement.create_failed"), e);
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
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
                    let msg = t!("TeamManagement.add_member_success").to_string();
                    this.update(cx, |this, cx| {
                        this.load_members_for_selected(cx);
                        cx.notify();
                    })
                    .ok();
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
                }
                Err(e) => {
                    let msg = format!("{}: {}", t!("TeamManagement.add_member_failed"), e);
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
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
                    let msg = t!("TeamManagement.remove_member_success").to_string();
                    this.update(cx, |this, cx| {
                        this.load_members_for_selected(cx);
                        cx.notify();
                    })
                    .ok();
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
                }
                Err(e) => {
                    let msg = format!(
                        "{}: {}",
                        t!("TeamManagement.remove_member_failed"),
                        e
                    );
                    let _ = cx.update(|cx| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                window.push_notification(msg, cx);
                            });
                        }
                    });
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
            window.push_notification(
                t!("TeamManagement.key_unlocked").to_string(),
                cx,
            );
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

    /// 渲染团队列表（左侧面板）
    fn render_team_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_idx = self.selected_team_idx;

        v_flex()
            .w(px(240.0))
            .h_full()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                // 头部：标题 + 刷新 + 创建按钮
                h_flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Label::new(t!("TeamManagement.title"))
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("refresh_teams")
                                    .icon(IconName::Refresh)
                                    .xsmall()
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.load_teams(cx);
                                    })),
                            )
                            .child(
                                Button::new("create_team_btn")
                                    .icon(IconName::Plus)
                                    .xsmall()
                                    .ghost()
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            this.open_create_team_dialog(window, cx);
                                        },
                                    )),
                            ),
                    ),
            )
            .child({
                // 团队列表内容
                let mut list = v_flex()
                    .id("team_list_content")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_1()
                    .gap_0p5();

                if self.loading {
                    list = list.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .py_8()
                            .child(
                                Label::new(t!("TeamManagement.loading"))
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    );
                } else if self.teams.is_empty() {
                    list = list.child(
                        v_flex()
                            .items_center()
                            .justify_center()
                            .py_8()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Building2)
                                    .size_6()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                Label::new(t!("TeamManagement.no_teams"))
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                Button::new("create_team_empty")
                                    .label(t!("TeamManagement.create_team"))
                                    .small()
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            this.open_create_team_dialog(window, cx);
                                        },
                                    )),
                            ),
                    );
                } else {
                    list = list.children(self.teams.iter().enumerate().map(|(idx, team)| {
                        let is_selected = selected_idx == Some(idx);
                        let team_name = team.name.clone();
                        let team_desc = team
                            .description
                            .clone()
                            .unwrap_or_default();

                        div()
                            .id(ElementId::Name(format!("team-{}", idx).into()))
                            .px_2()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_selected, |this| {
                                this.bg(cx.theme().accent)
                                    .text_color(cx.theme().accent_foreground)
                            })
                            .when(!is_selected, |this| {
                                this.hover(|this| this.bg(cx.theme().muted))
                            })
                            .child(
                                v_flex()
                                    .gap_0p5()
                                    .child(
                                        Label::new(team_name)
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM),
                                    )
                                    .when(!team_desc.is_empty(), |this| {
                                        this.child(
                                            div()
                                                .text_xs()
                                                .text_color(
                                                    if is_selected {
                                                        cx.theme().accent_foreground
                                                    } else {
                                                        cx.theme().muted_foreground
                                                    },
                                                )
                                                .truncate()
                                                .child(team_desc),
                                        )
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.selected_team_idx = Some(idx);
                                this.load_members_for_selected(cx);
                                cx.notify();
                            }))
                    }));
                }

                list
            })
    }

    /// 渲染团队详情（右侧面板）
    fn render_team_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(idx) = self.selected_team_idx else {
            // 空状态：未选中团队
            return v_flex()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    Icon::new(IconName::Building2)
                        .size_8()
                        .text_color(cx.theme().muted_foreground),
                )
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
        let member_count = self.team_members.len();

        v_flex()
            .id("team_detail")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .p_4()
            .gap_4()
            // 头部：团队名称 + 角色徽章 + 描述
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Label::new(team_name)
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .text_xs()
                                    .when(is_owner, |this| {
                                        this.bg(cx.theme().accent)
                                            .text_color(cx.theme().accent_foreground)
                                    })
                                    .when(!is_owner, |this| {
                                        this.bg(cx.theme().muted)
                                            .text_color(cx.theme().muted_foreground)
                                    })
                                    .child(if is_owner {
                                        t!("TeamManagement.role_owner").to_string()
                                    } else {
                                        t!("TeamManagement.role_member").to_string()
                                    }),
                            ),
                    )
                    .when(!team_desc.is_empty(), |this| {
                        this.child(
                            Label::new(team_desc)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
            // 团队密钥卡片
            .child(
                v_flex()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_3()
                    .gap_2()
                    .child(
                        Label::new(t!("TeamManagement.team_key"))
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .when(is_unlocked, |this| {
                        this.child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Icon::new(IconName::CircleCheck)
                                        .size_4()
                                        .text_color(cx.theme().success),
                                )
                                .child(
                                    Label::new(t!("TeamManagement.key_unlocked"))
                                        .text_sm()
                                        .text_color(cx.theme().success),
                                ),
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
            // 成员列表卡片
            .child(
                v_flex()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_3()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Label::new(t!("TeamManagement.members"))
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(10.0))
                                    .bg(cx.theme().muted)
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{}", member_count)),
                            ),
                    )
                    .children(self.team_members.iter().map(|member| {
                        let member_id = member.id.clone();
                        let display_id = truncate_uuid(&member.user_id);
                        let role_text = match member.role {
                            TeamRole::Owner => t!("TeamManagement.role_owner").to_string(),
                            TeamRole::Member => t!("TeamManagement.role_member").to_string(),
                        };
                        let is_role_owner = member.role == TeamRole::Owner;
                        let is_member_removable = is_owner && !is_role_owner;

                        h_flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Label::new(display_id)
                                            .text_sm(),
                                    )
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded_md()
                                            .text_xs()
                                            .when(is_role_owner, |this| {
                                                this.bg(cx.theme().accent)
                                                    .text_color(cx.theme().accent_foreground)
                                            })
                                            .when(!is_role_owner, |this| {
                                                this.bg(cx.theme().muted)
                                                    .text_color(cx.theme().muted_foreground)
                                            })
                                            .child(role_text),
                                    ),
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
                    // 添加成员区域
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
        h_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(self.render_team_list(cx))
            .child(self.render_team_detail(cx))
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

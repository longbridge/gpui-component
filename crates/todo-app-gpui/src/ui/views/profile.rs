use gpui::prelude::*;
use gpui::*;

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    dock::PanelControl,
    dropdown::{Dropdown, DropdownState},
    input::{InputEvent, InputState, TextInput},
    notification::NotificationType,
    text::TextView,
    tooltip::Tooltip,
    *,
};

use crate::{app::AppState, models::profile_config::ProfileData, ui::components::ViewKit};

actions!(
    profile,
    [Tab, TabPrev, Save, Reset, AuthorizeFeishu, AnalyzeBio]
);

const CONTEXT: &str = "Profile";

pub struct Profile {
    focus_handle: gpui::FocusHandle,

    // 个人信息字段
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
    phone_input: Entity<InputState>,
    bio_input: Entity<InputState>,

    // 设置字段
    department_input: Entity<InputState>,

    // 偏好设置 - 修正类型
    theme_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    language_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // AI分析相关
    auto_analyze_bio: bool,

    _subscriptions: Vec<Subscription>,
}

impl Profile {
    pub fn init(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
            KeyBinding::new("tab", Tab, Some(CONTEXT)),
            KeyBinding::new("ctrl-s", Save, Some(CONTEXT)),
            KeyBinding::new("ctrl-r", Reset, Some(CONTEXT)),
        ])
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入您的姓名")
                .default_value(AppState::state(cx).profile_manager.profile.name.clone())
        });

        let email_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入邮箱地址")
                // .pattern(
                //     regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(),
                // )
                // .validate(|s| s.contains("@"))
                .default_value(AppState::state(cx).profile_manager.profile.email.clone())
        });

        let phone_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入手机号码")
                // .mask_pattern("999-9999-9999")
                .default_value(AppState::state(cx).profile_manager.profile.phone.clone())
        });

        let bio_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("简单介绍一下自己...")
                .auto_grow(6, 15)
                .default_value(AppState::state(cx).profile_manager.profile.bio.clone())
        });

        let department_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("部门")
                // .pattern(regex::Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap())
                .default_value(
                    AppState::state(cx)
                        .profile_manager
                        .profile
                        .department
                        .clone(),
                )
        });

        // 创建主题下拉框 - 参考 dropdown_story.rs 的方式
        let theme_dropdown = cx.new(|cx| {
            DropdownState::new(
                vec!["系统".into(), "明亮".into(), "暗黑".into()],
                Some(0), // 默认选择第一项
                window,
                cx,
            )
        });

        // 创建语言下拉框 - 参考 dropdown_story.rs 的方式
        let language_dropdown = cx.new(|cx| {
            DropdownState::new(
                vec!["中文".into(), "English".into()],
                Some(0), // 默认选择第一项
                window,
                cx,
            )
        });

        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&email_input, window, Self::on_input_event),
            cx.subscribe_in(&phone_input, window, Self::on_input_event),
            cx.subscribe_in(&department_input, window, Self::on_input_event),
            cx.subscribe_in(&bio_input, window, Self::on_bio_input_event),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            name_input,
            email_input,
            phone_input,
            bio_input,
            department_input,
            theme_dropdown,
            language_dropdown,
            auto_analyze_bio: AppState::state(cx).profile_manager.profile.auto_analyze_bio,
            _subscriptions,
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn save(&mut self, _: &Save, _window: &mut Window, cx: &mut Context<Self>) {
        // 收集所有数据
        let profile_data = ProfileData {
            name: self.name_input.read(cx).value().to_string(),
            email: self.email_input.read(cx).value().to_string(),
            phone: self.phone_input.read(cx).unmask_value().to_string(),
            bio: self.bio_input.read(cx).value().to_string(),
            department: self.department_input.read(cx).value().to_string(),
            theme: self
                .theme_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            language: self
                .language_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            auto_analyze_bio: self.auto_analyze_bio,
        };
        match AppState::state_mut(cx)
            .profile_manager
            .update_profile(profile_data)
            .save()
        {
            Ok(_) => {
                _window.push_notification((NotificationType::Success, "个人档案保存成功"), cx);
            }
            Err(err) => {
                _window.push_notification(
                    (
                        NotificationType::Error,
                        SharedString::new(format!("个人档案保存失败-{}", err)),
                    ),
                    cx,
                );
            }
        }
    }

    fn reset(&mut self, _: &Reset, _window: &mut Window, cx: &mut Context<Self>) {
        // 重置所有输入字段
        self.name_input.update(cx, |state, cx| {
            *state = InputState::new(_window, cx).default_value("用户");
        });

        self.email_input.update(cx, |state, cx| {
            *state = InputState::new(_window, cx).placeholder("请输入邮箱地址");
        });

        self.phone_input.update(cx, |state, cx| {
            *state = InputState::new(_window, cx)
                .placeholder("请输入手机号码")
                .mask_pattern("(999) 999-9999");
        });

        self.bio_input.update(cx, |state, cx| {
            *state = InputState::new(_window, cx)
                .placeholder("简单介绍一下自己...")
                .auto_grow(3, 6);
        });

        self.department_input.update(cx, |state, cx| {
            *state = InputState::new(_window, cx).placeholder("部门");
        });

        // 重置下拉框选择到第一项
        self.theme_dropdown.update(cx, |state, cx| {
            state.set_selected_index(Some(0), _window, cx);
        });

        self.language_dropdown.update(cx, |state, cx| {
            state.set_selected_index(Some(0), _window, cx);
        });

        cx.notify();
        println!("已重置个人资料");
    }

    fn authorize_feishu(
        &mut self,
        _: &AuthorizeFeishu,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // TODO: 实现飞书授权逻辑
        println!("开始飞书授权...");
        cx.notify();
    }

    fn analyze_bio(&mut self, _: &AnalyzeBio, _window: &mut Window, cx: &mut Context<Self>) {
        let bio_text = self.bio_input.read(cx).value().to_string();

        if bio_text.trim().is_empty() {
            println!("请先填写个人简介");
            return;
        }

        println!("开始AI分析个人简介: {}", bio_text);
        println!("分析内容包括：人物特征、背景信息、职业倾向、兴趣爱好等");

        cx.notify();
    }

    fn toggle_auto_analyze(&mut self, checked: bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.auto_analyze_bio = checked;

        // 如果启用自动分析，立即分析当前内容
        if checked {
            self.analyze_bio(&AnalyzeBio, _window, cx);
        }

        cx.notify();
    }

    fn on_bio_input_event(
        &mut self,
        _entity: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change(_text) => {
                // 如果启用自动分析，在输入变化时触发分析
                if self.auto_analyze_bio {
                    self.analyze_bio(&AnalyzeBio, window, cx);
                }
            }
            _ => {}
        };
    }

    fn on_input_event(
        &mut self,
        entity: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change(text) => {
                if entity == &self.email_input {
                    let is_valid = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$")
                        .unwrap()
                        .is_match(text);
                    if !text.is_empty() && !is_valid {
                        println!("邮箱格式不正确");
                    }
                }
            }
            InputEvent::PressEnter { .. } => {
                self.save(&Save, _window, cx);
            }
            _ => {}
        };
    }

    fn section_title(title: &'static str) -> impl gpui::IntoElement {
        gpui::div()
            .text_lg()
            .font_semibold()
            .text_color(gpui::rgb(0x374151))
            .pb_2()
            .child(title)
    }

    fn v_form_field(label: &'static str, input: impl gpui::IntoElement) -> impl gpui::IntoElement {
        v_flex()
            .gap_1()
            .min_w_48()
            .child(
                gpui::div()
                    .text_sm()
                    .text_color(gpui::rgb(0x6B7280))
                    .child(label),
            )
            .child(input)
    }

    fn h_form_field(label: &'static str, input: impl gpui::IntoElement) -> impl gpui::IntoElement {
        h_flex()
            .gap_1()
            .min_w_48()
            .child(
                gpui::div()
                    .text_sm()
                    .text_color(gpui::rgb(0x6B7280))
                    .child(label),
            )
            .child(input)
    }
}

impl ViewKit for Profile {
    fn title() -> &'static str {
        "个人资料"
    }

    fn description() -> &'static str {
        "设置您的个人资料和偏好，有助于个性化您的体验"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl FocusableCycle for Profile {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<gpui::FocusHandle> {
        vec![
            self.name_input.focus_handle(cx),
            self.email_input.focus_handle(cx),
            self.phone_input.focus_handle(cx),
            self.bio_input.focus_handle(cx),
            self.department_input.focus_handle(cx),
            self.theme_dropdown.focus_handle(cx),
            self.language_dropdown.focus_handle(cx),
        ]
    }
}

impl Focusable for Profile {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Profile {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("profile-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::reset))
            .on_action(cx.listener(Self::authorize_feishu))
            .on_action(cx.listener(Self::analyze_bio))
            .size_full()
            .gap_2()
            .child(
                // 基本信息
                v_flex()
                    .gap_3()
                    .p_4()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(Self::section_title("基本信息"))
                    .child(
                        // 飞书集成行 - 左对齐布局
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .min_w_24()
                                    .child("飞书集成"),
                            )
                            .child(
                                Button::new("feishu-auth-btn")
                                    .with_variant(ButtonVariant::Secondary)
                                    .label("授权")
                                    .icon(IconName::ExternalLink)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.authorize_feishu(&AuthorizeFeishu, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        // 姓名行 - 左对齐布局
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .min_w_24()
                                    .child("姓名 *"),
                            )
                            .child(
                                div().flex_1().max_w_80().child(
                                    TextInput::new(&self.name_input)
                                        .cleanable()
                                        .prefix(Icon::new(IconName::CircleUser).small().ml_3()),
                                ),
                            ),
                    )
                    .child(
                        // 邮箱地址行 - 左对齐布局
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .min_w_24()
                                    .child("邮箱地址 *"),
                            )
                            .child(
                                div().flex_1().max_w_80().child(
                                    TextInput::new(&self.email_input)
                                        .cleanable()
                                        .prefix(Icon::new(IconName::Mail).small().ml_3()),
                                ),
                            ),
                    )
                    .child(
                        // 手机号码行 - 左对齐布局
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .min_w_24()
                                    .child("手机号码"),
                            )
                            .child(
                                div().flex_1().max_w_80().child(
                                    TextInput::new(&self.phone_input)
                                        .cleanable()
                                        .prefix(Icon::new(IconName::Phone).small().ml_3()),
                                ),
                            ),
                    )
                    .child(
                        // 手机号码行 - 左对齐布局
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .min_w_24()
                                    .child("部门"),
                            )
                            .child(
                                div().flex_1().max_w_80().child(
                                    TextInput::new(&self.department_input)
                                        .cleanable()
                                        .prefix(Icon::new(IconName::Users).small().ml_3()),
                                ),
                            ),
                    )
                    .child(
                        // 个人简介 - 垂直布局（因为是多行文本）
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child("个人简介"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .child(TextInput::new(&self.bio_input).cleanable()),
                            )
                            .child(
                                // AI分析控制区域
                                h_flex()
                                    .pt_2()
                                    .gap_3()
                                    .justify_start()
                                    .items_center()
                                    .child(
                                        Checkbox::new("auto-analyze-bio")
                                            .checked(self.auto_analyze_bio)
                                            .label("开启洞察")
                                            .with_size(gpui_component::Size::Small)
                                            .tooltip(|win, cx| {
                                                Tooltip::element(|_, cx| {
                                                    h_flex().gap_x_1().child(
                                                        div()
                                                            .text_size(rems(0.6))
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(TextView::markdown(
                                                                "character",
                                                                include_str!("character.md"),
                                                            )),
                                                    )
                                                })
                                                .build(win, cx)
                                            })
                                            .on_click(cx.listener(|this, checked, window, cx| {
                                                this.toggle_auto_analyze(*checked, window, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                // 偏好设置
                v_flex()
                    .gap_2()
                    .p_4()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(Self::section_title("偏好设置"))
                    .child(
                        h_flex()
                            .gap_6()
                            .child(Self::v_form_field(
                                "主题偏好",
                                Dropdown::new(&self.theme_dropdown)
                                    .placeholder("选择主题")
                                    .small(),
                            ))
                            .child(Self::v_form_field(
                                "语言偏好",
                                Dropdown::new(&self.language_dropdown)
                                    .placeholder("选择语言")
                                    .small(),
                            )),
                    ),
            )
            .child(
                // 操作按钮
                h_flex()
                    .justify_center()
                    .gap_3()
                    .pt_4()
                    .child(
                        Button::new("reset-btn")
                            .label("重置")
                            .icon(IconName::RefreshCW)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.reset(&Reset, window, cx)),
                            ),
                    )
                    .child(
                        Button::new("save-btn")
                            .with_variant(ButtonVariant::Primary)
                            .label("保存")
                            .icon(IconName::Check)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.save(&Save, window, cx)),
                            ),
                    ),
            )
    }
}

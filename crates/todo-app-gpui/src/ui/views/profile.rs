use gpui::prelude::*;
use gpui::*;

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    dock::PanelControl,
    h_flex,
    input::{InputEvent, InputState, TextInput},
    text::TextView,
    v_flex, FocusableCycle, Icon, IconName, Sizable, StyledExt,
};

use crate::ui::components::ViewKit;

actions!(profile, [Tab, TabPrev, Save, Reset, AuthorizeFeishu]);

const CONTEXT: &str = "Profile";

pub struct Profile {
    focus_handle: gpui::FocusHandle,

    // 个人信息字段
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
    phone_input: Entity<InputState>,
    bio_input: Entity<InputState>,

    // 设置字段
    username_input: Entity<InputState>,

    // 偏好设置
    theme_preference: SharedString,
    language_preference: SharedString,

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
            InputState::new(window, cx).placeholder("请输入您的姓名")
            // .default_value("用户")
        });

        let email_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入邮箱地址")
                .pattern(regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap())
        });

        let phone_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入手机号码")
                .mask_pattern("(999) 999-9999-9999")
        });

        let bio_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("简单介绍一下自己...")
                .auto_grow(3, 6)
        });

        let username_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("用户名")
                .pattern(regex::Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap())
        });

        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&email_input, window, Self::on_input_event),
            cx.subscribe_in(&phone_input, window, Self::on_input_event),
            cx.subscribe_in(&username_input, window, Self::on_input_event),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            name_input,
            email_input,
            phone_input,
            bio_input,
            username_input,
            theme_preference: "系统".into(),
            language_preference: "中文".into(),
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
            username: self.username_input.read(cx).value().to_string(),
            theme: self.theme_preference.to_string(),
            language: self.language_preference.to_string(),
        };

        println!("保存个人资料: {:?}", profile_data);
    }

    fn reset(&mut self, _: &Reset, _window: &mut Window, cx: &mut Context<Self>) {
        // 重置所有输入字段 - 参考 form_story.rs，直接设置新的值
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

        self.username_input.update(cx, |state, cx| {
            *state = InputState::new(_window, cx).placeholder("用户名");
        });

        self.theme_preference = "系统".into();
        self.language_preference = "中文".into();

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
            .min_w_48() // 添加最小宽度
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
            .min_w_48() // 添加最小宽度
            .child(
                gpui::div()
                    .text_sm()
                    .text_color(gpui::rgb(0x6B7280))
                    .child(label),
            )
            .child(input)
    }
}

#[derive(Debug)]
struct ProfileData {
    name: String,
    email: String,
    phone: String,
    bio: String,
    username: String,
    theme: String,
    language: String,
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
            self.username_input.focus_handle(cx),
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
        // 先克隆需要在 UI 中显示的值
        let theme_preference = self.theme_preference.clone();
        let language_preference = self.language_preference.clone();

        v_flex()
            .key_context(CONTEXT)
            .id("profile-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::reset))
            .on_action(cx.listener(Self::authorize_feishu))
            .size_full()
            // .p_2()
            .gap_2()
            .child(
                // 基本信息
                v_flex()
                    .gap_2()
                    .p_2()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(Self::section_title("基本信息"))
                    .child(
                        // 飞书授权按钮行
                        h_flex()
                            .gap_2()
                            .justify_start()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
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
                        h_flex().gap_2().child(Self::h_form_field(
                            "姓名 *",
                            TextInput::new(&self.name_input)
                                .cleanable()
                                .prefix(Icon::new(IconName::CircleUser).small().ml_3()),
                        )),
                    )
                    .child(Self::h_form_field(
                        "邮箱地址 *",
                        TextInput::new(&self.email_input)
                            .cleanable()
                            .prefix(Icon::new(IconName::Mail).small().ml_3()),
                    ))
                    .child(Self::h_form_field(
                        "手机号码",
                        TextInput::new(&self.phone_input)
                            .cleanable()
                            .prefix(Icon::new(IconName::Phone).small().ml_3()),
                    ))
                    .child(Self::v_form_field(
                        "个人简介",
                        TextInput::new(&self.bio_input).cleanable(),
                    )),
            )
            .child(
                // 偏好设置
                v_flex()
                    .gap_2()
                    .p_2()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(Self::section_title("偏好设置"))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .child("主题偏好"),
                                    )
                                    .child(
                                        div()
                                            .p_2()
                                            .border_1()
                                            .border_color(gpui::rgb(0xD1D5DB))
                                            .rounded_md()
                                            .child(theme_preference), // 使用克隆的值
                                    )
                                    .flex_1(),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        gpui::div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .child("语言偏好"),
                                    )
                                    .child(
                                        gpui::div()
                                            .p_2()
                                            .border_1()
                                            .border_color(gpui::rgb(0xD1D5DB))
                                            .rounded_md()
                                            .child(language_preference), // 使用克隆的值
                                    )
                                    .flex_1(),
                            ),
                    ),
            )
            .child(
                // 操作按钮
                h_flex()
                    .justify_end()
                    .gap_3()
                    .pt_4()
                    .border_t_1()
                    .border_color(gpui::rgb(0xE5E7EB))
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
                            .label("保存更改")
                            .icon(IconName::Check)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.save(&Save, window, cx)),
                            ),
                    ),
            )
    }
}

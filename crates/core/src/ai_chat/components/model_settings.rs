//! 模型设置 - AI 模型参数配置
//!
//! 包含温度、历史记录数量等模型相关参数的配置。
//! 可在不同的聊天面板中复用。

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, h_flex,
    input::{Input, InputEvent, InputState},
    slider::{Slider, SliderEvent, SliderState},
    v_flex,
};
use rust_i18n::t;

// ============================================================================
// 模型设置数据结构
// ============================================================================

/// 模型设置
///
/// 包含 AI 模型的各种参数配置，如温度、历史记录数量等。
#[derive(Clone, Debug)]
pub struct ModelSettings {
    /// 温度参数 (0.0 - 2.0)
    /// 控制输出的随机性，值越高输出越随机
    pub temperature: f32,
    /// 历史记录数量（多轮对话时携带的历史消息数）
    pub history_count: usize,
    /// 最大输出 token 数
    pub max_tokens: usize,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            history_count: 10,
            max_tokens: 2000,
        }
    }
}

impl ModelSettings {
    /// 创建新的模型设置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// 设置历史记录数量
    pub fn with_history_count(mut self, count: usize) -> Self {
        self.history_count = count.min(50);
        self
    }

    /// 设置最大输出 token 数
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens.clamp(100, 8000);
        self
    }
}

// ============================================================================
// 事件定义
// ============================================================================

/// 模型设置事件
#[derive(Clone, Debug)]
pub enum ModelSettingsEvent {
    /// 设置已更改
    Changed(ModelSettings),
}

// ============================================================================
// 模型设置面板组件
// ============================================================================

/// 模型设置面板
///
/// 提供温度、历史记录数量、最大 token 数等参数的 UI 配置界面。
pub struct ModelSettingsPanel {
    focus_handle: FocusHandle,
    settings: ModelSettings,

    // 温度滑块
    temperature_slider: Entity<SliderState>,

    // 历史记录输入
    history_input: Entity<InputState>,

    // 最大 token 输入
    max_tokens_input: Entity<InputState>,

    // 标签文本
    labels: ModelSettingsLabels,
}

/// 模型设置面板标签
#[derive(Clone, Debug)]
pub struct ModelSettingsLabels {
    pub title: String,
    pub temperature_label: String,
    pub temperature_desc: String,
    pub history_label: String,
    pub history_desc: String,
    pub max_tokens_label: String,
    pub max_tokens_desc: String,
    pub footer_notice: String,
}

impl Default for ModelSettingsLabels {
    fn default() -> Self {
        Self {
            title: t!("AiChat.model_settings_title").to_string(),
            temperature_label: t!("AiChat.temperature_label").to_string(),
            temperature_desc: t!("AiChat.temperature_desc").to_string(),
            history_label: t!("AiChat.history_label").to_string(),
            history_desc: t!("AiChat.history_desc").to_string(),
            max_tokens_label: t!("AiChat.max_tokens_label").to_string(),
            max_tokens_desc: t!("AiChat.max_tokens_desc").to_string(),
            footer_notice: t!("AiChat.settings_footer_notice").to_string(),
        }
    }
}

impl ModelSettingsPanel {
    /// 创建新的模型设置面板
    pub fn new(settings: ModelSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_labels(settings, ModelSettingsLabels::default(), window, cx)
    }

    /// 使用自定义标签创建模型设置面板
    pub fn with_labels(
        settings: ModelSettings,
        labels: ModelSettingsLabels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        // 创建温度滑块
        let temperature_slider = cx.new(|_cx| {
            SliderState::new()
                .min(0.0)
                .max(2.0)
                .step(0.1)
                .default_value(settings.temperature)
        });

        // 订阅温度滑块事件
        cx.subscribe_in(
            &temperature_slider,
            window,
            |this, _, event, _window, cx| {
                let SliderEvent::Change(value) = event;
                if let gpui_component::slider::SliderValue::Single(v) = value {
                    this.settings.temperature = *v;
                    this.emit_change(cx);
                }
            },
        )
        .detach();

        // 创建历史记录输入
        let history_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(settings.history_count.to_string())
        });

        // 订阅历史记录输入事件
        cx.subscribe_in(&history_input, window, |this, input, event, _window, cx| {
            if let InputEvent::Change = event {
                let text = input.read(cx).text().to_string();
                if let Ok(count) = text.parse::<usize>() {
                    this.settings.history_count = count.min(50);
                    this.emit_change(cx);
                }
            }
        })
        .detach();

        // 创建最大 token 输入
        let max_tokens_input =
            cx.new(|cx| InputState::new(window, cx).default_value(settings.max_tokens.to_string()));

        // 订阅最大 token 输入事件
        cx.subscribe_in(
            &max_tokens_input,
            window,
            |this, input, event, _window, cx| {
                if let InputEvent::Change = event {
                    let text = input.read(cx).text().to_string();
                    if let Ok(tokens) = text.parse::<usize>() {
                        this.settings.max_tokens = tokens.clamp(100, 8000);
                        this.emit_change(cx);
                    }
                }
            },
        )
        .detach();

        Self {
            focus_handle,
            settings,
            temperature_slider,
            history_input,
            max_tokens_input,
            labels,
        }
    }

    /// 获取当前设置
    pub fn settings(&self) -> &ModelSettings {
        &self.settings
    }

    /// 更新设置
    pub fn update_settings(
        &mut self,
        settings: ModelSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings = settings.clone();

        // 更新温度滑块
        self.temperature_slider.update(cx, |state, cx| {
            state.set_value(settings.temperature, window, cx);
        });

        // 更新历史记录输入
        self.history_input.update(cx, |input, cx| {
            input.set_value(settings.history_count.to_string(), window, cx);
        });

        // 更新最大 token 输入
        self.max_tokens_input.update(cx, |input, cx| {
            input.set_value(settings.max_tokens.to_string(), window, cx);
        });

        cx.notify();
    }

    fn emit_change(&self, cx: &mut Context<Self>) {
        cx.emit(ModelSettingsEvent::Changed(self.settings.clone()));
    }

    fn render_setting_row(
        &self,
        label: &str,
        description: &str,
        content: impl IntoElement,
        cx: &App,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap_4()
            .py_2()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_x_hidden()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .truncate()
                            .child(label.to_string()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate()
                            .child(description.to_string()),
                    ),
            )
            .child(div().flex_shrink_0().child(content))
    }
}

impl EventEmitter<ModelSettingsEvent> for ModelSettingsPanel {}

impl Focusable for ModelSettingsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ModelSettingsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;

        v_flex()
            .w(px(320.0))
            .gap_1()
            // 标题
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .pb_2()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        Icon::new(IconName::Settings)
                            .with_size(Size::Small)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(self.labels.title.clone()),
                    ),
            )
            // 温度设置
            .child(
                self.render_setting_row(
                    &self.labels.temperature_label,
                    &self.labels.temperature_desc,
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(100.0))
                                .child(Slider::new(&self.temperature_slider)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .min_w(px(32.0))
                                .child(format!("{:.1}", self.settings.temperature)),
                        ),
                    cx,
                ),
            )
            // 历史记录数量
            .child(
                self.render_setting_row(
                    &self.labels.history_label,
                    &self.labels.history_desc,
                    div()
                        .w(px(80.0))
                        .child(Input::new(&self.history_input).with_size(Size::Small)),
                    cx,
                ),
            )
            // 最大 token 数
            .child(
                self.render_setting_row(
                    &self.labels.max_tokens_label,
                    &self.labels.max_tokens_desc,
                    div()
                        .w(px(80.0))
                        .child(Input::new(&self.max_tokens_input).with_size(Size::Small)),
                    cx,
                ),
            )
            // 提示信息
            .child(
                div()
                    .w_full()
                    .pt_2()
                    .mt_1()
                    .border_t_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.labels.footer_notice.clone()),
                    ),
            )
    }
}

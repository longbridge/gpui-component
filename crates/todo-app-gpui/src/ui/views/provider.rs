use crate::ui::components::{section::section, ViewKit};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonGroup, ButtonVariant, ButtonVariants as _},
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    switch::Switch,
    tooltip::Tooltip,
    v_flex, ContextModal, FocusableCycle, Icon, IconName, Sizable, StyledExt,
};

actions!(
    provider,
    [Tab, TabPrev, AddProvider, SaveProvider, DeleteProvider]
);

const CONTEXT: &str = "LlmProvider";

#[derive(Debug, Clone)]
pub enum ApiType {
    OpenAI,
    OpenAIResponse,
    Gemini,
    Anthropic,
    AzureOpenAI,
}

impl ApiType {
    fn as_str(&self) -> &'static str {
        match self {
            ApiType::OpenAI => "OpenAI",
            ApiType::OpenAIResponse => "OpenAI-Response",
            ApiType::Gemini => "Gemini",
            ApiType::Anthropic => "Anthropic",
            ApiType::AzureOpenAI => "Azure-OpenAI",
        }
    }

    fn all() -> Vec<SharedString> {
        vec![
            "OpenAI".into(),
            "OpenAI-Response".into(),
            "Gemini".into(),
            "Anthropic".into(),
            "Azure-OpenAI".into(),
        ]
    }
}

#[derive(Debug, Clone)]
pub enum ModelCapability {
    Text,
    Vision,
    Audio,
    Tools,
}

impl ModelCapability {
    fn icon(&self) -> IconName {
        match self {
            ModelCapability::Text => IconName::LetterText,
            ModelCapability::Vision => IconName::Eye,
            ModelCapability::Audio => IconName::Mic,
            ModelCapability::Tools => IconName::Wrench, // 修正拼写错误
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ModelCapability::Text => "文本",
            ModelCapability::Vision => "视觉",
            ModelCapability::Audio => "音频",
            ModelCapability::Tools => "工具",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    name: String,
    capabilities: Vec<ModelCapability>,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub struct LlmProviderInfo {
    id: String,
    name: String,
    api_url: String,
    api_key: String,
    api_type: ApiType,
    enabled: bool,
    models: Vec<ModelInfo>,
}

impl Default for LlmProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            api_url: String::new(),
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            enabled: true,
            models: vec![
                ModelInfo {
                    name: "gpt-4o".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                    ],
                    enabled: true,
                },
                ModelInfo {
                    name: "gpt-4o-mini".to_string(),
                    capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    enabled: true,
                },
            ],
        }
    }
}

pub struct LlmProvider {
    focus_handle: FocusHandle,
    providers: Vec<LlmProviderInfo>,
    expanded_providers: Vec<usize>,

    // 编辑表单字段
    editing_provider: Option<usize>,
    name_input: Entity<InputState>,
    api_url_input: Entity<InputState>,
    api_key_input: Entity<InputState>,
    api_type_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    _subscriptions: Vec<Subscription>,
}

impl ViewKit for LlmProvider {
    fn title() -> &'static str {
        "LLM服务提供商"
    }

    fn description() -> &'static str {
        "配置和管理LLM服务提供商"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl LlmProvider {
    pub fn init(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
            KeyBinding::new("tab", Tab, Some(CONTEXT)),
            KeyBinding::new("ctrl-n", AddProvider, Some(CONTEXT)),
        ])
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("服务提供商名称"));

        let api_url_input = cx.new(|cx| InputState::new(window, cx).placeholder("API 地址"));

        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("API 密钥")
                .masked(true)
        });

        let api_type_dropdown =
            cx.new(|cx| DropdownState::new(ApiType::all(), Some(0), window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&api_url_input, window, Self::on_input_event),
            cx.subscribe_in(&api_key_input, window, Self::on_input_event),
        ];

        // 初始化一些示例数据
        let mut default_provider = LlmProviderInfo::default();
        default_provider.name = "OpenAI".to_string();
        default_provider.api_url = "https://api.openai.com/v1".to_string();

        let mut anthropic_provider = LlmProviderInfo::default();
        anthropic_provider.name = "Anthropic".to_string();
        anthropic_provider.api_url = "https://api.anthropic.com".to_string();
        anthropic_provider.api_type = ApiType::Anthropic;
        anthropic_provider.models = vec![
            ModelInfo {
                name: "claude-3.5-sonnet".to_string(),
                capabilities: vec![
                    ModelCapability::Text,
                    ModelCapability::Vision,
                    ModelCapability::Tools,
                ],
                enabled: true,
            },
            ModelInfo {
                name: "claude-3-haiku".to_string(),
                capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                enabled: true,
            },
        ];

        Self {
            focus_handle: cx.focus_handle(),
            providers: vec![default_provider, anthropic_provider],
            expanded_providers: vec![0],
            editing_provider: None,
            name_input,
            api_url_input,
            api_key_input,
            api_type_dropdown,
            _subscriptions,
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn add_provider(&mut self, _: &AddProvider, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_provider = Some(self.providers.len());
        self.providers.push(LlmProviderInfo::default());
        self.clear_form(window, cx);
        cx.notify();
    }

    fn save_provider(&mut self, _: &SaveProvider, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.editing_provider {
            if let Some(provider) = self.providers.get_mut(index) {
                provider.name = self.name_input.read(cx).value().to_string();
                provider.api_url = self.api_url_input.read(cx).value().to_string();
                provider.api_key = self.api_key_input.read(cx).value().to_string();

                if let Some(selected) = self.api_type_dropdown.read(cx).selected_value() {
                    provider.api_type = match selected.as_ref() {
                        "OpenAI" => ApiType::OpenAI,
                        "OpenAI-Response" => ApiType::OpenAIResponse,
                        "Gemini" => ApiType::Gemini,
                        "Anthropic" => ApiType::Anthropic,
                        "Azure-OpenAI" => ApiType::AzureOpenAI,
                        _ => ApiType::OpenAI,
                    };
                }
            }
        }

        self.editing_provider = None;
        self.clear_form(window, cx);
        cx.notify();
    }

    fn delete_provider(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) {
        if index < self.providers.len() {
            self.providers.remove(index);

            // 更新展开状态
            self.expanded_providers.retain(|&i| i != index);
            self.expanded_providers = self
                .expanded_providers
                .iter()
                .map(|&i| if i > index { i - 1 } else { i })
                .collect();

            // 如果正在编辑被删除的提供商，清除编辑状态
            if self.editing_provider == Some(index) {
                self.editing_provider = None;
            } else if let Some(editing) = self.editing_provider {
                if editing > index {
                    self.editing_provider = Some(editing - 1);
                }
            }

            cx.notify();
        }
    }

    fn toggle_provider_enabled(
        &mut self,
        index: usize,
        enabled: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.providers.get_mut(index) {
            provider.enabled = enabled;
            cx.notify();
        }
    }

    fn edit_provider(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_provider = Some(index);

        if let Some(provider) = self.providers.get(index) {
            self.name_input.update(cx, |state, cx| {
                *state = InputState::new(window, cx).default_value(&provider.name);
            });

            self.api_url_input.update(cx, |state, cx| {
                *state = InputState::new(window, cx).default_value(&provider.api_url);
            });

            self.api_key_input.update(cx, |state, cx| {
                *state = InputState::new(window, cx)
                    .default_value(&provider.api_key)
                    .masked(true);
            });

            let type_index = match provider.api_type {
                ApiType::OpenAI => 0,
                ApiType::OpenAIResponse => 1,
                ApiType::Gemini => 2,
                ApiType::Anthropic => 3,
                ApiType::AzureOpenAI => 4,
            };

            self.api_type_dropdown.update(cx, |state, cx| {
                state.set_selected_index(Some(type_index), window, cx);
            });
        }

        cx.notify();
    }

    fn clear_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx).placeholder("服务提供商名称");
        });

        self.api_url_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx).placeholder("API 地址");
        });

        self.api_key_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx)
                .placeholder("API 密钥")
                .masked(true);
        });

        self.api_type_dropdown.update(cx, |state, cx| {
            state.set_selected_index(Some(0), window, cx);
        });
    }

    fn toggle_accordion(&mut self, open_ixs: &[usize], _: &mut Window, cx: &mut Context<Self>) {
        self.expanded_providers = open_ixs.to_vec();
        cx.notify();
    }

    fn on_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
                if self.editing_provider.is_some() {
                    self.save_provider(&SaveProvider, window, cx);
                }
            }
            _ => {}
        };
    }

    fn render_provider_form(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_4()
            .bg(gpui::rgb(0xF9FAFB))
            .rounded_lg()
            .border_1()
            .border_color(gpui::rgb(0xE5E7EB))
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .text_color(gpui::rgb(0x374151))
                    .child(if self.editing_provider.is_some() {
                        "编辑服务提供商"
                    } else {
                        "添加服务提供商"
                    }),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        v_flex()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child("名称 *"),
                            )
                            .child(TextInput::new(&self.name_input).cleanable()),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child("接口类型"),
                            )
                            .child(
                                Dropdown::new(&self.api_type_dropdown)
                                    .placeholder("选择接口类型")
                                    .small(),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child("API 地址 *"),
                    )
                    .child(TextInput::new(&self.api_url_input).cleanable()),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child("API 密钥 *"),
                    )
                    .child(
                        TextInput::new(&self.api_key_input)
                            .cleanable()
                            .mask_toggle(),
                    ),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-edit")
                            .label("取消")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editing_provider = None;
                                this.clear_form(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("save-provider")
                            .with_variant(ButtonVariant::Primary)
                            .label("保存")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_provider(&SaveProvider, window, cx);
                            })),
                    ),
            )
    }
}

impl FocusableCycle for LlmProvider {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            self.name_input.focus_handle(cx),
            self.api_url_input.focus_handle(cx),
            self.api_key_input.focus_handle(cx),
            self.api_type_dropdown.focus_handle(cx),
        ]
    }
}

impl Focusable for LlmProvider {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LlmProvider {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("llm-provider")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::add_provider))
            .on_action(cx.listener(Self::save_provider))
            .size_full()
            .gap_4()
            .child(
                // 添加按钮（移到左侧）
                h_flex()
                    .justify_start()
                    .child(
                        Button::new("add-provider")
                            .with_variant(ButtonVariant::Primary)
                            .label("添加提供商")
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_provider(&AddProvider, window, cx);
                            }))
                    )
            )
            .child(
                // 编辑表单（条件显示）
                div().when(self.editing_provider.is_some(), |this| {
                    this.child(self.render_provider_form(cx))
                })
            )
            .child(
                // 提供商列表 - 填满剩余空间
                div()
                   .w_full()
                    .child({
                        let mut accordion = Accordion::new("providers").multiple(true);
                        
                        for (index, provider) in self.providers.iter().enumerate() {
                            // 克隆需要在 UI 中使用的数据
                            let provider_name = provider.name.clone();
                            let provider_api_url = provider.api_url.clone();
                            let provider_api_key = provider.api_key.clone();
                            let provider_api_type = provider.api_type.as_str().to_string();
                            let provider_enabled = provider.enabled;
                            let provider_models = provider.models.clone();
                            
                            accordion = accordion.item(|item| {
                                item
                                    .open(self.expanded_providers.contains(&index))
                                    .icon(if provider_enabled { 
                                        IconName::CircleCheck 
                                    } else { 
                                        IconName::CircleX 
                                    })
                                    .title(
                                        h_flex() // 外层 h_flex，用于整个标题行
                                            .w_full() // 确保占满可用宽度
                                            .items_center() // 垂直居中对齐子元素
                                            // 使用 justify_between 或依赖 flex_1 将元素推向两端
                                            .justify_between() 
                                            .child(
                                                // 左侧：提供商名称
                                                div() 
                                                    .font_medium()
                                                    .flex_1() // 关键：让此 div 占据可用空间
                                                    .min_w_0() // 关键：允许此 div 在空间不足时收缩并配合 ellipsis
                                                    .overflow_hidden() // 配合 ellipsis
                                                    .text_ellipsis()   // 文本过长时显示省略号
                                                    .child(provider_name.clone())
                                            )
                                            .child(
                                                // 右侧：API类型标签、开关和操作按钮组
                                                h_flex() 
                                                    .items_center() 
                                                    .gap_2() 
                                                    .flex_shrink_0() // 关键：防止此组收缩
                                                    .child(
                                                        div() // API 类型标签
                                                            .px_2()
                                                            .py_1()
                                                            .bg(gpui::rgb(0xDDD6FE))
                                                            .text_color(gpui::rgb(0x7C3AED))
                                                            .rounded_md()
                                                            .text_xs()
                                                            .whitespace_nowrap() // 防止标签文字换行
                                                            .child(provider_api_type.clone())
                                                    )
                                                    .child(
                                                        Switch::new(("provider-enabled", index))
                                                            .checked(provider_enabled)
                                                            .on_click(cx.listener(move |this, checked, window, cx| {
                                                                this.toggle_provider_enabled(index, *checked, window, cx);
                                                            }))
                                                    )
                                                    .child(
                                                        Button::new(("edit-provider", index))
                                                            .icon(IconName::SquarePen)
                                                            .small()
                                                            .ghost()
                                                            .tooltip("编辑")
                                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                                this.edit_provider(index, window, cx);
                                                            }))
                                                    )
                                                    .child(
                                                        Button::new(("delete-provider", index))
                                                            .icon(IconName::Trash2)
                                                            .small()
                                                            .ghost()
                                                            .text_color(gpui::rgb(0xEF4444))
                                                            .tooltip("删除")
                                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                                this.delete_provider(index, window, cx);
                                                            }))
                                                    )
                                            )
                                    )
                                    .content(
                                        v_flex()
                                            .gap_4()
                                            .child(
                                                // 基本信息
                                                v_flex()
                                                    .gap_2()
                                                    .child(
                                                        h_flex()
                                                            .gap_4()
                                                            .child(
                                                                v_flex()
                                                                    .gap_1()
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .font_medium()
                                                                            .text_color(gpui::rgb(0x374151))
                                                                            .child("API 地址")
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .text_color(gpui::rgb(0x6B7280))
                                                                            .child(provider_api_url.clone())
                                                                    )
                                                            )
                                                            .child(
                                                                v_flex()
                                                                    .gap_1()
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .font_medium()
                                                                            .text_color(gpui::rgb(0x374151))
                                                                            .child("API 密钥")
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .text_color(gpui::rgb(0x6B7280))
                                                                            .child(if provider_api_key.is_empty() {
                                                                                "未配置"
                                                                            } else {
                                                                                "••••••••"
                                                                            })
                                                                    )
                                                            )
                                                    )
                                            )
                                            .child(
                                                // 模型列表
                                                v_flex()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_medium()
                                                            .text_color(gpui::rgb(0x374151))
                                                            .child("支持的模型")
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_2()
                                                            .children(provider_models.iter().enumerate().map(|(model_index, model)| {
                                                                let model_name = model.name.clone();
                                                                let model_enabled = model.enabled;
                                                                let model_capabilities = model.capabilities.clone();
                                                                let unique_model_id = index * 1000 + model_index;
                                                                
                                                                h_flex()
                                                                    .items_center()
                                                                    .justify_between()
                                                                    .p_3()
                                                                    .bg(gpui::rgb(0xF9FAFB))
                                                                    .rounded_md()
                                                                    .border_1()
                                                                    .border_color(gpui::rgb(0xE5E7EB))
                                                                    .child(
                                                                        h_flex()
                                                                            .items_center()
                                                                            .gap_3()
                                                                            .child(
                                                                                div()
                                                                                    .font_medium()
                                                                                    .text_color(gpui::rgb(0x111827))
                                                                                    .child(model_name.clone())
                                                                            )
                                                                            .child(
                                                                                h_flex()
                                                                                    .gap_1()
                                                                                    .items_center()
                                                                                    .children(model_capabilities.iter().enumerate().map(|(cap_index, cap)| {
                                                                                        // 创建一个唯一的数字ID，避免字符串生命周期问题
                                                                                        let capability_unique_id = index * 1000000 + model_index * 1000 + cap_index;
                                                                                        let cap_label = cap.label();
                                                                                        
                                                                                        div()
                                                                                            .id(("capability", capability_unique_id))  // 使用元组形式的ID
                                                                                            .p_1()
                                                                                            .rounded_md()
                                                                                            .bg(gpui::rgb(0xF3F4F6))
                                                                                            .child(Icon::new(cap.icon()).xsmall())
                                                                                            .tooltip(move |window, cx| {
                                                                                                Tooltip::new(cap_label).build(window, cx)
                                                                                            })
                                                                                    }))
                                                                            )
                                                                    )
                                                                    .child(
                                                                        Switch::new(("model-enabled", unique_model_id))
                                                                            .checked(model_enabled)
                                                                            .small()
                                                                    )
                                                            }))
                                                    )
                                            )
                                    )
                            });
                        }
                        
                        accordion.on_toggle_click(cx.listener(Self::toggle_accordion))
                    })
            )
    }
}

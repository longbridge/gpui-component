//! Provider/Model 选择器组件
//!
//! 提供可复用的 Provider 和 Model 选择功能。

use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, SharedString, Styled,
    Subscription, Window, px,
};
use gpui_component::{
    IndexPath, Sizable, Size, h_flex,
    select::{Select, SelectEvent, SelectItem, SelectState},
};
use rust_i18n::t;

use crate::llm::ProviderConfig;

// ============================================================================
// Provider 选择项
// ============================================================================

/// Provider 配置项（用于 UI 选择）
#[derive(Clone, Debug)]
pub struct ProviderItem {
    /// Provider ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 默认模型
    pub model: String,
    /// Provider 类型名称
    pub provider_type: String,
    /// 可用模型列表
    pub models: Vec<String>,
    /// 是否为默认 provider
    pub is_default: bool,
    /// 是否为内置 provider
    pub is_builtin: bool,
}

impl ProviderItem {
    /// 创建新的 Provider 选择项
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        model: impl Into<String>,
        provider_type: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: model.into(),
            provider_type: provider_type.into(),
            models: Vec::new(),
            is_default: false,
            is_builtin: false,
        }
    }

    /// 从 ProviderConfig 创建
    pub fn from_config(config: &ProviderConfig) -> Self {
        let models = if config.models.is_empty() {
            vec![config.model.clone()]
        } else {
            config.models.clone()
        };
        Self {
            id: config.id.to_string(),
            name: config.name.clone(),
            model: config.model.clone(),
            provider_type: config.provider_type.display_name().to_string(),
            models,
            is_default: config.is_default,
            is_builtin: config.is_builtin(),
        }
    }

    /// 设置可用模型列表
    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }

    /// 设置是否为默认
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// 获取显示名称
    pub fn display_name(&self) -> String {
        format!("{} - {} ({})", self.provider_type, self.model, self.name)
    }
}

impl SelectItem for ProviderItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.display_name().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

// ============================================================================
// Model 选择项
// ============================================================================

/// 模型选择项
#[derive(Clone, Debug)]
pub struct ModelItem {
    /// 模型 ID
    pub id: String,
}

impl ModelItem {
    /// 创建新的模型选择项
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

impl SelectItem for ModelItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.id.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

// ============================================================================
// 事件定义
// ============================================================================

/// Provider 选择器事件
#[derive(Clone, Debug)]
pub enum ProviderSelectEvent {
    /// Provider 已更改
    ProviderChanged {
        provider_id: String,
        models: Vec<String>,
        default_model: Option<String>,
    },
    /// Model 已更改
    ModelChanged { model: String },
}

// ============================================================================
// Provider 选择器状态
// ============================================================================

/// Provider 选择器状态
///
/// 管理 Provider 和 Model 选择器的状态。
pub struct ProviderSelectState {
    /// Provider 列表
    providers: Vec<ProviderItem>,
    /// Model 列表
    models: Vec<ModelItem>,
    /// 当前选中的 Provider ID
    selected_provider: Option<String>,
    /// 当前选中的 Model
    selected_model: Option<String>,
    /// Provider 选择器状态
    provider_select: Entity<SelectState<Vec<ProviderItem>>>,
    /// Model 选择器状态
    model_select: Entity<SelectState<Vec<ModelItem>>>,
    /// 订阅
    _subscriptions: Vec<Subscription>,
}

impl ProviderSelectState {
    /// 创建新的 Provider 选择器状态
    ///
    /// `on_event` 回调接收 `&mut T`（宿主实体的可变引用），
    /// 因为事件可能在实体更新过程中同步触发（如 `set_providers` 设置默认选中时），
    /// 直接传递引用避免了重复借用导致的 panic。
    pub fn new<T: 'static>(
        window: &mut Window,
        cx: &mut Context<T>,
        on_event: impl Fn(ProviderSelectEvent, &mut T, &mut Window, &mut Context<T>) + 'static,
    ) -> Self {
        // 创建 Provider 选择器
        let provider_select =
            cx.new(|cx| SelectState::new(Vec::<ProviderItem>::new(), None, window, cx));

        // 创建 Model 选择器
        let model_select = cx.new(|cx| SelectState::new(Vec::<ModelItem>::new(), None, window, cx));

        let mut subscriptions = Vec::new();

        // 订阅 Provider 选择事件
        let on_event_clone = std::rc::Rc::new(on_event);
        let on_event_provider = on_event_clone.clone();
        subscriptions.push(cx.subscribe_in(
            &provider_select,
            window,
            move |this, _select, event: &SelectEvent<Vec<ProviderItem>>, window, cx| {
                if let SelectEvent::Confirm(Some(provider_id)) = event {
                    on_event_provider(
                        ProviderSelectEvent::ProviderChanged {
                            provider_id: provider_id.clone(),
                            models: Vec::new(),
                            default_model: None,
                        },
                        this,
                        window,
                        cx,
                    );
                }
            },
        ));

        // 订阅 Model 选择事件
        let on_event_model = on_event_clone;
        subscriptions.push(cx.subscribe_in(
            &model_select,
            window,
            move |this, _select, event: &SelectEvent<Vec<ModelItem>>, window, cx| {
                if let SelectEvent::Confirm(Some(model_id)) = event {
                    on_event_model(
                        ProviderSelectEvent::ModelChanged {
                            model: model_id.clone(),
                        },
                        this,
                        window,
                        cx,
                    );
                }
            },
        ));

        Self {
            providers: Vec::new(),
            models: Vec::new(),
            selected_provider: None,
            selected_model: None,
            provider_select,
            model_select,
            _subscriptions: subscriptions,
        }
    }

    /// 获取当前选中的 Provider ID
    pub fn selected_provider(&self) -> Option<&String> {
        self.selected_provider.as_ref()
    }

    /// 获取当前选中的 Model
    pub fn selected_model(&self) -> Option<&String> {
        self.selected_model.as_ref()
    }

    /// 获取当前选中的 Provider Item
    pub fn selected_provider_item(&self) -> Option<&ProviderItem> {
        self.selected_provider
            .as_ref()
            .and_then(|id| self.providers.iter().find(|p| &p.id == id))
    }

    /// 获取 Provider ID（解析为 i64）
    pub fn selected_provider_id(&self) -> Option<i64> {
        self.selected_provider
            .as_ref()
            .and_then(|id| id.parse().ok())
    }

    /// 检查当前选中的是否为内置 provider
    pub fn is_selected_builtin(&self) -> bool {
        self.selected_provider_item()
            .map(|p| p.is_builtin)
            .unwrap_or(false)
    }

    /// 设置 Providers
    pub fn set_providers(
        &mut self,
        providers: Vec<ProviderItem>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if providers.is_empty() {
            self.providers.clear();
            self.models.clear();
            self.selected_provider = None;
            self.selected_model = None;
            self.provider_select.update(cx, |state, cx| {
                state.set_items(Vec::new(), window, cx);
                state.set_selected_index(None, window, cx);
            });
            self.model_select.update(cx, |state, cx| {
                state.set_items(Vec::new(), window, cx);
                state.set_selected_index(None, window, cx);
            });
            return;
        }

        // 找到默认 provider
        let default_idx = providers.iter().position(|p| p.is_default).unwrap_or(0);
        let default_provider = providers.get(default_idx).cloned();

        self.providers = providers.clone();
        self.provider_select.update(cx, |state, cx| {
            state.set_items(providers, window, cx);
            state.set_selected_index(Some(IndexPath::new(default_idx)), window, cx);
        });

        // 设置默认 provider 的模型
        if let Some(provider) = default_provider {
            self.selected_provider = Some(provider.id.clone());
            let models = Self::build_model_list(&provider);
            let default_model = Self::resolve_default_model(&provider, &models);
            self.set_models(models, default_model, window, cx);
        }
    }

    /// 从 ProviderConfig 列表设置 Providers
    pub fn set_provider_configs(
        &mut self,
        configs: &[ProviderConfig],
        window: &mut Window,
        cx: &mut App,
    ) {
        let items: Vec<ProviderItem> = configs
            .iter()
            .filter(|c| c.enabled)
            .map(ProviderItem::from_config)
            .collect();

        self.set_providers(items, window, cx);
    }

    /// 设置 Models
    pub fn set_models(
        &mut self,
        models: Vec<String>,
        default_model: Option<String>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let items: Vec<ModelItem> = models.iter().map(|m| ModelItem::new(m)).collect();
        let selected_idx = default_model
            .as_ref()
            .and_then(|dm| models.iter().position(|m| m == dm))
            .unwrap_or(0);

        self.models = items.clone();
        self.selected_model = default_model.clone();

        self.model_select.update(cx, |state, cx| {
            state.set_items(items, window, cx);
            state.set_selected_index(Some(IndexPath::new(selected_idx)), window, cx);
        });
    }

    /// 重建 Provider 选择器
    ///
    /// 智能处理 providers 列表：
    /// - 空列表时添加占位符
    /// - 保持当前选中的 provider（如果仍在列表中）
    /// - 否则选择默认标记的 provider 或第一个
    ///
    /// 返回选中的 provider ID
    pub fn rebuild(
        &mut self,
        providers: Vec<ProviderItem>,
        placeholder_name: impl Into<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let placeholder_name = placeholder_name.into();
        let mut providers = providers;

        // 空列表时添加占位符
        if providers.is_empty() {
            providers.push(ProviderItem::new("default", placeholder_name, "-", ""));
        }

        // 确定选中的 provider（按优先级）
        let selected = self
            .selected_provider
            .clone()
            .filter(|id| providers.iter().any(|p| p.id == *id))
            .or_else(|| {
                providers
                    .iter()
                    .find(|p| p.is_default)
                    .map(|p| p.id.clone())
            })
            .or_else(|| providers.first().map(|p| p.id.clone()));

        // 更新 is_default 标记
        if let Some(ref selected_id) = selected {
            for provider in &mut providers {
                provider.is_default = provider.id == *selected_id;
            }
        }

        // 设置 providers（会同时设置模型列表）
        self.set_providers(providers, window, cx);

        selected
    }

    /// 根据 provider_id 切换模型列表
    ///
    /// 查找对应的 provider 并更新模型选择器。
    /// 返回选中的模型名称。
    pub fn update_models_for_provider(
        &mut self,
        provider_id: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let provider = self.providers.iter().find(|p| p.id == provider_id)?.clone();
        let models = Self::build_model_list(&provider);
        let default_model = Self::resolve_default_model(&provider, &models);
        self.set_models(models, default_model.clone(), window, cx);
        default_model
    }

    /// 从 ProviderItem 构建清洗后的模型列表
    ///
    /// 处理逻辑：
    /// - 模型列表为空时使用默认模型
    /// - 过滤空白模型名
    /// - 确保至少有一个模型
    pub fn build_model_list(provider: &ProviderItem) -> Vec<String> {
        let mut models = if provider.models.is_empty() {
            vec![provider.model.clone()]
        } else {
            provider.models.clone()
        };
        models.retain(|m| !m.trim().is_empty());
        if models.is_empty() {
            models.push(provider.model.clone());
        }
        models
    }

    /// 从 ProviderConfig 构建清洗后的模型列表
    pub fn build_model_list_from_config(config: &ProviderConfig) -> Vec<String> {
        let item = ProviderItem::from_config(config);
        Self::build_model_list(&item)
    }

    /// 确定默认选中的模型
    ///
    /// 优先选择 provider 配置的默认模型，不在列表中则选第一个。
    pub fn resolve_default_model(provider: &ProviderItem, models: &[String]) -> Option<String> {
        if models.is_empty() {
            return None;
        }
        if models.iter().any(|m| *m == provider.model) {
            Some(provider.model.clone())
        } else {
            Some(models[0].clone())
        }
    }

    /// 从 ProviderConfig 确定默认选中的模型
    pub fn resolve_default_model_from_config(
        config: &ProviderConfig,
        models: &[String],
    ) -> Option<String> {
        let item = ProviderItem::from_config(config);
        Self::resolve_default_model(&item, models)
    }

    /// 渲染 Provider 选择器
    pub fn render_provider_select(&self) -> impl IntoElement {
        Select::new(&self.provider_select)
            .with_size(Size::Small)
            .min_w(px(140.0))
            .placeholder(t!("AiChat.select_provider_placeholder").to_string())
    }

    /// 渲染 Model 选择器
    pub fn render_model_select(&self) -> impl IntoElement {
        Select::new(&self.model_select)
            .with_size(Size::Small)
            .min_w(px(140.0))
            .placeholder(t!("AiChat.select_model_placeholder").to_string())
    }

    /// 渲染 Provider 和 Model 选择器组合
    pub fn render(&self) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(self.render_provider_select())
            .child(self.render_model_select())
    }
}

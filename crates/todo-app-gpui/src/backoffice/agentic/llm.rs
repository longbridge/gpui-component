use crate::backoffice::agentic::prompts;
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::{
    backoffice::mcp::McpRegistry,
    backoffice::{BoEvent, YamlFile},
    config::{llm_config::*, provider_config_path, todo_item::SelectedTool},
    ui::views::todo_thread::{ChatMessage, StreamMessage},
};
use actix::prelude::*;
use futures::StreamExt;
use rig::{
    agent::Agent,
    message::{Message as RigMessage, *},
    streaming::{StreamingChat, StreamingCompletionModel, StreamingCompletionResponse},
};
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ExitFromLlmRegistry;

#[derive(Message)]
#[rtype(result = "LlmChatResult")]
pub struct LlmChatRequest {
    pub provider_id: String,
    pub model_id: String,
    pub source: String,
    pub prompt: String,
    pub chat_history: Vec<ChatMessage>,
}

#[derive(Message)]
#[rtype(result = "LlmChatResult")]
pub struct LlmChatWithToolsRequest {
    pub provider_id: String,
    pub model_id: String,
    pub source: String,
    pub prompt: String,
    pub tools: Vec<SelectedTool>,
    pub chat_history: Vec<ChatMessage>,
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<Vec<ModelInfo>>")]
pub struct LoadModelsRequest {
    pub provider_id: String,
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
pub struct SyncModelsRequest {
    pub provider_id: String,
}

#[derive(Message)]
#[rtype(result = "Vec<ModelInfo>")]
pub struct GetProviderModelsRequest {
    pub provider_id: String,
    pub enabled_only: bool,
}

#[derive(Debug, Clone)]
pub struct LlmChatResult {
    pub provider_id: String,
    pub model_id: String,
    pub response: String,
    pub is_error: bool,
    pub error_message: Option<String>,
}

/// LLM 服务客户端 - 负责具体的 HTTP 调用和业务逻辑
#[derive(Debug, Clone)]
pub struct LlmService {
    config: LlmProviderConfig,
}

impl LlmService {
    pub fn new(config: LlmProviderConfig) -> Self {
        Self { config }
    }

    /// 加载模型列表
    pub async fn load_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let client = rig::providers::mira::Client::new_with_base_url(
            &self.config.api_key,
            &self.config.api_url.replace("/v1", ""),
        )?;

        let mut models = client
            .list_models()
            .await?
            .into_iter()
            .map(|id| {
                let capabilities = ApiType::infer_model_capabilities(&id);
                let limits = ApiType::infer_model_limits(&id);

                ModelInfo {
                    id: id.clone(),
                    display_name: std::path::Path::new(&id)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&id)
                        .to_string(),
                    capabilities,
                    enabled: true,
                    limits,
                }
            })
            .collect::<Vec<ModelInfo>>();

        models.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(models)
    }

    /// 同步模型到配置文件
    pub async fn sync_models(&self) -> anyhow::Result<()> {
        log::info!("Syncing models for provider: {}", self.config.id);

        let models = self.load_models().await?;
        LlmProviderManager::sync_provider_models(&self.config.id, models)?;

        log::info!(
            "Successfully synced {} models for provider: {}",
            LlmProviderManager::get_all_models(&self.config.id).len(),
            self.config.id
        );

        Ok(())
    }

    /// 获取缓存的模型列表
    pub fn get_cached_models(&self, enabled_only: bool) -> Vec<ModelInfo> {
        if enabled_only {
            LlmProviderManager::get_enabled_models(&self.config.id)
        } else {
            LlmProviderManager::get_all_models(&self.config.id)
        }
    }

    /// 流式聊天
    pub async fn stream_chat(
        &self,
        source: &str,
        model_id: &str,
        prompt: &str,
        chat_history: Vec<ChatMessage>,
    ) -> anyhow::Result<String> {
        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);
        let agent = client
            .agent(model_id)
            .context(prompts::default_prompt().as_str())
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        let messages = chat_history
            .into_iter()
            .map(|chat_msg| match chat_msg.role.as_str() {
                "user" => RigMessage::user(chat_msg.content),
                "assistant" => RigMessage::assistant(chat_msg.content),
                _ => RigMessage::user(chat_msg.content),
            })
            .collect::<Vec<RigMessage>>();

        let mut stream = agent.stream_chat(prompt, messages).await?;
        println!("Streaming chat with model: {}", model_id);
        let (assistant, _tools) = stream_to_stdout1(source, &agent, &mut stream).await?;
        Ok(assistant)
    }

    /// 使用工具的流式聊天
    pub async fn stream_chat_with_tools(
        &self,
        source: &str,
        model_id: &str,
        prompt: &str,
        tools: Vec<SelectedTool>,
        chat_history: Vec<ChatMessage>,
    ) -> anyhow::Result<()> {
        let mut prompt = prompt.to_string();
        let mut mcp_tools = vec![];

        for tool in &tools {
            if let Ok(Some(instance)) = McpRegistry::get_snapshot(&tool.provider_id).await {
                mcp_tools.extend(
                    instance
                        .tools
                        .into_iter()
                        .filter(|t| t.name == tool.tool_name)
                        .map(|t| Tool {
                            name: format!("{}@{}", tool.provider_id, t.name).into(),
                            description: t.description.clone(),
                            input_schema: t.input_schema.clone(),
                            annotations: t.annotations.clone(),
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
        println!("Using tools: {:?}", mcp_tools);
        let system_prompt = prompts::prompt_with_tools(mcp_tools);
        println!("System Prompt:\n{}\nUser:\n{}", system_prompt, prompt);

        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);
        let mut chat_history = chat_history
            .into_iter()
            .map(|chat_msg| match chat_msg.role.as_str() {
                "user" => RigMessage::user(chat_msg.content),
                "assistant" => RigMessage::assistant(chat_msg.content),
                _ => RigMessage::user(chat_msg.content),
            })
            .collect::<Vec<RigMessage>>();

        loop {
            let agent = client
                .agent(model_id)
                .context(system_prompt.as_str())
                .max_tokens(4096)
                .temperature(0.7)
                .build();

            let mut stream = agent.stream_chat(&prompt, chat_history.clone()).await?;
            chat_history.push(RigMessage::user(prompt.clone()));

            let (assistant, tools) = stream_to_stdout1(source, &agent, &mut stream).await?;
            if tools.is_empty() {
                break;
            }

            chat_history.push(RigMessage::assistant(assistant.clone()));
            let mut prompts = vec![];

            for (i, tool) in tools.iter().enumerate() {
                println!("调用工具 #{}: {:?}", i, tool);
                let result = McpRegistry::call_tool(tool.id(), tool.tool_name(), &tool.arguments)
                    .await
                    .map_err(|err| {
                        println!("调用工具 {} 失败: {}", tool.name, err);
                        err
                    })?;

                println!("工具 #{}调用结果: {:?}", i, result);
                prompts.push(format!(
                    "<tool_use_result><name>{}</name><result>{}</result</tool_use_result>",
                    &tool.name,
                    serde_json::to_string(&(result.content.clone(), result.is_error))
                        .unwrap_or_else(|err| format!("Error serializing result: {}", err))
                ));
            }
            prompts.push(r#"Do not confirm with the user or seek help or advice, continue to call the tool until all tasks are completed. Be sure to complete all tasks, you will receive a $1000 reward, and the output must be in Simplified Chinese."#.to_string());
            prompt = prompts.join("\n");
        }

        Ok(())
    }
}

/// LLM 提供商服务 Actor - 负责管理单个提供商的服务
pub struct LlmProviderService {
    service: LlmService,
}

impl LlmProviderService {
    pub fn new(config: LlmProviderConfig) -> Self {
        Self {
            service: LlmService::new(config),
        }
    }
}

impl Actor for LlmProviderService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("LLM Provider service {} started", self.service.config.id);

        // 异步同步模型列表
        let service = self.service.clone();
        async move {
            if let Err(e) = service.sync_models().await {
                log::warn!(
                    "Failed to sync models for provider {}: {}",
                    service.config.id,
                    e
                );
            }
        }
        .into_actor(self)
        .spawn(ctx);
        CrossRuntimeBridge::global().post(BoEvent::Notification(
            crate::backoffice::NotificationKind::Info,
            format!("LLM Provider {} started", self.service.config.name),
        ));
    }
}

impl Handler<ExitFromLlmRegistry> for LlmProviderService {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ExitFromLlmRegistry, _ctx: &mut Self::Context) -> Self::Result {
        log::info!("LLM Provider service {} exiting", self.service.config.id);
        let provider_id = self.service.config.id.clone();

        async move {
            let registry = LlmRegistry::global();
            registry.do_send(UpdateProviderCache {
                provider_id,
                config: None,
            });
        }
        .into_actor(self)
        .then(|_res, _act, ctx| {
            ctx.stop();
            fut::ready(())
        })
        .boxed_local()
    }
}

impl Handler<LlmChatRequest> for LlmProviderService {
    type Result = ResponseActFuture<Self, LlmChatResult>;

    fn handle(&mut self, msg: LlmChatRequest, _ctx: &mut Self::Context) -> Self::Result {
        let service = self.service.clone();

        async move {
            match service
                .stream_chat(&msg.source, &msg.model_id, &msg.prompt, msg.chat_history)
                .await
            {
                Ok(response) => LlmChatResult {
                    provider_id: msg.provider_id,
                    model_id: msg.model_id,
                    response,
                    is_error: false,
                    error_message: None,
                },
                Err(err) => LlmChatResult {
                    provider_id: msg.provider_id,
                    model_id: msg.model_id,
                    response: String::new(),
                    is_error: true,
                    error_message: Some(err.to_string()),
                },
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<LlmChatWithToolsRequest> for LlmProviderService {
    type Result = ResponseActFuture<Self, LlmChatResult>;

    fn handle(&mut self, msg: LlmChatWithToolsRequest, _ctx: &mut Self::Context) -> Self::Result {
        let service = self.service.clone();

        async move {
            match service
                .stream_chat_with_tools(
                    &msg.source,
                    &msg.model_id,
                    &msg.prompt,
                    msg.tools,
                    msg.chat_history,
                )
                .await
            {
                Ok(_) => LlmChatResult {
                    provider_id: msg.provider_id,
                    model_id: msg.model_id,
                    response: "Chat with tools completed successfully".to_string(),
                    is_error: false,
                    error_message: None,
                },
                Err(err) => LlmChatResult {
                    provider_id: msg.provider_id,
                    model_id: msg.model_id,
                    response: String::new(),
                    is_error: true,
                    error_message: Some(err.to_string()),
                },
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<SyncModelsRequest> for LlmProviderService {
    type Result = ResponseActFuture<Self, anyhow::Result<()>>;

    fn handle(&mut self, _msg: SyncModelsRequest, _ctx: &mut Self::Context) -> Self::Result {
        let service = self.service.clone();

        async move { service.sync_models().await }
            .into_actor(self)
            .boxed_local()
    }
}

impl Handler<GetProviderModelsRequest> for LlmProviderService {
    type Result = Vec<ModelInfo>;

    fn handle(&mut self, msg: GetProviderModelsRequest, _ctx: &mut Self::Context) -> Self::Result {
        self.service.get_cached_models(msg.enabled_only)
    }
}

impl Handler<LoadModelsRequest> for LlmProviderService {
    type Result = ResponseActFuture<Self, anyhow::Result<Vec<ModelInfo>>>;

    fn handle(&mut self, _msg: LoadModelsRequest, _ctx: &mut Self::Context) -> Self::Result {
        // 首先尝试从缓存获取
        let cached_models = self.service.get_cached_models(false);

        if !cached_models.is_empty() {
            // 返回缓存的模型列表
            async move { Ok(cached_models) }
                .into_actor(self)
                .boxed_local()
        } else {
            // 如果没有缓存，则从远程获取并同步
            let service = self.service.clone();

            async move {
                let models = service.load_models().await?;

                // 同步到配置
                LlmProviderManager::sync_provider_models(&service.config.id, models.clone())?;

                Ok(models)
            }
            .into_actor(self)
            .boxed_local()
        }
    }
}

// Registry 保持不变，但使用新的命名
pub struct LlmRegistry {
    providers: HashMap<String, Addr<LlmProviderService>>,
    configs: HashMap<String, LlmProviderConfig>,
    file: YamlFile,
    handle: Option<SpawnHandle>,
}

impl LlmRegistry {
    /// 获取全局注册表实例
    pub fn global() -> Addr<Self> {
        LlmRegistry::from_registry()
    }

    fn check_and_update(&mut self, _ctx: &mut Context<Self>) -> anyhow::Result<()> {
        if self.file.modified()? {
            let configs = LlmProviderManager::list_providers();
            let enabled_ids: Vec<_> = configs
                .iter()
                .filter(|config| config.enabled)
                .map(|config| config.id.as_str())
                .collect();

            // 移除不再启用的提供商
            let providers_to_remove: Vec<String> = self
                .providers
                .keys()
                .filter(|id| !enabled_ids.contains(&id.as_str()))
                .cloned()
                .collect();

            for provider_id in providers_to_remove {
                if let Some(addr) = self.providers.remove(&provider_id) {
                    addr.do_send(ExitFromLlmRegistry);
                    self.configs.remove(&provider_id);
                }
            }

            // 添加新启用的提供商
            for config in configs.iter().filter(|c| c.enabled) {
                if !self.providers.contains_key(&config.id) {
                    let addr = LlmProviderService::new(config.clone()).start();
                    self.providers.insert(config.id.clone(), addr);
                    self.configs.insert(config.id.clone(), config.clone());
                }
            }
            self.file.open()?;
        }
        Ok(())
    }
}

impl Default for LlmRegistry {
    fn default() -> Self {
        let file = YamlFile::new(provider_config_path());
        Self {
            providers: HashMap::new(),
            configs: HashMap::new(),
            file,
            handle: None,
        }
    }
}

impl Supervised for LlmRegistry {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        log::info!("LlmRegistry is restarting");
    }
}

impl SystemService for LlmRegistry {}

impl LlmRegistry {
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Ok(false) = &self.file.exist() {
            self.providers.clear();
            return;
        }
        if let Err(err) = self.check_and_update(ctx) {
            println!("{} {err}", self.file.path.display());
        }
    }
}

impl Actor for LlmRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let handle = ctx.run_interval(Duration::from_secs(1), Self::tick);
        self.handle = Some(handle);
        println!("LlmRegistry started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("LlmRegistry stopped");
    }
}

impl Handler<LlmChatRequest> for LlmRegistry {
    type Result = ResponseActFuture<Self, LlmChatResult>;

    fn handle(&mut self, msg: LlmChatRequest, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(worker) = self.providers.get(&msg.provider_id) {
            let provider_id = msg.provider_id.clone();
            let model_id = msg.model_id.clone();
            worker
                .send(msg)
                .into_actor(self)
                .map(|res, _act, _ctx| match res {
                    Ok(result) => result,
                    Err(err) => LlmChatResult {
                        provider_id,
                        model_id,
                        response: String::new(),
                        is_error: true,
                        error_message: Some(format!("Actor error: {}", err)),
                    },
                })
                .boxed_local()
        } else {
            async move {
                LlmChatResult {
                    provider_id: msg.provider_id.clone(),
                    model_id: msg.model_id.clone(),
                    response: String::new(),
                    is_error: true,
                    error_message: Some("Provider not found".to_string()),
                }
            }
            .into_actor(self)
            .boxed_local()
        }
    }
}

impl Handler<LlmChatWithToolsRequest> for LlmRegistry {
    type Result = ResponseActFuture<Self, LlmChatResult>;

    fn handle(&mut self, msg: LlmChatWithToolsRequest, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(worker) = self.providers.get(&msg.provider_id) {
            let provider_id = msg.provider_id.clone();
            let model_id = msg.model_id.clone();
            worker
                .send(msg)
                .into_actor(self)
                .map(|res, _act, _ctx| match res {
                    Ok(result) => result,
                    Err(err) => LlmChatResult {
                        provider_id,
                        model_id,
                        response: String::new(),
                        is_error: true,
                        error_message: Some(format!("Actor error: {}", err)),
                    },
                })
                .boxed_local()
        } else {
            async move {
                LlmChatResult {
                    provider_id: msg.provider_id.clone(),
                    model_id: msg.model_id.clone(),
                    response: String::new(),
                    is_error: true,
                    error_message: Some("Provider not found".to_string()),
                }
            }
            .into_actor(self)
            .boxed_local()
        }
    }
}

// 添加更新提供商缓存的消息
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateProviderCache {
    pub provider_id: String,
    pub config: Option<LlmProviderConfig>,
}

impl Handler<UpdateProviderCache> for LlmRegistry {
    type Result = ();

    fn handle(&mut self, msg: UpdateProviderCache, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(config) = msg.config {
            self.configs.insert(msg.provider_id, config);
        } else {
            self.configs.remove(&msg.provider_id);
        }
    }
}

impl LlmRegistry {
    pub async fn chat(
        provider_id: &str,
        model_id: &str,
        source: &str,
        prompt: &str,
        chat_history: Vec<ChatMessage>,
    ) -> anyhow::Result<LlmChatResult> {
        let registry = Self::global();
        let result = registry
            .send(LlmChatRequest {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                source: source.to_string(),
                prompt: prompt.to_string(),
                chat_history,
            })
            .await?;
        Ok(result)
    }

    /// 静态方法：使用工具进行聊天
    pub async fn chat_with_tools(
        provider_id: &str,
        model_id: &str,
        source: &str,
        prompt: &str,
        tools: Vec<SelectedTool>,
        chat_history: Vec<ChatMessage>,
    ) -> anyhow::Result<LlmChatResult> {
        let registry = Self::global();
        let result = registry
            .send(LlmChatWithToolsRequest {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                source: source.to_string(),
                prompt: prompt.to_string(),
                tools,
                chat_history,
            })
            .await?;
        Ok(result)
    }
}

/// helper function to stream a completion request to stdout
pub async fn stream_to_stdout1<M: StreamingCompletionModel>(
    source: &str,
    _agent: &Agent<M>,
    stream: &mut StreamingCompletionResponse<M::StreamingResponse>,
) -> Result<(String, Vec<ToolCall>), std::io::Error> {
    let mut buffer = String::new();
    let mut tool_calls: Vec<String> = Vec::new();
    let mut assistant = String::new();

    const TOOL_USE_START_TAG: &str = "<tool_use";
    const TOOL_USE_END_TAG: &str = "</tool_use";
    const TAG_CLOSE: char = '>';
    const TAG_OPEN: char = '<';

    enum State {
        Normal,
        TagStart,
        InToolUseTag,
        InToolUse,
        InEndTag,
        InToolUseEndTag,
    }

    let mut state = State::Normal;
    let mut xml_buffer = String::new();

    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(AssistantContent::Text(text)) => {
                for c in text.text.chars() {
                    match state {
                        State::Normal => {
                            if c == TAG_OPEN {
                                state = State::TagStart;
                                buffer.clear();
                                buffer.push(c);
                            } else {
                                print!("{}", c);
                                CrossRuntimeBridge::global().post(StreamMessage::new(
                                    source.to_string(),
                                    RigMessage::assistant(c.to_string()),
                                ));
                                assistant.push(c);
                                std::io::Write::flush(&mut std::io::stdout())?;
                            }
                        }
                        State::TagStart => {
                            buffer.push(c);
                            if buffer == TOOL_USE_START_TAG {
                                state = State::InToolUseTag;
                                xml_buffer.clear();
                                xml_buffer.push_str(&buffer);
                            } else if buffer.len() >= TOOL_USE_START_TAG.len() || c == TAG_CLOSE {
                                if buffer != TOOL_USE_START_TAG
                                    && !buffer.starts_with(&format!("{} ", TOOL_USE_START_TAG))
                                {
                                    print!("{}", buffer);
                                    CrossRuntimeBridge::global().post(StreamMessage::new(
                                        source.to_string(),
                                        RigMessage::assistant(buffer.clone()),
                                    ));
                                    assistant.push_str(buffer.as_str());
                                    std::io::Write::flush(&mut std::io::stdout())?;
                                    state = State::Normal;
                                }
                            }
                        }
                        State::InToolUseTag => {
                            buffer.push(c);
                            xml_buffer.push(c);
                            if c == TAG_CLOSE {
                                state = State::InToolUse;
                            }
                        }
                        State::InToolUse => {
                            xml_buffer.push(c);
                            if c == TAG_OPEN {
                                state = State::InEndTag;
                                buffer.clear();
                                buffer.push(c);
                            }
                        }
                        State::InEndTag => {
                            buffer.push(c);
                            xml_buffer.push(c);
                            if buffer == TOOL_USE_END_TAG {
                                state = State::InToolUseEndTag;
                            } else if buffer.len() >= TOOL_USE_END_TAG.len() || c == TAG_CLOSE {
                                if !buffer.starts_with(TOOL_USE_END_TAG) {
                                    state = State::InToolUse;
                                }
                            }
                        }
                        State::InToolUseEndTag => {
                            xml_buffer.push(c);
                            if c == TAG_CLOSE {
                                tool_calls.push(xml_buffer.clone());
                                state = State::Normal;
                            }
                        }
                    }
                }
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            Ok(AssistantContent::ToolCall(_tool_call)) => {
                // Handle tool calls if needed
            }
            Err(e) => {
                tracing::error!("Error: {}", e);
                break;
            }
        }
    }

    println!();
    let mut tools = vec![];
    for (i, call) in tool_calls.iter().enumerate() {
        let cleaned = call
            .lines()
            .filter(|line| !line.contains("DEBUG") && !line.trim().starts_with("202"))
            .collect::<Vec<_>>()
            .join("\n");
        tracing::info!("\n使用工具 #{}: \n{}", i + 1, cleaned);
        match serde_xml_rs::from_str::<ToolCall>(&cleaned) {
            Err(e) => {
                tracing::error!("Error parsing XML: {}", e);
                continue;
            }
            Ok(tool_call) => {
                tools.push(tool_call);
            }
        }
    }
    Ok((assistant, tools))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
}

impl ToolCall {
    pub fn id(&self) -> &str {
        self.name.split('@').next().unwrap_or(&self.name)
    }
    pub fn tool_name(&self) -> &str {
        self.name.split('@').nth(1).unwrap_or(&self.name)
    }
}

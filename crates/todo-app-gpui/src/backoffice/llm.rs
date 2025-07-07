mod service;
use crate::backoffice::agentic::{prompts, ToolInfo};
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::{
    backoffice::mcp::McpRegistry,
    backoffice::{BoEvent, YamlFile},
    config::{llm_config::*, provider_config_path, todo_item::SelectedTool},
    ui::views::todo_thread::{ChatMessage, StreamMessage},
};
use actix::prelude::*;
use futures::StreamExt;
use gpui_component::fuchsia;
use rig::completion::{Completion, Prompt};
use rig::streaming::StreamingCompletion;
use rig::{
    agent::Agent,
    message::{Message as RigMessage, *},
    streaming::{StreamingChat, StreamingCompletionModel, StreamingCompletionResponse},
};
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};


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

// Registry 保持不变，但使用新的命名
pub struct LlmRegistry {
    providers: HashMap<String, RigLlmService>,
    file: YamlFile,
    handle: Option<SpawnHandle>,
}

impl LlmRegistry {
    /// 获取全局注册表实例
    pub fn global() -> Addr<Self> {
        LlmRegistry::from_registry()
    }

    fn check_and_update(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
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
                    self.providers.remove(&provider_id);
                }
            }

            // 添加新启用的提供商
            for config in configs.iter().filter(|c| c.enabled) {
                if !self.providers.contains_key(&config.id) {
                   let mut llm =  RigLlmService::new(config.clone())?;
                   async move {
                     llm.load_models().await;
                     llm
                   }.into_actor(self).then(|llm,act,ctx|{
                    act.providers.insert(llm.config.id.clone(), llm);
                    fut::ready(())
                   }).spawn(ctx);
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

// impl Handler<UpdateProviderCache> for LlmRegistry {
//     type Result = ();

//     fn handle(&mut self, msg: UpdateProviderCache, _ctx: &mut Self::Context) -> Self::Result {
//         if let Some(config) = msg.config {
//             self.configs.insert(msg.provider_id, config);
//         } else {
//             self.configs.remove(&msg.provider_id);
//         }
//     }
// }

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
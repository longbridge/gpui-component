mod parser;
mod provider;
mod stream_tools;
pub mod types;

use crate::backoffice::llm::provider::LlmChoice;
use crate::backoffice::llm::types::{ChatMessage, ChatStream};
use crate::{
    backoffice::YamlFile,
    config::{llm_config::*, provider_config_path},
};
use actix::prelude::*;
use std::{collections::HashMap, time::Duration};

#[derive(Message, Debug, Clone)]
#[rtype(result = "anyhow::Result<ChatStream>")]
pub struct LlmChatRequest {
    pub provider_id: String,
    pub model_id: String,
    pub source: String,
    pub prompt: String,
    pub history: Vec<ChatMessage>,
}

impl LlmChatRequest {
    pub fn new(provider_id: String, model_id: String) -> Self {
        Self {
            provider_id,
            model_id,
            source: String::new(),
            prompt: String::new(),
            history: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.prompt = prompt;
        self
    }

    pub fn with_history(mut self, history: Vec<ChatMessage>) -> Self {
        self.history = history;
        self
    }

    pub fn with_message(mut self, message: ChatMessage) -> Self {
        self.history.push(message);
        self
    }
}

pub struct LlmRegistry {
    providers: HashMap<String, LlmProviderConfig>,
    file: YamlFile,
    handle: Option<SpawnHandle>,
}

impl LlmRegistry {
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
            self.providers
                .retain(|id, _| enabled_ids.contains(&id.as_str()));

            // 添加新启用的提供商
            for config in configs.iter().filter(|c| c.enabled) {
                if !self.providers.contains_key(&config.id) {
                    let config_clone = config.clone();
                    let mut config = config_clone.clone();
                    async move {
                        let llm = LlmChoice::new(&config_clone)?;
                        llm.load_models().await
                    }
                    .into_actor(self)
                    .then(move |models, act, _ctx| match models {
                        Ok(models) => {
                            config.models = models;
                            act.providers.insert(config.id.clone(), config.clone());
                            LlmProviderManager::update_provider(&config.id.clone(), config).ok();
                            fut::ready(())
                        }
                        Err(err) => {
                            tracing::error!("Failed to load models for {}: {}", config.id, err);
                            fut::ready(())
                        }
                    })
                    .spawn(ctx);
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
        tracing::info!("LlmRegistry is restarting");
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
            tracing::error!("{} {err}", self.file.path.display());
        }
    }
}

impl Actor for LlmRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let handle = ctx.run_interval(Duration::from_secs(1), Self::tick);
        self.handle = Some(handle);
        tracing::trace!("LlmRegistry started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::trace!("LlmRegistry stopped");
    }
}

impl Handler<LlmChatRequest> for LlmRegistry {
    type Result = ResponseActFuture<Self, anyhow::Result<ChatStream>>;

    fn handle(&mut self, msg: LlmChatRequest, _ctx: &mut Self::Context) -> Self::Result {
        tracing::trace!(
            "Received LLM chat request: provider_id={}, model_id={}, source={}, messages={}",
            msg.provider_id,
            msg.model_id,
            msg.source,
            msg.history.len(),
        );

        if let Some(config) = self.providers.get(&msg.provider_id).cloned() {
            let model_id = msg.model_id.clone();
            let messages = msg.history;
            let source = msg.source;
            let prompt = msg.prompt;
            async move {
                tracing::trace!(
                    "Starting LLM chat with provider: {}, model: {}, source: {}",
                    msg.provider_id,
                    model_id,
                    source
                );
                let llm = LlmChoice::new(&config)?;
                stream_tools::chat_stream_with_tools_simple(llm, &model_id, &prompt, messages, 128)
                    .await
            }
            .into_actor(self)
            .map(|res, _act, _ctx| res)
            .boxed_local()
        } else {
            let provider_id = msg.provider_id.clone();
            async move {
                Err(anyhow::anyhow!(
                    "Provider '{}' not found or not enabled",
                    provider_id
                ))
            }
            .into_actor(self)
            .boxed_local()
        }
    }
}

impl LlmRegistry {
    pub async fn chat_stream(request: LlmChatRequest) -> anyhow::Result<ChatStream> {
        let registry = Self::global();
        let result = registry.send(request).await??;
        Ok(result)
    }
}

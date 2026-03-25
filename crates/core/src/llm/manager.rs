use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use gpui::Global;
use parking_lot::RwLock;

use super::connector::{LlmConnector, LlmProvider};
use super::onet_cli_provider::OnetCliLLMProvider;
use super::types::{ProviderConfig, ProviderType};
use crate::cloud_sync::client::CloudApiClient;

struct ProviderCacheEntry {
    signature: String,
    provider: Arc<dyn LlmProvider>,
}

pub struct ProviderManager {
    providers: Arc<DashMap<i64, ProviderCacheEntry>>,
    cloud_client: RwLock<Option<Arc<dyn CloudApiClient>>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(DashMap::new()),
            cloud_client: RwLock::new(None),
        }
    }

    /// 设置云端 API 客户端（用于 OnetCli Provider）
    pub fn set_cloud_client(&self, client: Arc<dyn CloudApiClient>) {
        *self.cloud_client.write() = Some(client);
    }

    pub async fn get_provider(&self, config: &ProviderConfig) -> Result<Arc<dyn LlmProvider>> {
        let id = config.id;
        let signature = provider_cache_signature(config);

        if let Some(entry) = self.providers.get(&id)
            && entry.signature == signature
        {
            return Ok(Arc::clone(&entry.provider));
        }

        if !config.enabled {
            anyhow::bail!("Provider is disabled: {}", id);
        }

        let provider: Arc<dyn LlmProvider> = match config.provider_type {
            ProviderType::OnetCli => {
                let cloud_client = self.cloud_client.read().clone().ok_or_else(|| {
                    anyhow::anyhow!("CloudApiClient not set for OnetCli provider")
                })?;

                let onet_provider = OnetCliLLMProvider::new(cloud_client);

                Arc::new(onet_provider)
            }
            _ => {
                let connector = LlmConnector::from_config(config)?;
                Arc::new(connector)
            }
        };

        self.providers.insert(
            id,
            ProviderCacheEntry {
                signature,
                provider: Arc::clone(&provider),
            },
        );

        Ok(provider)
    }

    pub fn remove_provider(&self, id: i64) {
        self.providers.remove(&id);
    }

    pub fn clear_cache(&self) {
        self.providers.clear();
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

fn provider_cache_signature(config: &ProviderConfig) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        config.provider_type.as_str(),
        config.model,
        config.api_base.as_deref().unwrap_or_default(),
        config.api_version.as_deref().unwrap_or_default(),
        config.api_key.as_deref().unwrap_or_default(),
        config.enabled,
        config.name
    )
}

pub struct GlobalProviderState {
    manager: Arc<ProviderManager>,
}

impl Clone for GlobalProviderState {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
        }
    }
}

impl GlobalProviderState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(ProviderManager::new()),
        }
    }

    pub fn manager(&self) -> Arc<ProviderManager> {
        Arc::clone(&self.manager)
    }

    /// 设置云端 API 客户端
    pub fn set_cloud_client(&self, client: Arc<dyn CloudApiClient>) {
        self.manager.set_cloud_client(client);
    }
}

impl Default for GlobalProviderState {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for GlobalProviderState {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_cache_signature_changes_with_model() {
        let base = ProviderConfig {
            id: 1,
            provider_type: ProviderType::Aliyun,
            name: "aliyun".to_string(),
            api_key: Some("sk-test".to_string()),
            model: "qwen-plus".to_string(),
            ..Default::default()
        };
        let mut changed = base.clone();
        changed.model = "qwen3.5-plus".to_string();

        assert_ne!(
            provider_cache_signature(&base),
            provider_cache_signature(&changed)
        );
    }
}

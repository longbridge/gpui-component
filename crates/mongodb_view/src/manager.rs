//! MongoDB 全局状态管理

use crate::connection::{MongoConnection, MongoConnectionImpl};
use crate::types::{MongoConnectionConfig, MongoError};
use dashmap::DashMap;
use gpui::Global;
use one_core::storage::MongoDBParams;
use rust_i18n::t;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MongoDB 连接存储
type ConnectionMap = DashMap<String, Arc<RwLock<Box<dyn MongoConnection>>>>;

/// MongoDB 全局状态
#[derive(Clone, Default)]
pub struct GlobalMongoState {
    connections: Arc<ConnectionMap>,
}

impl Global for GlobalMongoState {}

impl GlobalMongoState {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_connection(
        &self,
        config: MongoConnectionConfig,
    ) -> Result<String, MongoError> {
        let connection_id = config.id.clone();
        if connection_id.is_empty() {
            return Err(MongoError::Internal(
                t!("MongoManager.connection_id_required").to_string(),
            ));
        }

        let mut connection = MongoConnectionImpl::new(config);
        connection.connect().await?;

        let connection_arc: Arc<RwLock<Box<dyn MongoConnection>>> =
            Arc::new(RwLock::new(Box::new(connection)));
        self.connections
            .insert(connection_id.clone(), connection_arc);

        Ok(connection_id)
    }

    pub fn get_connection(
        &self,
        connection_id: &str,
    ) -> Option<Arc<RwLock<Box<dyn MongoConnection>>>> {
        self.connections
            .get(connection_id)
            .map(|entry| entry.clone())
    }

    pub async fn remove_connection(&self, connection_id: &str) -> Result<(), MongoError> {
        if let Some((_, connection)) = self.connections.remove(connection_id) {
            let mut guard = connection.write().await;
            guard.disconnect().await?;
        }
        Ok(())
    }

    pub fn has_connection(&self, connection_id: &str) -> bool {
        self.connections.contains_key(connection_id)
    }

    pub fn connection_ids(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub async fn close_all(&self) {
        let ids: Vec<String> = self.connection_ids();
        for id in ids {
            let _ = self.remove_connection(&id).await;
        }
    }
}

/// MongoDB 连接管理器辅助函数
pub struct MongoManager;

impl MongoManager {
    pub async fn test_connection(config: &MongoConnectionConfig) -> Result<(), MongoError> {
        let mut connection = MongoConnectionImpl::new(config.clone());
        connection.connect().await?;
        connection.ping().await?;
        connection.disconnect().await?;
        Ok(())
    }

    pub fn build_connection_string(params: &MongoDBParams) -> Result<String, MongoError> {
        let raw_connection_string = params.connection_string.trim().to_string();
        if !raw_connection_string.is_empty() {
            return Ok(raw_connection_string);
        }

        let host_value = params.host.trim().to_string();
        if host_value.is_empty() {
            return Err(MongoError::Internal(
                t!("MongoManager.host_required").to_string(),
            ));
        }

        let scheme = if params.use_srv_record {
            "mongodb+srv"
        } else {
            "mongodb"
        };

        let mut connection_string = String::new();
        connection_string.push_str(scheme);
        connection_string.push_str("://");

        let username_value = params
            .username
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");
        let password_value = params
            .password
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");

        if !username_value.is_empty() {
            connection_string.push_str(username_value);
            if !password_value.is_empty() {
                connection_string.push(':');
                connection_string.push_str(password_value);
            }
            connection_string.push('@');
        }

        if params.use_srv_record || host_value.contains(':') || host_value.contains(',') {
            connection_string.push_str(&host_value);
        } else {
            let port_value = params.port.unwrap_or(27017);
            connection_string.push_str(&format!("{}:{}", host_value, port_value));
        }

        let database_value = params
            .database
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");
        if !database_value.is_empty() {
            connection_string.push('/');
            connection_string.push_str(database_value);
        }

        let mut query_pairs: Vec<(String, String)> = Vec::new();

        let auth_source_value = params
            .auth_source
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");
        if !auth_source_value.is_empty() {
            query_pairs.push(("authSource".to_string(), auth_source_value.to_string()));
        }

        let replica_set_value = params
            .replica_set
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");
        if !replica_set_value.is_empty() {
            query_pairs.push(("replicaSet".to_string(), replica_set_value.to_string()));
        }

        let read_preference_value = params
            .read_preference
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");
        if !read_preference_value.is_empty() {
            query_pairs.push((
                "readPreference".to_string(),
                read_preference_value.to_string(),
            ));
        }

        if params.direct_connection {
            query_pairs.push(("directConnection".to_string(), "true".to_string()));
        }

        if params.use_tls {
            query_pairs.push(("tls".to_string(), "true".to_string()));
        }

        if let Some(connect_timeout_seconds) = params.connect_timeout_seconds {
            let timeout_millis = connect_timeout_seconds.saturating_mul(1000);
            query_pairs.push(("connectTimeoutMS".to_string(), timeout_millis.to_string()));
        }

        let application_name_value = params
            .application_name
            .as_ref()
            .map(|value| value.trim())
            .unwrap_or("");
        if !application_name_value.is_empty() {
            query_pairs.push(("appName".to_string(), application_name_value.to_string()));
        }

        if !query_pairs.is_empty() {
            if database_value.is_empty() {
                connection_string.push('/');
            }
            connection_string.push('?');
            let query_string = query_pairs
                .into_iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<String>>()
                .join("&");
            connection_string.push_str(&query_string);
        }

        Ok(connection_string)
    }

    pub async fn test_parameters(name: String, params: &MongoDBParams) -> Result<(), MongoError> {
        let connection_string = Self::build_connection_string(params)?;
        let config = MongoConnectionConfig {
            id: "test".to_string(),
            name,
            connection_string,
        };
        Self::test_connection(&config).await
    }

    pub fn config_from_stored(
        stored: &one_core::storage::StoredConnection,
    ) -> Result<MongoConnectionConfig, MongoError> {
        let parameters = stored
            .to_mongodb_params()
            .map_err(|e| MongoError::Serialization(e.to_string()))?;
        let connection_string = Self::build_connection_string(&parameters)?;

        Ok(MongoConnectionConfig {
            id: stored.id.map(|id| id.to_string()).unwrap_or_default(),
            name: stored.name.clone(),
            connection_string,
        })
    }
}

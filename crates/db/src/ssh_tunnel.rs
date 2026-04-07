use std::time::Duration;

use one_core::storage::DbConnectionConfig;
use ssh::{start_local_port_forward, LocalPortForwardTunnel, SshAuth, SshConnectConfig};
use tokio::time::timeout;

use crate::connection::DbError;

const SSH_TUNNEL_ENABLED: &str = "ssh_tunnel_enabled";
const SSH_HOST: &str = "ssh_host";
const SSH_PORT: &str = "ssh_port";
const SSH_USERNAME: &str = "ssh_username";
const SSH_AUTH_TYPE: &str = "ssh_auth_type";
const SSH_PASSWORD: &str = "ssh_password";
const SSH_PRIVATE_KEY_PATH: &str = "ssh_private_key_path";
const SSH_PRIVATE_KEY_PASSPHRASE: &str = "ssh_private_key_passphrase";
const SSH_TARGET_HOST: &str = "ssh_target_host";
const SSH_TARGET_PORT: &str = "ssh_target_port";
const SSH_TIMEOUT: &str = "ssh_timeout";

/// Default SSH connection timeout in seconds.
const DEFAULT_SSH_TIMEOUT_SECS: u64 = 30;

pub struct ResolvedConnectionTarget {
    pub host: String,
    pub port: u16,
    pub tunnel: Option<LocalPortForwardTunnel>,
}

fn normalize_direct_host(host: &str) -> String {
    if host.eq_ignore_ascii_case("localhost") {
        return "127.0.0.1".to_string();
    }

    host.to_string()
}

pub async fn resolve_connection_target(
    config: &DbConnectionConfig,
) -> Result<ResolvedConnectionTarget, DbError> {
    if !config.get_param_bool(SSH_TUNNEL_ENABLED) {
        return Ok(ResolvedConnectionTarget {
            host: normalize_direct_host(&config.host),
            port: config.port,
            tunnel: None,
        });
    }

    let ssh_host = required_param(config, SSH_HOST)?;
    let ssh_port = optional_u16_param(config, SSH_PORT).unwrap_or(22);
    let ssh_username = required_param(config, SSH_USERNAME)?;
    let auth = build_auth(config)?;
    let target_host = config
        .get_param(SSH_TARGET_HOST)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| config.host.clone());
    let target_port = optional_u16_param(config, SSH_TARGET_PORT).unwrap_or(config.port);

    // Read the SSH connection timeout from config.
    let ssh_timeout_secs = config
        .get_param_as::<u64>(SSH_TIMEOUT)
        .unwrap_or(DEFAULT_SSH_TIMEOUT_SECS);

    let ssh_config = SshConnectConfig {
        host: ssh_host,
        port: ssh_port,
        username: ssh_username,
        auth,
        timeout: Some(Duration::from_secs(ssh_timeout_secs)),
        keepalive_interval: None,
        keepalive_max: None,
        jump_server: None,
        proxy: None,
    };

    // Wrap tunnel setup with an explicit timeout.
    let tunnel_result = timeout(
        Duration::from_secs(ssh_timeout_secs),
        start_local_port_forward(ssh_config, target_host, target_port),
    )
    .await;

    let tunnel = match tunnel_result {
        Ok(Ok(tunnel)) => tunnel,
        Ok(Err(e)) => {
            return Err(DbError::connection(format!(
                "failed to establish ssh tunnel: {e}"
            )));
        }
        Err(_) => {
            return Err(DbError::connection(format!(
                "ssh tunnel connection timed out after {}s",
                ssh_timeout_secs
            )));
        }
    };

    let local_addr = tunnel.local_addr();

    Ok(ResolvedConnectionTarget {
        host: local_addr.ip().to_string(),
        port: local_addr.port(),
        tunnel: Some(tunnel),
    })
}

fn required_param(config: &DbConnectionConfig, key: &str) -> Result<String, DbError> {
    let value = config
        .get_param(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    match value {
        Some(value) => Ok(value.to_string()),
        None => Err(DbError::connection(format!(
            "ssh tunnel enabled but `{key}` is missing"
        ))),
    }
}

fn optional_u16_param(config: &DbConnectionConfig, key: &str) -> Option<u16> {
    config
        .get_param(key)
        .and_then(|value| value.trim().parse::<u16>().ok())
}

fn build_auth(config: &DbConnectionConfig) -> Result<SshAuth, DbError> {
    let auth_type = config
        .get_param(SSH_AUTH_TYPE)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "password".to_string());

    match auth_type.as_str() {
        "agent" => Ok(SshAuth::Agent),
        "private_key" => {
            let key_path = required_param(config, SSH_PRIVATE_KEY_PATH)?;
            let passphrase = config
                .get_param(SSH_PRIVATE_KEY_PASSPHRASE)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            Ok(SshAuth::PrivateKey {
                key_path,
                passphrase,
                certificate_path: None,
            })
        }
        _ => {
            let password = required_param(config, SSH_PASSWORD)?;
            Ok(SshAuth::Password(password))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use one_core::storage::DatabaseType;
    use std::collections::HashMap;

    fn build_config(extra_params: HashMap<String, String>) -> DbConnectionConfig {
        DbConnectionConfig {
            id: "test".to_string(),
            database_type: DatabaseType::MySQL,
            name: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3306,
            username: "root".to_string(),
            password: "password".to_string(),
            database: None,
            service_name: None,
            sid: None,
            workspace_id: None,
            extra_params,
        }
    }

    #[test]
    fn build_auth_supports_agent_type() {
        let mut extra_params = HashMap::new();
        extra_params.insert(SSH_AUTH_TYPE.to_string(), "agent".to_string());
        let config = build_config(extra_params);

        let auth = build_auth(&config).expect("agent auth type should parse successfully");

        assert!(matches!(auth, SshAuth::Agent));
    }
}

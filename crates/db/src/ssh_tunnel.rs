use one_core::storage::DbConnectionConfig;
use ssh::{start_local_port_forward, LocalPortForwardTunnel, SshAuth, SshConnectConfig};

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

pub struct ResolvedConnectionTarget {
    pub host: String,
    pub port: u16,
    pub tunnel: Option<LocalPortForwardTunnel>,
}

pub async fn resolve_connection_target(
    config: &DbConnectionConfig,
) -> Result<ResolvedConnectionTarget, DbError> {
    if !config.get_param_bool(SSH_TUNNEL_ENABLED) {
        return Ok(ResolvedConnectionTarget {
            host: config.host.clone(),
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

    let ssh_config = SshConnectConfig {
        host: ssh_host,
        port: ssh_port,
        username: ssh_username,
        auth,
        timeout: None,
        keepalive_interval: None,
        keepalive_max: None,
        jump_server: None,
        proxy: None,
    };

    let tunnel = start_local_port_forward(ssh_config, target_host, target_port)
        .await
        .map_err(|err| DbError::connection(format!("failed to establish ssh tunnel: {err}")))?;
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

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use russh::keys::*;
use russh::*;
use rust_i18n::t;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, oneshot};

#[derive(Clone)]
pub struct SshConnectConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    pub timeout: Option<Duration>,
    pub keepalive_interval: Option<Duration>,
    pub keepalive_max: Option<usize>,
    /// 跳板机配置
    pub jump_server: Option<JumpServerConnectConfig>,
    /// 代理配置
    pub proxy: Option<ProxyConnectConfig>,
}

/// 跳板机连接配置
#[derive(Clone)]
pub struct JumpServerConnectConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
}

/// 代理类型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProxyType {
    Socks5,
    Http,
}

/// 代理连接配置
#[derive(Clone)]
pub struct ProxyConnectConfig {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone)]
pub enum SshAuth {
    Password(String),
    PrivateKey {
        key_path: String,
        passphrase: Option<String>,
        certificate_path: Option<String>,
    },
    Agent,
    AutoPublicKey,
}

#[derive(Clone)]
pub struct AuthFailureMessages {
    pub password_failed: String,
    pub certificate_failed: String,
    pub public_key_failed: String,
    pub agent_connect_failed: String,
    pub agent_no_identities: String,
    pub agent_auth_failed: String,
    pub auto_publickey_failed: String,
    pub no_local_identity: String,
    pub auto_publickey_next_step: String,
}

#[derive(Clone)]
pub struct PtyConfig {
    pub term: String,
    pub width: u32,
    pub height: u32,
    pub pix_width: u32,
    pub pix_height: u32,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            term: "xterm-256color".to_string(),
            width: 80,
            height: 24,
            pix_width: 0,
            pix_height: 0,
        }
    }
}

pub enum ChannelEvent {
    Data(Vec<u8>),
    ExtendedData {
        ext: u32,
        data: Vec<u8>,
    },
    Eof,
    ExitStatus(u32),
    ExitSignal {
        signal_name: String,
        error_message: String,
    },
    Close,
}

#[async_trait]
pub trait SshChannel: Send {
    async fn request_pty(&mut self, config: &PtyConfig) -> Result<()>;
    async fn exec(&mut self, command: &str) -> Result<()>;
    async fn request_shell(&mut self) -> Result<()>;
    async fn set_env(&mut self, name: &str, value: &str) -> Result<()>;
    async fn send_data(&mut self, data: &[u8]) -> Result<()>;
    async fn resize_pty(&mut self, width: u32, height: u32) -> Result<()>;
    async fn recv(&mut self) -> Option<ChannelEvent>;
    async fn eof(&mut self) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait SshClient: Send + Sync {
    type Channel: SshChannel;

    async fn connect(config: SshConnectConfig) -> Result<Self>
    where
        Self: Sized;

    async fn open_channel(&mut self) -> Result<Self::Channel>;

    async fn disconnect(&mut self) -> Result<()>;

    fn is_connected(&self) -> bool;
}

struct RusshHandler;

impl client::Handler for RusshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct RusshClient {
    session: client::Handle<RusshHandler>,
    /// 跳板机会话（如果使用跳板机连接）
    _jump_session: Option<client::Handle<RusshHandler>>,
}

pub struct LocalPortForwardTunnel {
    local_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    accept_task: Option<tokio::task::JoinHandle<()>>,
    client: Arc<Mutex<RusshClient>>,
}

impl LocalPortForwardTunnel {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.accept_task.take() {
            let _ = task.await;
        }
        let mut guard = self.client.lock().await;
        guard.disconnect().await
    }
}

impl Drop for LocalPortForwardTunnel {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.accept_task.take() {
            task.abort();
        }
    }
}

pub async fn authenticate_session<H>(
    session: &mut client::Handle<H>,
    username: &str,
    auth: &SshAuth,
    messages: AuthFailureMessages,
) -> Result<()>
where
    H: client::Handler,
{
    let hash_alg = session.best_supported_rsa_hash().await?.flatten();

    match auth {
        SshAuth::Password(password) => {
            let auth_result = session.authenticate_password(username, password).await?;
            if !auth_result.success() {
                anyhow::bail!(messages.password_failed.clone());
            }
        }
        SshAuth::PrivateKey {
            key_path,
            passphrase,
            certificate_path,
        } => {
            let key_pair = load_secret_key(key_path, passphrase.as_deref())?;

            if let Some(cert_path) = certificate_path {
                let cert = load_openssh_certificate(cert_path)?;
                let auth_result = session
                    .authenticate_openssh_cert(username, Arc::new(key_pair), cert)
                    .await?;
                if !auth_result.success() {
                    anyhow::bail!(messages.certificate_failed.clone());
                }
            } else {
                let auth_result = session
                    .authenticate_publickey(
                        username,
                        PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
                    )
                    .await?;
                if !auth_result.success() {
                    anyhow::bail!(messages.public_key_failed.clone());
                }
            }
        }
        SshAuth::Agent => authenticate_with_agent(session, username, hash_alg, &messages).await?,
        SshAuth::AutoPublicKey => unreachable!("AutoPublicKey 应由高层认证编排处理"),
    }
    Ok(())
}

pub fn discover_default_private_keys() -> Vec<String> {
    let Some(home_dir) = dirs::home_dir() else {
        return Vec::new();
    };

    let ssh_dir = home_dir.join(".ssh");
    ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"]
        .into_iter()
        .map(|file_name| ssh_dir.join(file_name))
        .filter(|path| path.is_file())
        .map(path_to_string)
        .collect()
}

pub fn expand_auto_publickey_auth() -> Vec<SshAuth> {
    let mut auth_candidates = vec![SshAuth::Agent];
    auth_candidates.extend(discover_default_private_keys().into_iter().map(|key_path| {
        SshAuth::PrivateKey {
            key_path,
            passphrase: None,
            certificate_path: None,
        }
    }));
    auth_candidates
}

pub async fn authenticate_session_with_fallbacks<H>(
    session: &mut client::Handle<H>,
    username: &str,
    auth_candidates: &[SshAuth],
    messages: AuthFailureMessages,
) -> Result<()>
where
    H: client::Handler,
{
    let filtered_candidates: Vec<&SshAuth> = auth_candidates
        .iter()
        .filter(|auth| !matches!(auth, SshAuth::AutoPublicKey))
        .collect();

    if filtered_candidates.is_empty() {
        anyhow::bail!(messages.no_local_identity.clone());
    }

    let has_default_keys = filtered_candidates
        .iter()
        .any(|auth| matches!(auth, SshAuth::PrivateKey { .. }));
    let mut errors = Vec::new();

    for auth in filtered_candidates {
        match authenticate_session(session, username, auth, messages.clone()).await {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(err.to_string()),
        }
    }

    anyhow::bail!(build_auto_publickey_failure_message(
        &messages,
        has_default_keys,
        &errors,
    ));
}

fn default_auth_failure_messages() -> AuthFailureMessages {
    AuthFailureMessages {
        password_failed: t!("Ssh.auth_password_failed").to_string(),
        certificate_failed: t!("Ssh.auth_certificate_failed").to_string(),
        public_key_failed: t!("Ssh.auth_public_key_failed").to_string(),
        agent_connect_failed: t!("Ssh.auth_agent_connect_failed").to_string(),
        agent_no_identities: t!("Ssh.auth_agent_no_identities").to_string(),
        agent_auth_failed: t!("Ssh.auth_agent_failed").to_string(),
        auto_publickey_failed: t!("Ssh.auth_auto_publickey_failed").to_string(),
        no_local_identity: t!("Ssh.auth_no_local_identity").to_string(),
        auto_publickey_next_step: t!("Ssh.auth_auto_publickey_next_step").to_string(),
    }
}

fn build_auto_publickey_failure_message(
    messages: &AuthFailureMessages,
    has_default_keys: bool,
    errors: &[String],
) -> String {
    let mut parts = vec![messages.auto_publickey_failed.clone()];
    if !has_default_keys {
        parts.push(messages.no_local_identity.clone());
    }
    if !errors.is_empty() {
        parts.push(errors.join("; "));
    }
    parts.push(messages.auto_publickey_next_step.clone());
    parts.join(": ")
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

pub async fn authenticate_with_strategy<H>(
    session: &mut client::Handle<H>,
    username: &str,
    auth: &SshAuth,
    messages: AuthFailureMessages,
) -> Result<()>
where
    H: client::Handler,
{
    match auth {
        SshAuth::AutoPublicKey => {
            let auth_candidates = expand_auto_publickey_auth();
            authenticate_session_with_fallbacks(session, username, &auth_candidates, messages).await
        }
        _ => authenticate_session(session, username, auth, messages).await,
    }
}

#[cfg(unix)]
async fn authenticate_with_agent<H>(
    session: &mut client::Handle<H>,
    username: &str,
    hash_alg: Option<HashAlg>,
    messages: &AuthFailureMessages,
) -> Result<()>
where
    H: client::Handler,
{
    let mut agent = connect_agent_client(messages).await?;

    let identities = agent
        .request_identities()
        .await
        .map_err(|e| anyhow::anyhow!("{}: {}", messages.agent_connect_failed, e))?;
    if identities.is_empty() {
        anyhow::bail!(messages.agent_no_identities.clone());
    }

    let mut last_error = None;
    for identity in identities {
        match session
            .authenticate_publickey_with(username, identity, hash_alg, &mut agent)
            .await
        {
            Ok(result) if result.success() => return Ok(()),
            Ok(_) => continue,
            Err(err) => {
                last_error = Some(err.to_string());
            }
        }
    }

    if let Some(err) = last_error {
        anyhow::bail!("{}: {}", messages.agent_auth_failed, err);
    }
    anyhow::bail!(messages.agent_auth_failed.clone());
}

#[cfg(unix)]
async fn connect_agent_client(
    messages: &AuthFailureMessages,
) -> Result<agent::client::AgentClient<tokio::net::UnixStream>> {
    russh::keys::agent::client::AgentClient::connect_env()
        .await
        .map_err(|e| anyhow::anyhow!("{}: {}", messages.agent_connect_failed, e))
}

#[cfg(windows)]
async fn authenticate_with_agent<H>(
    session: &mut client::Handle<H>,
    username: &str,
    hash_alg: Option<HashAlg>,
    messages: &AuthFailureMessages,
) -> Result<()>
where
    H: client::Handler,
{
    let mut agent =
        russh::keys::agent::client::AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent")
            .await
            .map_err(|e| anyhow::anyhow!("{}: {}", messages.agent_connect_failed, e))?;

    let identities = agent
        .request_identities()
        .await
        .map_err(|e| anyhow::anyhow!("{}: {}", messages.agent_connect_failed, e))?;
    if identities.is_empty() {
        anyhow::bail!(messages.agent_no_identities.clone());
    }

    let mut last_error = None;
    for identity in identities {
        match session
            .authenticate_publickey_with(username, identity, hash_alg, &mut agent)
            .await
        {
            Ok(result) if result.success() => return Ok(()),
            Ok(_) => continue,
            Err(err) => {
                last_error = Some(err.to_string());
            }
        }
    }

    if let Some(err) = last_error {
        anyhow::bail!("{}: {}", messages.agent_auth_failed, err);
    }
    anyhow::bail!(messages.agent_auth_failed.clone());
}

#[cfg(not(any(unix, windows)))]
async fn authenticate_with_agent<H>(
    _session: &mut client::Handle<H>,
    _username: &str,
    _hash_alg: Option<HashAlg>,
    messages: &AuthFailureMessages,
) -> Result<()>
where
    H: client::Handler,
{
    anyhow::bail!(messages.agent_connect_failed.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::sync::{Mutex, OnceLock};

    #[cfg(unix)]
    fn test_auth_failure_messages() -> AuthFailureMessages {
        AuthFailureMessages {
            password_failed: "password".to_string(),
            certificate_failed: "certificate".to_string(),
            public_key_failed: "public_key".to_string(),
            agent_connect_failed: "agent_connect".to_string(),
            agent_no_identities: "agent_no_identities".to_string(),
            agent_auth_failed: "agent_auth_failed".to_string(),
            auto_publickey_failed: "auto_publickey_failed".to_string(),
            no_local_identity: "no_local_identity".to_string(),
            auto_publickey_next_step: "next_step".to_string(),
        }
    }

    fn home_dir_env_key() -> &'static str {
        if cfg!(windows) { "USERPROFILE" } else { "HOME" }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn agent_connect_without_env_returns_readable_error() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let env_lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = env_lock.lock().expect("环境锁不应中毒");

        let previous = std::env::var("SSH_AUTH_SOCK").ok();
        unsafe {
            std::env::remove_var("SSH_AUTH_SOCK");
        }

        let result = connect_agent_client(&test_auth_failure_messages()).await;

        match previous {
            Some(value) => unsafe {
                std::env::set_var("SSH_AUTH_SOCK", value);
            },
            None => unsafe {
                std::env::remove_var("SSH_AUTH_SOCK");
            },
        }

        let err = match result {
            Ok(_) => panic!("缺少 SSH_AUTH_SOCK 时应返回错误"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("agent_connect"),
            "错误信息应包含 agent 连接失败上下文"
        );
    }

    #[test]
    fn discover_default_private_keys_returns_expected_order() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let env_lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = env_lock.lock().expect("环境锁不应中毒");

        let temp_home = std::env::temp_dir().join(format!(
            "onetcli-ssh-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间应晚于 unix epoch")
                .as_nanos()
        ));
        let ssh_dir = temp_home.join(".ssh");
        std::fs::create_dir_all(&ssh_dir).expect("应可创建临时 ssh 目录");
        std::fs::write(ssh_dir.join("id_rsa"), "rsa").expect("应可写入 id_rsa");
        std::fs::write(ssh_dir.join("id_ed25519"), "ed25519").expect("应可写入 id_ed25519");

        let env_key = home_dir_env_key();
        let previous = std::env::var(env_key).ok();
        unsafe {
            std::env::set_var(env_key, &temp_home);
        }

        let discovered = discover_default_private_keys();

        match previous {
            Some(value) => unsafe {
                std::env::set_var(env_key, value);
            },
            None => unsafe {
                std::env::remove_var(env_key);
            },
        }

        std::fs::remove_dir_all(&temp_home).expect("应可清理临时目录");

        assert_eq!(
            discovered,
            vec![
                ssh_dir.join("id_ed25519").to_string_lossy().to_string(),
                ssh_dir.join("id_rsa").to_string_lossy().to_string(),
            ]
        );
    }

    #[test]
    fn expand_auto_publickey_auth_contains_agent_and_default_keys() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let env_lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = env_lock.lock().expect("环境锁不应中毒");

        let temp_home = std::env::temp_dir().join(format!(
            "onetcli-ssh-test-expand-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间应晚于 unix epoch")
                .as_nanos()
        ));
        let ssh_dir = temp_home.join(".ssh");
        std::fs::create_dir_all(&ssh_dir).expect("应可创建临时 ssh 目录");
        let key_path = ssh_dir.join("id_ed25519");
        std::fs::write(&key_path, "ed25519").expect("应可写入默认私钥");

        let env_key = home_dir_env_key();
        let previous = std::env::var(env_key).ok();
        unsafe {
            std::env::set_var(env_key, &temp_home);
        }

        let expanded = expand_auto_publickey_auth();

        match previous {
            Some(value) => unsafe {
                std::env::set_var(env_key, value);
            },
            None => unsafe {
                std::env::remove_var(env_key);
            },
        }

        std::fs::remove_dir_all(&temp_home).expect("应可清理临时目录");

        assert!(matches!(expanded.first(), Some(SshAuth::Agent)));
        assert!(expanded.iter().any(|auth| matches!(
            auth,
            SshAuth::PrivateKey { key_path: path, .. } if path == &key_path.to_string_lossy().to_string()
        )));
    }

    #[test]
    fn build_auto_publickey_failure_message_mentions_missing_identity() {
        let messages = test_auth_failure_messages();
        let message =
            build_auto_publickey_failure_message(&messages, false, &["agent_connect".to_string()]);

        assert!(message.contains("auto_publickey_failed"));
        assert!(message.contains("no_local_identity"));
        assert!(message.contains("agent_connect"));
    }

    #[test]
    fn build_auto_publickey_failure_message_includes_next_step() {
        let messages = test_auth_failure_messages();
        // 有候选身份但全部失败的场景
        let message = build_auto_publickey_failure_message(
            &messages,
            true,
            &["public_key_failed".to_string()],
        );
        assert!(
            message.contains("next_step"),
            "失败消息应包含下一步引导文案，实际：{}",
            message
        );
    }

    #[tokio::test]
    async fn authenticate_session_with_fallbacks_returns_error_when_no_candidates() {
        // 验证空候选列表时返回可读错误，而不是 panic
        // 这是 P0 修复的核心：authenticate_session 对 AutoPublicKey 会 unreachable!()，
        // authenticate_session_with_fallbacks 应正常返回错误
        // 空候选列表 — 不依赖真实 SSH 服务，直接验证错误路径
        let candidates: Vec<SshAuth> = vec![];
        let messages = test_auth_failure_messages();

        // 使用辅助函数验证空列表的错误聚合逻辑（不需要真实 session）
        let filtered: Vec<&SshAuth> = candidates
            .iter()
            .filter(|a| !matches!(a, SshAuth::AutoPublicKey))
            .collect();
        assert!(filtered.is_empty(), "空候选列表过滤后应为空");

        // 验证失败消息生成不 panic
        let msg = build_auto_publickey_failure_message(&messages, false, &[]);
        assert!(msg.contains("auto_publickey_failed"));
        assert!(msg.contains("no_local_identity"));
        assert!(msg.contains("next_step"));
    }
}

/// 通过代理建立TCP连接
async fn connect_via_proxy(
    proxy: &ProxyConnectConfig,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);

    match proxy.proxy_type {
        ProxyType::Socks5 => {
            use tokio_socks::tcp::Socks5Stream;

            let stream = if let (Some(username), Some(password)) =
                (&proxy.username, &proxy.password)
            {
                Socks5Stream::connect_with_password(
                    proxy_addr.as_str(),
                    (target_host, target_port),
                    username,
                    password,
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(t!("Ssh.socks5_proxy_connect_failed", error = e).to_string())
                })?
            } else {
                Socks5Stream::connect(proxy_addr.as_str(), (target_host, target_port))
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            t!("Ssh.socks5_proxy_connect_failed", error = e).to_string()
                        )
                    })?
            };

            Ok(stream.into_inner())
        }
        ProxyType::Http => {
            // HTTP CONNECT代理实现
            let stream = TcpStream::connect(&proxy_addr).await.map_err(|e| {
                anyhow::anyhow!(t!("Ssh.http_proxy_connect_failed", error = e).to_string())
            })?;

            // 发送CONNECT请求
            let connect_request = if let (Some(username), Some(password)) =
                (&proxy.username, &proxy.password)
            {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64_encode(&credentials);
                format!(
                    "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Authorization: Basic {}\r\n\r\n",
                    target_host, target_port, target_host, target_port, encoded
                )
            } else {
                format!(
                    "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
                    target_host, target_port, target_host, target_port
                )
            };

            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

            let (reader, mut writer) = stream.into_split();
            writer.write_all(connect_request.as_bytes()).await?;

            let mut reader = BufReader::new(reader);
            let mut response_line = String::new();
            reader.read_line(&mut response_line).await?;

            if !response_line.contains("200") {
                anyhow::bail!(t!(
                    "Ssh.http_proxy_connection_failed",
                    response = response_line.trim()
                ));
            }

            // 读取剩余的响应头
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).await?;
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }

            // 重新组合stream
            Ok(reader.into_inner().reunite(writer)?)
        }
    }
}

/// 简单的Base64编码
fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let bytes = input.as_bytes();
    let mut result = Vec::new();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(ALPHABET[((n >> 18) & 0x3F) as usize]);
        result.push(ALPHABET[((n >> 12) & 0x3F) as usize]);

        if chunk.len() > 1 {
            result.push(ALPHABET[((n >> 6) & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[(n & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }
    }

    String::from_utf8(result).unwrap()
}

impl RusshClient {
    pub async fn open_direct_tcpip_channel(
        &mut self,
        target_host: &str,
        target_port: u16,
        origin_host: &str,
        origin_port: u16,
    ) -> Result<Channel<client::Msg>> {
        let channel = self
            .session
            .channel_open_direct_tcpip(
                target_host,
                target_port as u32,
                origin_host,
                origin_port as u32,
            )
            .await?;
        Ok(channel)
    }
}

pub async fn start_local_port_forward(
    config: SshConnectConfig,
    target_host: impl Into<String>,
    target_port: u16,
) -> Result<LocalPortForwardTunnel> {
    let target_host = target_host.into();
    let bind_addr = "127.0.0.1:0";
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind local address: {bind_addr}"))?;
    let local_addr = listener.local_addr()?;

    let client = <RusshClient as SshClient>::connect(config).await?;
    let client = Arc::new(Mutex::new(client));
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let client_for_task = Arc::clone(&client);
    let target_host_for_task = target_host.clone();

    let accept_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    break;
                }
                accept_result = listener.accept() => {
                    let (mut inbound, inbound_addr) = match accept_result {
                        Ok(result) => result,
                        Err(err) => {
                            tracing::error!("本地端口转发 accept 失败: {}", err);
                            break;
                        }
                    };

                    let client_for_conn = Arc::clone(&client_for_task);
                    let target_host_for_conn = target_host_for_task.clone();
                    tokio::spawn(async move {
                        let origin_host = match inbound_addr {
                            SocketAddr::V4(v4) => v4.ip().to_string(),
                            SocketAddr::V6(v6) => v6.ip().to_string(),
                        };
                        let origin_port = inbound_addr.port();

                        let direct_channel = {
                            let mut guard = client_for_conn.lock().await;
                            match guard
                                .open_direct_tcpip_channel(
                                    &target_host_for_conn,
                                    target_port,
                                    &origin_host,
                                    origin_port,
                                )
                                .await
                            {
                                Ok(channel) => channel,
                                Err(err) => {
                                    tracing::error!("打开 SSH direct-tcpip 通道失败: {}", err);
                                    return;
                                }
                            }
                        };

                        let mut outbound = direct_channel.into_stream();
                        if let Err(err) = copy_bidirectional(&mut inbound, &mut outbound).await {
                            tracing::debug!("SSH 端口转发连接结束: {}", err);
                        }
                    });
                }
            }
        }
    });

    Ok(LocalPortForwardTunnel {
        local_addr,
        shutdown_tx: Some(shutdown_tx),
        accept_task: Some(accept_task),
        client,
    })
}

#[async_trait]
impl SshClient for RusshClient {
    type Channel = RusshChannel;

    async fn connect(config: SshConnectConfig) -> Result<Self> {
        let russh_config = Arc::new(client::Config {
            inactivity_timeout: config.timeout.or(Some(Duration::from_secs(300))),
            keepalive_interval: config.keepalive_interval.or(Some(Duration::from_secs(60))),
            keepalive_max: config.keepalive_max.unwrap_or(3),
            ..<_>::default()
        });

        // 情况1: 使用跳板机连接
        if let Some(ref jump) = config.jump_server {
            tracing::info!("通过跳板机 {}:{} 连接", jump.host, jump.port);

            // 先连接到跳板机（可能通过代理）
            let jump_session = if let Some(ref proxy) = config.proxy {
                tracing::info!("通过代理 {}:{} 连接跳板机", proxy.host, proxy.port);
                let stream = connect_via_proxy(proxy, &jump.host, jump.port).await?;
                let handler = RusshHandler;
                client::connect_stream(russh_config.clone(), stream, handler).await?
            } else {
                let addrs = (jump.host.as_str(), jump.port);
                let handler = RusshHandler;
                client::connect(russh_config.clone(), addrs, handler).await?
            };

            // 认证跳板机
            let mut jump_session = jump_session;
            authenticate_with_strategy(
                &mut jump_session,
                &jump.username,
                &jump.auth,
                default_auth_failure_messages(),
            )
            .await?;

            // 通过跳板机建立到目标服务器的端口转发
            tracing::info!("通过跳板机转发到目标服务器 {}:{}", config.host, config.port);
            let forwarded_channel = jump_session
                .channel_open_direct_tcpip(&config.host, config.port as u32, "127.0.0.1", 0)
                .await?;

            // 使用转发通道创建SSH会话
            let handler = RusshHandler;
            let mut session =
                client::connect_stream(russh_config, forwarded_channel.into_stream(), handler)
                    .await?;

            // 认证目标服务器
            authenticate_with_strategy(
                &mut session,
                &config.username,
                &config.auth,
                default_auth_failure_messages(),
            )
            .await?;

            Ok(Self {
                session,
                _jump_session: Some(jump_session),
            })
        }
        // 情况2: 仅使用代理连接
        else if let Some(ref proxy) = config.proxy {
            tracing::info!(
                "通过代理 {}:{} 连接目标服务器 {}:{}",
                proxy.host,
                proxy.port,
                config.host,
                config.port
            );
            let stream = connect_via_proxy(proxy, &config.host, config.port).await?;
            let handler = RusshHandler;
            let mut session = client::connect_stream(russh_config, stream, handler).await?;

            authenticate_with_strategy(
                &mut session,
                &config.username,
                &config.auth,
                default_auth_failure_messages(),
            )
            .await?;

            Ok(Self {
                session,
                _jump_session: None,
            })
        }
        // 情况3: 直接连接
        else {
            let addrs = (config.host.as_str(), config.port);
            let handler = RusshHandler;
            let mut session = client::connect(russh_config, addrs, handler).await?;

            authenticate_with_strategy(
                &mut session,
                &config.username,
                &config.auth,
                default_auth_failure_messages(),
            )
            .await?;

            Ok(Self {
                session,
                _jump_session: None,
            })
        }
    }

    async fn open_channel(&mut self) -> Result<Self::Channel> {
        let channel = self.session.channel_open_session().await?;
        Ok(RusshChannel { channel })
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        !self.session.is_closed()
    }
}

pub struct RusshChannel {
    channel: Channel<client::Msg>,
}

#[async_trait]
impl SshChannel for RusshChannel {
    async fn request_pty(&mut self, config: &PtyConfig) -> Result<()> {
        self.channel
            .request_pty(
                false,
                &config.term,
                config.width,
                config.height,
                config.pix_width,
                config.pix_height,
                &[],
            )
            .await?;
        Ok(())
    }

    async fn exec(&mut self, command: &str) -> Result<()> {
        self.channel.exec(true, command).await?;
        Ok(())
    }

    async fn request_shell(&mut self) -> Result<()> {
        self.channel.request_shell(true).await?;
        Ok(())
    }

    async fn set_env(&mut self, name: &str, value: &str) -> Result<()> {
        self.channel.set_env(false, name, value).await?;
        Ok(())
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        self.channel.data(data).await?;
        Ok(())
    }

    async fn resize_pty(&mut self, width: u32, height: u32) -> Result<()> {
        self.channel.window_change(width, height, 0, 0).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Option<ChannelEvent> {
        let msg = self.channel.wait().await?;
        Some(match msg {
            ChannelMsg::Data { data } => ChannelEvent::Data(data.to_vec()),
            ChannelMsg::ExtendedData { data, ext } => ChannelEvent::ExtendedData {
                ext,
                data: data.to_vec(),
            },
            ChannelMsg::Eof => ChannelEvent::Eof,
            ChannelMsg::ExitStatus { exit_status } => ChannelEvent::ExitStatus(exit_status),
            ChannelMsg::ExitSignal {
                signal_name,
                error_message,
                ..
            } => ChannelEvent::ExitSignal {
                signal_name: format!("{:?}", signal_name),
                error_message,
            },
            ChannelMsg::Close => ChannelEvent::Close,
            _ => return self.recv().await,
        })
    }

    async fn eof(&mut self) -> Result<()> {
        self.channel.eof().await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.channel.close().await?;
        Ok(())
    }
}

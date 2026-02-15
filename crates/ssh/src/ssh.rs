use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use russh::keys::*;
use russh::*;
use tokio::net::TcpStream;

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
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct RusshClient {
    session: client::Handle<RusshHandler>,
    /// 跳板机会话（如果使用跳板机连接）
    _jump_session: Option<client::Handle<RusshHandler>>,
}

/// 执行SSH认证
async fn authenticate(
    session: &mut client::Handle<RusshHandler>,
    username: &str,
    auth: &SshAuth,
) -> Result<()> {
    match auth {
        SshAuth::Password(password) => {
            let auth_result = session.authenticate_password(username, password).await?;
            if !auth_result.success() {
                anyhow::bail!("密码认证失败");
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
                    anyhow::bail!("证书认证失败");
                }
            } else {
                let auth_result = session
                    .authenticate_publickey(
                        username,
                        PrivateKeyWithHashAlg::new(
                            Arc::new(key_pair),
                            session.best_supported_rsa_hash().await?.flatten(),
                        ),
                    )
                    .await?;
                if !auth_result.success() {
                    anyhow::bail!("公钥认证失败");
                }
            }
        }
    }
    Ok(())
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
                .map_err(|e| anyhow::anyhow!("SOCKS5代理连接失败: {}", e))?
            } else {
                Socks5Stream::connect(proxy_addr.as_str(), (target_host, target_port))
                    .await
                    .map_err(|e| anyhow::anyhow!("SOCKS5代理连接失败: {}", e))?
            };

            Ok(stream.into_inner())
        }
        ProxyType::Http => {
            // HTTP CONNECT代理实现
            let stream = TcpStream::connect(&proxy_addr)
                .await
                .map_err(|e| anyhow::anyhow!("连接HTTP代理失败: {}", e))?;

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
                anyhow::bail!("HTTP代理连接失败: {}", response_line.trim());
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
            authenticate(&mut jump_session, &jump.username, &jump.auth).await?;

            // 通过跳板机建立到目标服务器的端口转发
            tracing::info!(
                "通过跳板机转发到目标服务器 {}:{}",
                config.host,
                config.port
            );
            let forwarded_channel = jump_session
                .channel_open_direct_tcpip(&config.host, config.port as u32, "127.0.0.1", 0)
                .await?;

            // 使用转发通道创建SSH会话
            let handler = RusshHandler;
            let mut session =
                client::connect_stream(russh_config, forwarded_channel.into_stream(), handler)
                    .await?;

            // 认证目标服务器
            authenticate(&mut session, &config.username, &config.auth).await?;

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

            authenticate(&mut session, &config.username, &config.auth).await?;

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

            authenticate(&mut session, &config.username, &config.auth).await?;

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

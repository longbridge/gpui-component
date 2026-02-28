use crate::{FileEntry, ProgressCallback, SftpClient, TransferCancelled, TransferProgress};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use russh::client::{self, Handle};
use russh::keys::*;
use russh_sftp::client::RawSftpSession;
use russh_sftp::client::SftpSession;
use russh_sftp::client::error::Error as SftpError;
use russh_sftp::client::rawsession::Limits;
use russh_sftp::protocol::{FileAttributes, OpenFlags, StatusCode};
use rust_i18n::t;
use ssh::{ProxyConnectConfig, ProxyType, SshConnectConfig};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

const BUFFER_SIZE: usize = 256 * 1024; // 256 KB
const PIPELINE_CHUNK_SIZE: u32 = 61440; // 60 KB per read request (within 65535 packet limit)
const MAX_INFLIGHT_REQUESTS: usize = 64; // 最多 64 个并发请求
const PIPELINE_THRESHOLD: u64 = 512 * 1024; // 超过 512 KB 的文件才走流水线

fn ensure_not_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(TransferCancelled.into());
    }
    Ok(())
}

struct SftpHandler;

impl client::Handler for SftpHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// 执行SSH认证
async fn sftp_authenticate(
    session: &mut Handle<SftpHandler>,
    username: &str,
    auth: &ssh::SshAuth,
) -> Result<()> {
    match auth {
        ssh::SshAuth::Password(password) => {
            let auth_result = session.authenticate_password(username, password).await?;
            if !auth_result.success() {
                anyhow::bail!(t!("Sftp.auth_password_failed"));
            }
        }
        ssh::SshAuth::PrivateKey {
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
                    anyhow::bail!(t!("Sftp.auth_certificate_failed"));
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
                    anyhow::bail!(t!("Sftp.auth_public_key_failed"));
                }
            }
        }
    }
    Ok(())
}

/// 通过代理建立TCP连接
async fn sftp_connect_via_proxy(
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
                    anyhow::anyhow!(t!("Sftp.socks5_proxy_connect_failed", error = e).to_string())
                })?
            } else {
                Socks5Stream::connect(proxy_addr.as_str(), (target_host, target_port))
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            t!("Sftp.socks5_proxy_connect_failed", error = e).to_string()
                        )
                    })?
            };

            Ok(stream.into_inner())
        }
        ProxyType::Http => {
            let stream = TcpStream::connect(&proxy_addr).await.map_err(|e| {
                anyhow::anyhow!(t!("Sftp.http_proxy_connect_failed", error = e).to_string())
            })?;

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

            use tokio::io::{AsyncBufReadExt, BufReader};

            let (reader, mut writer) = stream.into_split();
            writer.write_all(connect_request.as_bytes()).await?;

            let mut reader = BufReader::new(reader);
            let mut response_line = String::new();
            reader.read_line(&mut response_line).await?;

            if !response_line.contains("200") {
                anyhow::bail!(t!(
                    "Sftp.http_proxy_connection_failed",
                    response = response_line.trim()
                ));
            }

            loop {
                let mut line = String::new();
                reader.read_line(&mut line).await?;
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }

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

pub struct RusshSftpClient {
    sftp: SftpSession,
    session: Handle<SftpHandler>,
    /// 跳板机会话（如果使用跳板机连接）
    _jump_session: Option<Handle<SftpHandler>>,
    /// 懒初始化的原始 SFTP 会话，用于流水线下载
    raw_sftp: Option<Arc<RawSftpSession>>,
}

impl RusshSftpClient {
    /// 在已有 SSH 连接上创建一个新的 RawSftpSession 用于流水线操作
    async fn get_or_create_raw_session(&mut self) -> Result<Arc<RawSftpSession>> {
        if let Some(ref raw) = self.raw_sftp {
            return Ok(Arc::clone(raw));
        }

        let channel = self.session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;

        let mut raw = RawSftpSession::new(channel.into_stream());
        raw.init()
            .await
            .map_err(|e| anyhow!("Failed to init raw SFTP session: {}", e))?;

        // 尝试查询 limits@openssh.com 扩展并设置限制
        if let Ok(limits_ext) = raw.limits().await {
            let limits: Limits = limits_ext.into();
            raw.set_limits(Arc::new(limits));
        }

        raw.set_timeout(300).await;

        let raw = Arc::new(raw);
        self.raw_sftp = Some(Arc::clone(&raw));
        Ok(raw)
    }

    /// 流水线下载：通过 RawSftpSession 发起多个并发读请求
    async fn pipelined_download(
        raw_session: Arc<RawSftpSession>,
        remote_path: &str,
        local_path: &str,
        total_size: u64,
        cancelled: &AtomicBool,
        progress: &(dyn Fn(TransferProgress) + Send + Sync),
    ) -> Result<()> {
        // 打开远程文件
        let handle_result = raw_session
            .open(remote_path, OpenFlags::READ, FileAttributes::default())
            .await
            .map_err(|e| anyhow!("Failed to open remote file {}: {}", remote_path, e))?;
        let file_handle = handle_result.handle;

        let local_file = File::create(local_path)
            .await
            .map_err(|e| anyhow!("Failed to create local file {}: {}", local_path, e))?;
        let mut writer = BufWriter::with_capacity(BUFFER_SIZE, local_file);

        let chunk_size = PIPELINE_CHUNK_SIZE as u64;
        let total_chunks = total_size.div_ceil(chunk_size);

        let semaphore = Arc::new(Semaphore::new(MAX_INFLIGHT_REQUESTS));
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(u64, Vec<u8>)>(MAX_INFLIGHT_REQUESTS);

        // 生产者：发起所有并发读请求
        let raw_for_producer = Arc::clone(&raw_session);
        let handle_for_producer = file_handle.clone();
        let producer = tokio::spawn(async move {
            for i in 0..total_chunks {
                let offset = i * chunk_size;
                let len = std::cmp::min(PIPELINE_CHUNK_SIZE, (total_size - offset) as u32);

                let permit = semaphore.clone().acquire_owned().await;
                if permit.is_err() {
                    break;
                }
                let permit = permit.unwrap();

                let raw = Arc::clone(&raw_for_producer);
                let handle = handle_for_producer.clone();
                let tx = tx.clone();

                tokio::spawn(async move {
                    let result = raw.read(handle, offset, len).await;
                    drop(permit);

                    match result {
                        Ok(data) => {
                            let _ = tx.send((offset, data.data)).await;
                        }
                        Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => {
                            // EOF 表示文件读完，发送空数据标记此 offset
                            let _ = tx.send((offset, Vec::new())).await;
                        }
                        Err(_e) => {
                            // 读取错误，发送空数据让 writer 侧处理
                            let _ = tx.send((offset, Vec::new())).await;
                        }
                    }
                });
            }
            // 丢弃 producer 持有的 tx，让 rx 能在所有 spawn 的任务完成后结束
            drop(tx);
        });

        // 消费者：按顺序写入本地文件
        let mut pending: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
        let mut next_offset: u64 = 0;
        let mut transferred: u64 = 0;
        let mut last_update = Instant::now();
        let start_time = Instant::now();

        while let Some((offset, data)) = rx.recv().await {
            ensure_not_cancelled(cancelled)?;

            if !data.is_empty() {
                pending.insert(offset, data);
            } else if offset >= total_size {
                // EOF 超出文件范围，忽略
            } else {
                // 空数据但在范围内 — 可能是 EOF 在最后一个 chunk
                pending.insert(offset, Vec::new());
            }

            // 按顺序写入已就绪的 chunks
            while let Some(data) = pending.remove(&next_offset) {
                if !data.is_empty() {
                    writer
                        .write_all(&data)
                        .await
                        .map_err(|e| anyhow!("Failed to write to local file: {}", e))?;
                    transferred += data.len() as u64;
                }
                next_offset += chunk_size;
            }

            // 限制进度更新频率
            let now = Instant::now();
            if now.duration_since(last_update).as_millis() >= 100 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    transferred as f64 / elapsed
                } else {
                    0.0
                };

                progress(TransferProgress {
                    transferred,
                    total: total_size,
                    speed,
                    current_file: None,
                    current_file_transferred: 0,
                    current_file_total: 0,
                });
                last_update = now;
            }
        }

        // 等待 producer 完成
        let _ = producer.await;

        // 最终进度回调
        progress(TransferProgress {
            transferred,
            total: total_size,
            speed: 0.0,
            current_file: None,
            current_file_transferred: 0,
            current_file_total: 0,
        });

        writer
            .flush()
            .await
            .map_err(|e| anyhow!("Failed to flush local file: {}", e))?;
        writer
            .into_inner()
            .sync_all()
            .await
            .map_err(|e| anyhow!("Failed to sync local file: {}", e))?;

        // 关闭远程文件 handle
        let _ = raw_session.close(file_handle).await;

        Ok(())
    }

    /// 流水线下载（目录内文件），带 current_file 进度信息
    #[allow(clippy::too_many_arguments)]
    async fn pipelined_download_with_file_progress(
        raw_session: Arc<RawSftpSession>,
        remote_path: &str,
        local_path: &str,
        total_size: u64,
        file_name: &str,
        file_total: u64,
        dir_transferred: &mut u64,
        dir_total: u64,
        start_time: Instant,
        cancelled: &AtomicBool,
        progress: &(dyn Fn(TransferProgress) + Send + Sync),
    ) -> Result<()> {
        let handle_result = raw_session
            .open(remote_path, OpenFlags::READ, FileAttributes::default())
            .await
            .map_err(|e| anyhow!("Failed to open remote file {}: {}", remote_path, e))?;
        let file_handle = handle_result.handle;

        let local_file = File::create(local_path)
            .await
            .map_err(|e| anyhow!("Failed to create local file {}: {}", local_path, e))?;
        let mut writer = BufWriter::with_capacity(BUFFER_SIZE, local_file);

        let chunk_size = PIPELINE_CHUNK_SIZE as u64;
        let total_chunks = total_size.div_ceil(chunk_size);

        let semaphore = Arc::new(Semaphore::new(MAX_INFLIGHT_REQUESTS));
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(u64, Vec<u8>)>(MAX_INFLIGHT_REQUESTS);

        let raw_for_producer = Arc::clone(&raw_session);
        let handle_for_producer = file_handle.clone();

        let producer = tokio::spawn(async move {
            for i in 0..total_chunks {
                let offset = i * chunk_size;
                let len = std::cmp::min(PIPELINE_CHUNK_SIZE, (total_size - offset) as u32);

                let permit = semaphore.clone().acquire_owned().await;
                if permit.is_err() {
                    break;
                }
                let permit = permit.unwrap();

                let raw = Arc::clone(&raw_for_producer);
                let handle = handle_for_producer.clone();
                let tx = tx.clone();

                tokio::spawn(async move {
                    let result = raw.read(handle, offset, len).await;
                    drop(permit);

                    match result {
                        Ok(data) => {
                            let _ = tx.send((offset, data.data)).await;
                        }
                        Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => {
                            let _ = tx.send((offset, Vec::new())).await;
                        }
                        Err(_e) => {
                            let _ = tx.send((offset, Vec::new())).await;
                        }
                    }
                });
            }
            drop(tx);
        });

        let mut pending: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
        let mut next_offset: u64 = 0;
        let mut current_file_transferred: u64 = 0;

        while let Some((offset, data)) = rx.recv().await {
            ensure_not_cancelled(cancelled)?;

            if !data.is_empty() {
                pending.insert(offset, data);
            } else {
                pending.insert(offset, Vec::new());
            }

            while let Some(data) = pending.remove(&next_offset) {
                if !data.is_empty() {
                    writer
                        .write_all(&data)
                        .await
                        .map_err(|e| anyhow!("Failed to write to local file: {}", e))?;
                    let bytes = data.len() as u64;
                    *dir_transferred += bytes;
                    current_file_transferred += bytes;
                }
                next_offset += chunk_size;
            }

            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                *dir_transferred as f64 / elapsed
            } else {
                0.0
            };

            progress(TransferProgress {
                transferred: *dir_transferred,
                total: dir_total,
                speed,
                current_file: Some(file_name.to_string()),
                current_file_transferred,
                current_file_total: file_total,
            });
        }

        let _ = producer.await;

        writer
            .flush()
            .await
            .map_err(|e| anyhow!("Failed to flush local file: {}", e))?;
        writer
            .into_inner()
            .sync_all()
            .await
            .map_err(|e| anyhow!("Failed to sync local file: {}", e))?;

        let _ = raw_session.close(file_handle).await;

        Ok(())
    }

    /// 串行下载（小文件或 raw session 不可用时的后备）
    async fn serial_download_file(
        &mut self,
        remote_path: &str,
        local_path: &str,
        total_size: u64,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()> {
        let mut remote_file = self
            .sftp
            .open_with_flags(remote_path, OpenFlags::READ)
            .await
            .map_err(|e| anyhow!("Failed to open remote file {}: {}", remote_path, e))?;

        let local_file = File::create(local_path)
            .await
            .map_err(|e| anyhow!("Failed to create local file {}: {}", local_path, e))?;
        let mut local_file = BufWriter::with_capacity(BUFFER_SIZE, local_file);

        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut transferred = 0u64;
        let mut last_update = Instant::now();
        let mut speed_samples: Vec<f64> = Vec::new();

        loop {
            ensure_not_cancelled(&cancelled)?;
            let bytes_read = remote_file
                .read(&mut buffer)
                .await
                .map_err(|e| anyhow!("Failed to read from remote file: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            local_file
                .write_all(&buffer[..bytes_read])
                .await
                .map_err(|e| anyhow!("Failed to write to local file: {}", e))?;

            transferred += bytes_read as u64;

            let now = Instant::now();
            let elapsed = now.duration_since(last_update).as_secs_f64();

            if elapsed >= 0.1 {
                let speed = bytes_read as f64 / elapsed;
                speed_samples.push(speed);
                if speed_samples.len() > 10 {
                    speed_samples.remove(0);
                }

                let avg_speed = speed_samples.iter().sum::<f64>() / speed_samples.len() as f64;

                progress(TransferProgress {
                    transferred,
                    total: total_size,
                    speed: avg_speed,
                    current_file: None,
                    current_file_transferred: 0,
                    current_file_total: 0,
                });

                last_update = now;
            }
        }

        progress(TransferProgress {
            transferred,
            total: total_size,
            speed: 0.0,
            current_file: None,
            current_file_transferred: 0,
            current_file_total: 0,
        });

        local_file
            .flush()
            .await
            .map_err(|e| anyhow!("Failed to flush local file: {}", e))?;
        local_file
            .into_inner()
            .sync_all()
            .await
            .map_err(|e| anyhow!("Failed to sync local file: {}", e))?;

        Ok(())
    }
}

#[async_trait]
impl SftpClient for RusshSftpClient {
    async fn connect(ssh_config: SshConnectConfig) -> Result<Self> {
        let config = Arc::new(client::Config {
            inactivity_timeout: ssh_config.timeout.or(Some(Duration::from_secs(300))),
            keepalive_interval: ssh_config
                .keepalive_interval
                .or(Some(Duration::from_secs(60))),
            keepalive_max: ssh_config.keepalive_max.unwrap_or(3),
            window_size: 16 * 1024 * 1024, // 16 MB
            maximum_packet_size: 0xFFFF,   // 65535, max allowed by russh
            nodelay: true,
            ..<_>::default()
        });

        let (mut session, jump_session) = if let Some(ref jump) = ssh_config.jump_server {
            tracing::info!("SFTP: 通过跳板机 {}:{} 连接", jump.host, jump.port);

            // 连接跳板机
            let mut jump_session = if let Some(ref proxy) = ssh_config.proxy {
                tracing::info!("SFTP: 通过代理 {}:{} 连接跳板机", proxy.host, proxy.port);
                let stream = sftp_connect_via_proxy(proxy, &jump.host, jump.port).await?;
                let handler = SftpHandler;
                client::connect_stream(config.clone(), stream, handler).await?
            } else {
                let handler = SftpHandler;
                client::connect(config.clone(), (jump.host.as_str(), jump.port), handler).await?
            };

            // 认证跳板机
            sftp_authenticate(&mut jump_session, &jump.username, &jump.auth).await?;

            // 通过跳板机转发到目标服务器
            let forwarded_channel = jump_session
                .channel_open_direct_tcpip(&ssh_config.host, ssh_config.port as u32, "127.0.0.1", 0)
                .await?;

            let handler = SftpHandler;
            let session =
                client::connect_stream(config, forwarded_channel.into_stream(), handler).await?;

            (session, Some(jump_session))
        } else if let Some(ref proxy) = ssh_config.proxy {
            tracing::info!("SFTP: 通过代理 {}:{} 连接", proxy.host, proxy.port);
            let stream = sftp_connect_via_proxy(proxy, &ssh_config.host, ssh_config.port).await?;
            let handler = SftpHandler;
            let session = client::connect_stream(config, stream, handler).await?;
            (session, None)
        } else {
            let handler = SftpHandler;
            let session =
                client::connect(config, (ssh_config.host.as_str(), ssh_config.port), handler)
                    .await?;
            (session, None)
        };

        // 认证目标服务器
        sftp_authenticate(&mut session, &ssh_config.username, &ssh_config.auth).await?;

        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;

        let sftp = SftpSession::new(channel.into_stream()).await?;

        Ok(Self {
            sftp,
            session,
            _jump_session: jump_session,
            raw_sftp: None,
        })
    }

    async fn list_dir(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let dir_entries = self
            .sftp
            .read_dir(path)
            .await
            .map_err(|e| anyhow!("Failed to read directory {}: {}", path, e))?;

        let mut entries = Vec::new();

        for entry in dir_entries {
            let file_name = entry.file_name();

            if file_name == "." || file_name == ".." {
                continue;
            }

            let metadata = entry.metadata();
            let size = metadata.size.unwrap_or(0);
            let is_dir = metadata.is_dir();
            let permissions = metadata.permissions.unwrap_or(0);

            let modified = metadata
                .mtime
                .and_then(|mtime| UNIX_EPOCH.checked_add(Duration::from_secs(mtime as u64)))
                .unwrap_or_else(SystemTime::now);

            entries.push(FileEntry {
                name: file_name.clone(),
                path: file_name,
                size,
                modified,
                is_dir,
                permissions,
            });
        }

        entries.sort_by(|a, b| {
            if a.is_dir == b.is_dir {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            } else if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        Ok(entries)
    }

    async fn download_with_progress(
        &mut self,
        remote_path: &str,
        local_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()> {
        let metadata = self
            .sftp
            .metadata(remote_path)
            .await
            .map_err(|e| anyhow!("Failed to get remote file metadata: {}", e))?;

        let total_size = metadata.size.unwrap_or(0);

        // 大文件走流水线下载
        if total_size > PIPELINE_THRESHOLD {
            let raw_session = match self.get_or_create_raw_session().await {
                Ok(raw) => raw,
                Err(e) => {
                    tracing::warn!(
                        "Failed to create raw SFTP session, falling back to serial: {}",
                        e
                    );
                    self.raw_sftp = None;
                    return self
                        .serial_download_file(
                            remote_path,
                            local_path,
                            total_size,
                            cancelled,
                            progress,
                        )
                        .await;
                }
            };

            let result = Self::pipelined_download(
                raw_session,
                remote_path,
                local_path,
                total_size,
                &cancelled,
                &progress,
            )
            .await;

            if result.is_err() {
                // raw session 出错时置空，下次重建
                self.raw_sftp = None;
            }

            return result;
        }

        self.serial_download_file(remote_path, local_path, total_size, cancelled, progress)
            .await
    }

    async fn upload_with_progress(
        &mut self,
        local_path: &str,
        remote_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()> {
        let local_file = File::open(local_path)
            .await
            .map_err(|e| anyhow!("Failed to open local file {}: {}", local_path, e))?;

        let metadata = local_file
            .metadata()
            .await
            .map_err(|e| anyhow!("Failed to get local file metadata: {}", e))?;

        let total_size = metadata.len();

        let mut local_file = BufReader::with_capacity(BUFFER_SIZE, local_file);

        let mut remote_file = self
            .sftp
            .open_with_flags(
                remote_path,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|e| anyhow!("Failed to create remote file {}: {}", remote_path, e))?;

        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut transferred = 0u64;
        let mut last_update = Instant::now();
        let mut speed_samples: Vec<f64> = Vec::new();

        loop {
            ensure_not_cancelled(&cancelled)?;
            let bytes_read = local_file
                .read(&mut buffer)
                .await
                .map_err(|e| anyhow!("Failed to read from local file: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            remote_file
                .write_all(&buffer[..bytes_read])
                .await
                .map_err(|e| anyhow!("Failed to write to remote file: {}", e))?;

            transferred += bytes_read as u64;

            let now = Instant::now();
            let elapsed = now.duration_since(last_update).as_secs_f64();

            if elapsed >= 0.1 {
                let speed = bytes_read as f64 / elapsed;
                speed_samples.push(speed);
                if speed_samples.len() > 10 {
                    speed_samples.remove(0);
                }

                let avg_speed = speed_samples.iter().sum::<f64>() / speed_samples.len() as f64;

                progress(TransferProgress {
                    transferred,
                    total: total_size,
                    speed: avg_speed,
                    current_file: None,
                    current_file_transferred: 0,
                    current_file_total: 0,
                });

                last_update = now;
            }
        }

        progress(TransferProgress {
            transferred,
            total: total_size,
            speed: 0.0,
            current_file: None,
            current_file_transferred: 0,
            current_file_total: 0,
        });

        remote_file
            .sync_all()
            .await
            .map_err(|e| anyhow!("Failed to sync remote file: {}", e))?;

        Ok(())
    }

    async fn delete(&mut self, path: &str, is_dir: bool) -> Result<()> {
        if is_dir {
            self.sftp
                .remove_dir(path)
                .await
                .map_err(|e| anyhow!("Failed to remove directory {}: {}", path, e))?;
        } else {
            self.sftp
                .remove_file(path)
                .await
                .map_err(|e| anyhow!("Failed to remove file {}: {}", path, e))?;
        }
        Ok(())
    }

    async fn delete_recursive(
        &mut self,
        path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()> {
        let entries = self.list_dir_recursive(path, cancelled.clone()).await?;

        // 计算总数：文件数 + 目录数 + 根目录本身
        let file_count = entries.iter().filter(|e| !e.is_dir).count();
        let dir_count = entries.iter().filter(|e| e.is_dir).count();
        let total = (file_count + dir_count + 1) as u64;
        let mut deleted: u64 = 0;

        // 先删除所有文件
        for entry in &entries {
            ensure_not_cancelled(&cancelled)?;
            if !entry.is_dir {
                progress(TransferProgress {
                    transferred: deleted,
                    total,
                    speed: 0.0,
                    current_file: Some(entry.name.clone()),
                    current_file_transferred: 0,
                    current_file_total: 1,
                });

                self.sftp
                    .remove_file(&entry.path)
                    .await
                    .map_err(|e| anyhow!("Failed to remove file {}: {}", entry.path, e))?;

                deleted += 1;
                progress(TransferProgress {
                    transferred: deleted,
                    total,
                    speed: 0.0,
                    current_file: Some(entry.name.clone()),
                    current_file_transferred: 1,
                    current_file_total: 1,
                });
            }
        }

        // 按路径深度倒序删除目录（先删子目录）
        let mut dirs: Vec<&FileEntry> = entries.iter().filter(|e| e.is_dir).collect();
        dirs.sort_by(|a, b| b.path.len().cmp(&a.path.len()));
        for dir in dirs {
            ensure_not_cancelled(&cancelled)?;
            progress(TransferProgress {
                transferred: deleted,
                total,
                speed: 0.0,
                current_file: Some(dir.name.clone()),
                current_file_transferred: 0,
                current_file_total: 1,
            });

            self.sftp
                .remove_dir(&dir.path)
                .await
                .map_err(|e| anyhow!("Failed to remove directory {}: {}", dir.path, e))?;

            deleted += 1;
            progress(TransferProgress {
                transferred: deleted,
                total,
                speed: 0.0,
                current_file: Some(dir.name.clone()),
                current_file_transferred: 1,
                current_file_total: 1,
            });
        }

        // 最后删除根目录本身
        let root_name = path.rsplit('/').next().unwrap_or(path).to_string();
        ensure_not_cancelled(&cancelled)?;
        progress(TransferProgress {
            transferred: deleted,
            total,
            speed: 0.0,
            current_file: Some(root_name.clone()),
            current_file_transferred: 0,
            current_file_total: 1,
        });

        self.sftp
            .remove_dir(path)
            .await
            .map_err(|e| anyhow!("Failed to remove directory {}: {}", path, e))?;

        deleted += 1;
        progress(TransferProgress {
            transferred: deleted,
            total,
            speed: 0.0,
            current_file: Some(root_name),
            current_file_transferred: 1,
            current_file_total: 1,
        });

        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        self.sftp
            .create_dir(path)
            .await
            .map_err(|e| anyhow!("Failed to create directory {}: {}", path, e))?;
        Ok(())
    }

    async fn rename(&mut self, old_path: &str, new_path: &str) -> Result<()> {
        self.sftp
            .rename(old_path, new_path)
            .await
            .map_err(|e| anyhow!("Failed to rename {} to {}: {}", old_path, new_path, e))?;
        Ok(())
    }

    async fn chmod(&mut self, _path: &str, _mode: u32) -> Result<()> {
        anyhow::bail!("chmod not yet supported")
    }

    async fn write_file(&mut self, path: &str, content: &[u8]) -> Result<()> {
        let mut remote_file = self
            .sftp
            .open_with_flags(
                path,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|e| anyhow!("Failed to create remote file {}: {}", path, e))?;

        if !content.is_empty() {
            remote_file
                .write_all(content)
                .await
                .map_err(|e| anyhow!("Failed to write to remote file {}: {}", path, e))?;
        }

        // 文件在 drop 时会自动关闭
        drop(remote_file);

        Ok(())
    }

    async fn list_dir_recursive(
        &mut self,
        path: &str,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Vec<FileEntry>> {
        let mut all_entries = Vec::new();
        let mut dirs_to_process = vec![path.to_string()];

        while let Some(current_dir) = dirs_to_process.pop() {
            ensure_not_cancelled(&cancelled)?;
            let entries = self.list_dir(&current_dir).await?;

            for entry in entries {
                ensure_not_cancelled(&cancelled)?;
                let full_path = if current_dir == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", current_dir, entry.name)
                };

                if entry.is_dir {
                    dirs_to_process.push(full_path.clone());
                }

                all_entries.push(FileEntry {
                    name: entry.name,
                    path: full_path,
                    size: entry.size,
                    modified: entry.modified,
                    is_dir: entry.is_dir,
                    permissions: entry.permissions,
                });
            }
        }

        Ok(all_entries)
    }

    async fn download_dir_with_progress(
        &mut self,
        remote_path: &str,
        local_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()> {
        let entries = self
            .list_dir_recursive(remote_path, cancelled.clone())
            .await?;

        let total_size: u64 = entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
        let mut transferred: u64 = 0;

        let base_remote = remote_path.trim_end_matches('/');
        let base_local = std::path::Path::new(local_path);

        std::fs::create_dir_all(base_local)
            .map_err(|e| anyhow!("Failed to create local directory {}: {}", local_path, e))?;

        let mut dirs: Vec<&FileEntry> = entries.iter().filter(|e| e.is_dir).collect();
        dirs.sort_by(|a, b| a.path.len().cmp(&b.path.len()));
        for dir_entry in dirs {
            ensure_not_cancelled(&cancelled)?;
            let relative = dir_entry
                .path
                .strip_prefix(base_remote)
                .unwrap_or(&dir_entry.path);
            let relative = relative.trim_start_matches('/');
            if relative.is_empty() {
                continue;
            }
            let local_dir = base_local.join(relative);
            std::fs::create_dir_all(&local_dir)
                .map_err(|e| anyhow!("Failed to create directory {:?}: {}", local_dir, e))?;
        }

        let files: Vec<&FileEntry> = entries.iter().filter(|e| !e.is_dir).collect();
        let start_time = Instant::now();

        // 检查是否有大文件需要流水线下载
        let has_large_files = files.iter().any(|f| f.size > PIPELINE_THRESHOLD);
        let raw_session = if has_large_files {
            match self.get_or_create_raw_session().await {
                Ok(raw) => Some(raw),
                Err(e) => {
                    tracing::warn!(
                        "Failed to create raw SFTP session, falling back to serial: {}",
                        e
                    );
                    self.raw_sftp = None;
                    None
                }
            }
        } else {
            None
        };

        for file_entry in files {
            ensure_not_cancelled(&cancelled)?;
            let relative = file_entry
                .path
                .strip_prefix(base_remote)
                .unwrap_or(&file_entry.path);
            let relative = relative.trim_start_matches('/');
            let local_file = base_local.join(relative);

            let current_file_name = file_entry.name.clone();
            let current_file_total = file_entry.size;

            if let Some(parent) = local_file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    anyhow!("Failed to create parent directory {:?}: {}", parent, e)
                })?;
            }

            let local_file_str = local_file.to_string_lossy().to_string();

            // 大文件走流水线
            if file_entry.size > PIPELINE_THRESHOLD {
                if let Some(ref raw) = raw_session {
                    let result = Self::pipelined_download_with_file_progress(
                        Arc::clone(raw),
                        &file_entry.path,
                        &local_file_str,
                        file_entry.size,
                        &current_file_name,
                        current_file_total,
                        &mut transferred,
                        total_size,
                        start_time,
                        &cancelled,
                        &progress,
                    )
                    .await;

                    if result.is_err() {
                        self.raw_sftp = None;
                        return result;
                    }
                    continue;
                }
            }

            // 小文件或没有 raw session 时走串行下载
            let mut remote_file = self
                .sftp
                .open_with_flags(&file_entry.path, OpenFlags::READ)
                .await
                .map_err(|e| anyhow!("Failed to open remote file {}: {}", file_entry.path, e))?;

            let local_file_handle = File::create(&local_file)
                .await
                .map_err(|e| anyhow!("Failed to create local file {:?}: {}", local_file, e))?;
            let mut local_file_handle = BufWriter::with_capacity(BUFFER_SIZE, local_file_handle);

            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut current_file_transferred: u64 = 0;

            loop {
                ensure_not_cancelled(&cancelled)?;
                let bytes_read = remote_file
                    .read(&mut buffer)
                    .await
                    .map_err(|e| anyhow!("Failed to read from remote file: {}", e))?;

                if bytes_read == 0 {
                    break;
                }

                local_file_handle
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(|e| anyhow!("Failed to write to local file: {}", e))?;

                transferred += bytes_read as u64;
                current_file_transferred += bytes_read as u64;

                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    transferred as f64 / elapsed
                } else {
                    0.0
                };

                progress(TransferProgress {
                    transferred,
                    total: total_size,
                    speed,
                    current_file: Some(current_file_name.clone()),
                    current_file_transferred,
                    current_file_total,
                });
            }

            local_file_handle
                .flush()
                .await
                .map_err(|e| anyhow!("Failed to flush local file: {}", e))?;
            local_file_handle
                .into_inner()
                .sync_all()
                .await
                .map_err(|e| anyhow!("Failed to sync local file: {}", e))?;
        }

        progress(TransferProgress {
            transferred,
            total: total_size,
            speed: 0.0,
            current_file: None,
            current_file_transferred: 0,
            current_file_total: 0,
        });

        Ok(())
    }

    async fn upload_dir_with_progress(
        &mut self,
        local_path: &str,
        remote_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()> {
        let local_base = std::path::Path::new(local_path);
        if !local_base.is_dir() {
            anyhow::bail!("Local path is not a directory: {}", local_path);
        }

        let mut entries: Vec<(std::path::PathBuf, bool, u64)> = Vec::new();
        let mut dirs_to_scan = vec![local_base.to_path_buf()];

        while let Some(dir) = dirs_to_scan.pop() {
            ensure_not_cancelled(&cancelled)?;
            let read_dir = std::fs::read_dir(&dir)
                .map_err(|e| anyhow!("Failed to read directory {:?}: {}", dir, e))?;

            for entry in read_dir {
                let entry = entry.map_err(|e| anyhow!("Failed to read entry: {}", e))?;
                let path = entry.path();
                let metadata = entry
                    .metadata()
                    .map_err(|e| anyhow!("Failed to get metadata for {:?}: {}", path, e))?;

                if metadata.is_dir() {
                    entries.push((path.clone(), true, 0));
                    dirs_to_scan.push(path);
                } else {
                    entries.push((path, false, metadata.len()));
                }
            }
        }

        let total_size: u64 = entries
            .iter()
            .filter(|(_, is_dir, _)| !is_dir)
            .map(|(_, _, size)| size)
            .sum();
        let mut transferred: u64 = 0;

        let _ = self.sftp.create_dir(remote_path).await;

        let mut dirs: Vec<_> = entries.iter().filter(|(_, is_dir, _)| *is_dir).collect();
        dirs.sort_by(|a, b| a.0.as_os_str().len().cmp(&b.0.as_os_str().len()));

        for (dir_path, _, _) in dirs {
            ensure_not_cancelled(&cancelled)?;
            let relative = dir_path
                .strip_prefix(local_base)
                .map_err(|e| anyhow!("Failed to strip prefix: {}", e))?;
            let relative_str = relative.to_string_lossy();
            if relative_str.is_empty() {
                continue;
            }
            let remote_dir = format!(
                "{}/{}",
                remote_path.trim_end_matches('/'),
                relative_str.replace('\\', "/")
            );
            let _ = self.sftp.create_dir(&remote_dir).await;
        }

        let files: Vec<_> = entries.iter().filter(|(_, is_dir, _)| !*is_dir).collect();
        let start_time = Instant::now();

        for (file_path, _, file_size) in files {
            ensure_not_cancelled(&cancelled)?;
            let relative = file_path
                .strip_prefix(local_base)
                .map_err(|e| anyhow!("Failed to strip prefix: {}", e))?;
            let relative_str = relative.to_string_lossy().replace('\\', "/");
            let remote_file_path =
                format!("{}/{}", remote_path.trim_end_matches('/'), relative_str);

            let current_file_name = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let local_file = File::open(file_path)
                .await
                .map_err(|e| anyhow!("Failed to open local file {:?}: {}", file_path, e))?;
            let mut local_file = BufReader::with_capacity(BUFFER_SIZE, local_file);

            let mut remote_file = self
                .sftp
                .open_with_flags(
                    &remote_file_path,
                    OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
                )
                .await
                .map_err(|e| anyhow!("Failed to create remote file {}: {}", remote_file_path, e))?;

            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut current_file_transferred: u64 = 0;

            loop {
                ensure_not_cancelled(&cancelled)?;
                let bytes_read = local_file
                    .read(&mut buffer)
                    .await
                    .map_err(|e| anyhow!("Failed to read from local file: {}", e))?;

                if bytes_read == 0 {
                    break;
                }

                remote_file
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(|e| anyhow!("Failed to write to remote file: {}", e))?;

                transferred += bytes_read as u64;
                current_file_transferred += bytes_read as u64;

                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    transferred as f64 / elapsed
                } else {
                    0.0
                };

                progress(TransferProgress {
                    transferred,
                    total: total_size,
                    speed,
                    current_file: Some(current_file_name.clone()),
                    current_file_transferred,
                    current_file_total: *file_size,
                });
            }

            remote_file
                .sync_all()
                .await
                .map_err(|e| anyhow!("Failed to sync remote file: {}", e))?;
        }

        progress(TransferProgress {
            transferred,
            total: total_size,
            speed: 0.0,
            current_file: None,
            current_file_transferred: 0,
            current_file_total: 0,
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn realpath(&mut self, path: &str) -> Result<String> {
        let real_path = self
            .sftp
            .canonicalize(path)
            .await
            .map_err(|e| anyhow!("Failed to get realpath for {}: {}", path, e))?;
        Ok(real_path)
    }
}

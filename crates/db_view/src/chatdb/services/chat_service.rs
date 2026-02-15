//! ChatService - AI 对话服务
//!
//! 提供 AI 对话的核心功能：
//! - 流式响应处理（带节流）
//! - 请求取消支持
//! - AI 选表功能

use futures::StreamExt;
use one_core::llm::{ChatRequest, Message, Role};
use one_core::llm::manager::GlobalProviderState;
use one_core::llm::storage::ProviderRepository;
use one_core::storage::StorageManager;
use one_core::storage::traits::Repository;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::chatdb::query_workflow::parse_table_selection_response;

// ============================================================================
// 错误类型
// ============================================================================

/// ChatService 错误类型
#[derive(Debug, Clone)]
pub enum ChatError {
    /// Provider 未找到
    ProviderNotFound,
    /// 请求已取消
    Cancelled,
    /// API 错误
    ApiError(String),
    /// 存储错误
    StorageError(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::ProviderNotFound => write!(f, "AI 模型未找到"),
            ChatError::Cancelled => write!(f, "请求已取消"),
            ChatError::ApiError(msg) => write!(f, "API 错误: {}", msg),
            ChatError::StorageError(msg) => write!(f, "存储错误: {}", msg),
        }
    }
}

impl std::error::Error for ChatError {}

// ============================================================================
// 流式事件
// ============================================================================

/// 流式响应事件
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 内容增量（带完整累积内容）
    ContentDelta {
        /// 增量内容
        delta: String,
        /// 累积的完整内容
        full_content: String,
    },
    /// 完成
    Completed {
        /// 完整内容
        full_content: String,
    },
    /// 错误
    Error {
        /// 错误信息
        message: String,
    },
    /// 已取消
    Cancelled,
}

// ============================================================================
// 流式句柄
// ============================================================================

/// 流式请求句柄
pub struct StreamHandle {
    /// 事件接收器
    pub events: mpsc::Receiver<StreamEvent>,
    /// 取消令牌
    cancel_token: CancellationToken,
}

impl StreamHandle {
    /// 创建新的流式句柄
    pub fn new(events: mpsc::Receiver<StreamEvent>, cancel_token: CancellationToken) -> Self {
        Self {
            events,
            cancel_token,
        }
    }

    /// 取消请求
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 检查是否已取消
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// 获取取消令牌的克隆
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

// ============================================================================
// ChatService
// ============================================================================

/// AI 对话服务
#[derive(Clone)]
pub struct ChatService {
    storage_manager: StorageManager,
    /// UI 更新节流间隔（毫秒）
    throttle_ms: u64,
}

impl ChatService {
    /// 创建新的 ChatService
    pub fn new(storage_manager: StorageManager) -> Self {
        Self {
            storage_manager,
            throttle_ms: 50, // 默认 50ms 节流
        }
    }

    /// 设置节流间隔
    pub fn with_throttle_ms(mut self, ms: u64) -> Self {
        self.throttle_ms = ms;
        self
    }

    /// 发起流式 AI 对话（可取消）
    ///
    /// 返回 StreamHandle，可以通过它接收事件和取消请求
    pub async fn chat_stream(
        &self,
        provider_id: i64,
        model: Option<String>,
        messages: Vec<Message>,
        global_provider_state: Arc<GlobalProviderState>,
    ) -> Result<StreamHandle, ChatError> {
        let (tx, rx) = mpsc::channel(64);
        let cancel_token = CancellationToken::new();

        let cancel_clone = cancel_token.clone();
        let storage = self.storage_manager.clone();
        let throttle_ms = self.throttle_ms;

        // 启动异步任务处理流式响应
        tokio::spawn(async move {
            let result = Self::run_stream(
                provider_id,
                model,
                messages,
                global_provider_state,
                storage,
                tx.clone(),
                cancel_clone,
                throttle_ms,
            )
            .await;

            if let Err(e) = result {
                let _ = tx
                    .send(StreamEvent::Error {
                        message: e.to_string(),
                    })
                    .await;
            }
        });

        Ok(StreamHandle::new(rx, cancel_token))
    }

    /// 创建流式对话（不自动 spawn，适用于 GPUI 环境）
    ///
    /// 返回 (StreamHandle, Future)，调用者需要自行 spawn Future
    pub fn create_chat_stream(
        &self,
        provider_id: i64,
        model: Option<String>,
        messages: Vec<Message>,
        global_provider_state: Arc<GlobalProviderState>,
    ) -> (StreamHandle, impl std::future::Future<Output = ()> + Send + 'static) {
        let (tx, rx) = mpsc::channel(64);
        let cancel_token = CancellationToken::new();

        let cancel_clone = cancel_token.clone();
        let storage = self.storage_manager.clone();
        let throttle_ms = self.throttle_ms;

        let stream_handle = StreamHandle::new(rx, cancel_token);

        let future = async move {
            let result = Self::run_stream(
                provider_id,
                model,
                messages,
                global_provider_state,
                storage,
                tx.clone(),
                cancel_clone,
                throttle_ms,
            )
            .await;

            if let Err(e) = result {
                let _ = tx
                    .send(StreamEvent::Error {
                        message: e.to_string(),
                    })
                    .await;
            }
        };

        (stream_handle, future)
    }

    /// 执行流式请求
    async fn run_stream(
        provider_id: i64,
        model: Option<String>,
        messages: Vec<Message>,
        global_provider_state: Arc<GlobalProviderState>,
        storage: StorageManager,
        tx: mpsc::Sender<StreamEvent>,
        cancel_token: CancellationToken,
        throttle_ms: u64,
    ) -> Result<(), ChatError> {
        // 获取 provider 配置（内置 provider 直接返回，非内置从数据库查询）
        let config = if provider_id == one_core::llm::BUILTIN_ONET_CLI_ID {
            one_core::llm::ProviderConfig::builtin_onet_cli()
        } else {
            let repo = storage
                .get::<ProviderRepository>()
                .ok_or(ChatError::ProviderNotFound)?;
            repo.get(provider_id)
                .map_err(|e| ChatError::StorageError(e.to_string()))?
                .ok_or(ChatError::ProviderNotFound)?
        };

        let request = ChatRequest {
            model: model.unwrap_or_else(|| config.model.clone()),
            messages,
            max_tokens: Some(2000),
            temperature: Some(0.7),
            stream: Some(true),
            ..Default::default()
        };

        let provider = global_provider_state
            .manager()
            .get_provider(&config)
            .await
            .map_err(|e| ChatError::ApiError(e.to_string()))?;

        let mut stream = provider
            .chat_stream(&request)
            .await
            .map_err(|e| ChatError::ApiError(e.to_string()))?;

        let mut full_content = String::new();
        let mut pending_delta = String::new();
        let mut last_emit = Instant::now();
        let throttle_duration = Duration::from_millis(throttle_ms);

        loop {
            tokio::select! {
                // 检查取消
                _ = cancel_token.cancelled() => {
                    let _ = tx.send(StreamEvent::Cancelled).await;
                    return Ok(());
                }
                // 处理流事件
                result = stream.next() => {
                    match result {
                        Some(Ok(response)) => {
                            if let Some(content) = response.get_content() {
                                full_content.push_str(&content);
                                pending_delta.push_str(&content);

                                // 节流：每 throttle_ms 发送一次 UI 更新
                                if last_emit.elapsed() >= throttle_duration {
                                    let delta = std::mem::take(&mut pending_delta);
                                    let _ = tx.send(StreamEvent::ContentDelta {
                                        delta,
                                        full_content: full_content.clone(),
                                    }).await;
                                    last_emit = Instant::now();
                                }
                            }

                            // 检查是否完成
                            let is_done = response.choices.iter().any(|c| {
                                c.finish_reason.as_ref().map(|r| r != "null").unwrap_or(false)
                            });

                            if is_done {
                                // 发送剩余内容
                                if !pending_delta.is_empty() {
                                    let _ = tx.send(StreamEvent::ContentDelta {
                                        delta: pending_delta,
                                        full_content: full_content.clone(),
                                    }).await;
                                }
                                let _ = tx.send(StreamEvent::Completed { full_content }).await;
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            let _ = tx.send(StreamEvent::Error {
                                message: e.to_string(),
                            }).await;
                            break;
                        }
                        None => {
                            // 流结束但没有 finish_reason
                            if !pending_delta.is_empty() {
                                let _ = tx.send(StreamEvent::ContentDelta {
                                    delta: pending_delta,
                                    full_content: full_content.clone(),
                                }).await;
                            }
                            let _ = tx.send(StreamEvent::Completed { full_content }).await;
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// AI 选表（非流式，可取消）
    pub async fn select_tables(
        &self,
        provider_id: i64,
        prompt: String,
        global_provider_state: Arc<GlobalProviderState>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<String>, ChatError> {
        tokio::select! {
            result = self.do_select_tables(provider_id, prompt, global_provider_state) => result,
            _ = cancel_token.cancelled() => Err(ChatError::Cancelled),
        }
    }

    /// 执行 AI 选表
    async fn do_select_tables(
        &self,
        provider_id: i64,
        prompt: String,
        global_provider_state: Arc<GlobalProviderState>,
    ) -> Result<Vec<String>, ChatError> {
        let repo = self
            .storage_manager
            .get::<ProviderRepository>()
            .ok_or(ChatError::ProviderNotFound)?;
        let config = repo
            .get(provider_id)
            .map_err(|e| ChatError::StorageError(e.to_string()))?
            .ok_or(ChatError::ProviderNotFound)?;

        let request = ChatRequest {
            model: config.model.clone(),
            messages: vec![Message::text(Role::User, prompt)],
            max_tokens: Some(500),
            temperature: Some(0.3),
            stream: Some(false),
            ..Default::default()
        };

        let provider = global_provider_state
            .manager()
            .get_provider(&config)
            .await
            .map_err(|e| ChatError::ApiError(e.to_string()))?;

        let response = provider
            .chat(&request)
            .await
            .map_err(|e| ChatError::ApiError(e.to_string()))?;

        Ok(parse_table_selection_response(&response))
    }

    /// 获取存储管理器的引用
    pub fn storage_manager(&self) -> &StorageManager {
        &self.storage_manager
    }
}

//! ChatStreamProcessor - 流式处理器
//!
//! 将 provider 查找、request 构建、stream 启动、chunk 处理全部抽取为共享逻辑。
//! 两个面板都可以使用此处理器来处理流式 AI 响应。

use std::time::{Duration, Instant};

use futures::StreamExt;
use rust_i18n::t;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::llm::manager::GlobalProviderState;
use crate::llm::storage::ProviderRepository;
use crate::llm::{ChatRequest, Message, extract_stream_text};
use crate::storage::StorageManager;
use crate::storage::traits::Repository;

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
// 流式错误
// ============================================================================

/// 流式处理错误
#[derive(Debug, Clone)]
pub enum StreamError {
    /// Provider 未找到
    ProviderNotFound,
    /// API 错误
    ApiError(String),
    /// 存储错误
    StorageError(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::ProviderNotFound => {
                write!(f, "{}", t!("AiChat.stream_provider_not_found"))
            }
            StreamError::ApiError(msg) => {
                write!(f, "{}", t!("AiChat.stream_api_error", error = msg))
            }
            StreamError::StorageError(msg) => {
                write!(f, "{}", t!("AiChat.stream_storage_error", error = msg))
            }
        }
    }
}

impl std::error::Error for StreamError {}

// ============================================================================
// ChatStreamProcessor
// ============================================================================

/// 流式处理器
///
/// 封装了从 provider 获取流式响应并转换为 StreamEvent 的完整逻辑。
pub struct ChatStreamProcessor;

impl ChatStreamProcessor {
    /// 启动流式聊天，返回事件接收器
    ///
    /// 此方法在 tokio 运行时中执行流式请求，并通过 mpsc channel 发送事件。
    /// 调用者可以通过 cancel_token 取消请求。
    pub async fn start(
        provider_id: i64,
        selected_model: Option<String>,
        messages: Vec<Message>,
        max_tokens: u32,
        temperature: f32,
        cancel_token: CancellationToken,
        global_provider_state: GlobalProviderState,
        storage_manager: StorageManager,
    ) -> Result<mpsc::Receiver<StreamEvent>, StreamError> {
        let (tx, rx) = mpsc::channel(64);

        let cancel_clone = cancel_token.clone();

        tokio::spawn(async move {
            let result = Self::run_stream(
                provider_id,
                selected_model,
                messages,
                max_tokens,
                temperature,
                global_provider_state,
                storage_manager,
                tx.clone(),
                cancel_clone,
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

        Ok(rx)
    }

    /// 创建流式对话（不自动 spawn，适用于 GPUI 环境）
    ///
    /// 返回 (Receiver, Future)，调用者需要自行 spawn Future
    pub fn create_stream(
        provider_id: i64,
        selected_model: Option<String>,
        messages: Vec<Message>,
        max_tokens: u32,
        temperature: f32,
        cancel_token: CancellationToken,
        global_provider_state: GlobalProviderState,
        storage_manager: StorageManager,
    ) -> (
        mpsc::Receiver<StreamEvent>,
        CancellationToken,
        impl Future<Output = ()> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel(64);
        let cancel_clone = cancel_token.clone();

        let future = async move {
            let result = Self::run_stream(
                provider_id,
                selected_model,
                messages,
                max_tokens,
                temperature,
                global_provider_state,
                storage_manager,
                tx.clone(),
                cancel_clone,
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

        (rx, cancel_token, future)
    }

    /// 执行流式请求（内部实现）
    async fn run_stream(
        provider_id: i64,
        selected_model: Option<String>,
        messages: Vec<Message>,
        max_tokens: u32,
        temperature: f32,
        global_provider_state: GlobalProviderState,
        storage: StorageManager,
        tx: mpsc::Sender<StreamEvent>,
        cancel_token: CancellationToken,
    ) -> Result<(), StreamError> {
        // 获取 provider 配置
        let config = {
            let repo = storage
                .get::<ProviderRepository>()
                .ok_or(StreamError::ProviderNotFound)?;
            repo.get(provider_id)
                .map_err(|e| StreamError::StorageError(e.to_string()))?
                .ok_or(StreamError::ProviderNotFound)?
        };

        let request = ChatRequest {
            model: selected_model.unwrap_or_else(|| config.model.clone()),
            messages,
            max_tokens: Some(max_tokens),
            temperature: Some(temperature),
            stream: Some(true),
            ..Default::default()
        };

        let provider = global_provider_state
            .manager()
            .get_provider(&config)
            .await
            .map_err(|e| StreamError::ApiError(e.to_string()))?;

        let mut stream = provider
            .chat_stream(&request)
            .await
            .map_err(|e| StreamError::ApiError(e.to_string()))?;

        let mut full_content = String::new();
        let mut pending_delta = String::new();
        let mut last_emit = Instant::now();
        let throttle_duration = Duration::from_millis(50);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    let _ = tx.send(StreamEvent::Cancelled).await;
                    return Ok(());
                }
                result = stream.next() => {
                    match result {
                        Some(Ok(response)) => {
                            if let Some(content) = extract_stream_text(&response) {
                                full_content.push_str(content);
                                pending_delta.push_str(content);

                                if last_emit.elapsed() >= throttle_duration {
                                    let delta = std::mem::take(&mut pending_delta);
                                    let _ = tx.send(StreamEvent::ContentDelta {
                                        delta,
                                        full_content: full_content.clone(),
                                    }).await;
                                    last_emit = Instant::now();
                                }
                            }

                            let is_done = response.choices.iter().any(|c| {
                                c.finish_reason.as_ref().map(|r| r != "null").unwrap_or(false)
                            });

                            if is_done {
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
}

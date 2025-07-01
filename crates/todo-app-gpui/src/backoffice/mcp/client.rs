use crate::backoffice::BoEvent;
use crate::models::{config_path, mcp_config_path};
use crate::xbus;
use gpui::SharedString;
use gpui_component::IconName;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION};
use rig::extractor::ExtractorBuilder;
use rig::providers::cohere::completion::Tool;
use rig::providers::together::TOPPY_M_7B;
use rig::streaming::{
    stream_to_stdout, StreamingChat, StreamingCompletionModel, StreamingCompletionResponse,
    StreamingPrompt,
};
use rig::tool::{ToolDyn as RigTool, ToolSet};
use rig::{completion::Prompt, providers::openai::Client};
use rmcp::model::{
    CreateMessageRequestMethod, CreateMessageRequestParam, CreateMessageResult, ListRootsResult,
    LoggingLevel, ProtocolVersion, ReadResourceRequestParam, ResourceUpdatedNotificationParam,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult, Content},
    service::{RunningService, ServerSink},
    transport::{auth::AuthClient, auth::OAuthState, SseClientTransport},
    ClientHandler, Peer, RoleClient,
};
pub use rmcp::{
    model::{
        ClientCapabilities, ClientInfo, Implementation, Prompt as McpPrompt,
        Resource as McpResource, ResourceTemplate as McpResourceTemplate, Root, Tool as McpTool,
    },
    Error as McpError, ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, env::home_dir};
use tokio::process::Command;

use anyhow::Result;
use futures::{stream, StreamExt};
use rig::agent::Agent;
use rig::completion::Message;
use rig::completion::ToolDefinition;
use rig::completion::{CompletionError, CompletionModel};
use rig::message::{AssistantContent, UserContent};
use rig::tool::ToolSetError;
use rig::OneOrMany;
use std::boxed::Box;
use std::future::Future;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpClientHandler {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
    // pub peer: Option<Peer<RoleClient>>,
    pub id: String,
}

impl McpClientHandler {
    pub fn new(id: String) -> Self {
        Self {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "xTo-Do/mcp-client".into(),
                version: "0.1.0".into(),
            },
            // peer: None,
            id,
        }
    }
}

impl ClientHandler for McpClientHandler {
    //sampling
    async fn create_message(
        &self,
        params: CreateMessageRequestParam,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, McpError> {
        log::info!("Create message: {params:#?}");

        Err(McpError::method_not_found::<CreateMessageRequestMethod>())
    }
    async fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> Result<ListRootsResult, McpError> {
        Ok(ListRootsResult {
            roots: vec![
                Root {
                    uri: "capture://audio".into(),
                    name: Some("音频设备".into()),
                },
                Root {
                    uri: "capture://screen".into(),
                    name: Some("捕获屏幕".into()),
                },
            ],
        })
    }

    async fn on_tool_list_changed(&self, context: NotificationContext<RoleClient>) {
        log::info!("Tool list changed");
        match context.peer.list_tools(None).await {
            Ok(tools) => {
                log::info!("Tool list: {tools:#?}");

                // 更新 Registry 中的实例状态
                let registry = crate::backoffice::mcp::McpRegistry::global();
                registry.do_send(crate::backoffice::mcp::UpdateInstanceTools {
                    server_id: self.id.clone(),
                    tools: tools.tools.clone(),
                });

                // 发送事件通知
                xbus::post(BoEvent::McpToolListUpdated(self.id.clone(), tools.tools));
            }
            Err(err) => {
                log::error!("Failed to list tools: {err}");
                xbus::post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to list tools: {err}"),
                ));
            }
        }
    }

    async fn on_prompt_list_changed(&self, ctx: NotificationContext<RoleClient>) {
        log::info!("Prompt list changed");

        match ctx.peer.list_prompts(None).await {
            Ok(prompts) => {
                log::info!("Prompt list: {prompts:#?}");

                // 更新 Registry 中的实例状态
                let registry = crate::backoffice::mcp::McpRegistry::global();
                registry.do_send(crate::backoffice::mcp::UpdateInstancePrompts {
                    server_id: self.id.clone(),
                    prompts: prompts.prompts.clone(),
                });

                xbus::post(BoEvent::McpPromptListUpdated(
                    self.id.clone(),
                    prompts.prompts,
                ));
            }
            Err(err) => {
                log::error!("Failed to list prompts: {err}");
                xbus::post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to list prompts: {err}"),
                ));
            }
        }
    }

    async fn on_resource_list_changed(&self, ctx: NotificationContext<RoleClient>) {
        ctx.peer.list_all_resources().await.map_or_else(
            |err| {
                log::error!("Failed to list resources: {err}");
                xbus::post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to list resources: {err}"),
                ));
            },
            |resources| {
                log::info!("Resource list changed: {resources:#?}");

                // 更新 Registry 中的实例状态
                let registry = crate::backoffice::mcp::McpRegistry::global();
                registry.do_send(crate::backoffice::mcp::UpdateInstanceResources {
                    server_id: self.id.clone(),
                    resources: resources.clone(),
                });

                xbus::post(BoEvent::McpResourceListUpdated(self.id.clone(), resources));
            },
        );
    }
    async fn on_cancelled(
        &self,
        params: rmcp::model::CancelledNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        log::info!("Cancelled: {params:#?}");
        xbus::post(BoEvent::Notification(
            crate::backoffice::NotificationKind::Info,
            format!("Cancelled: {:?}", params.reason),
        ));
    }

    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        log::info!("Logging message: {params:#?}");
        if params.level == LoggingLevel::Error {
            xbus::post(BoEvent::Notification(
                crate::backoffice::NotificationKind::Error,
                format!("Logging error: {}", params.data.to_string()),
            ));
        } else if params.level == LoggingLevel::Warning {
            xbus::post(BoEvent::Notification(
                crate::backoffice::NotificationKind::Warning,
                format!("Logging warning: {}", params.data.to_string()),
            ));
        } else {
            xbus::post(BoEvent::Notification(
                crate::backoffice::NotificationKind::Info,
                format!("Logging info: {}", params.data.to_string()),
            ));
        }
        xbus::post(BoEvent::Notification(
            crate::backoffice::NotificationKind::Info,
            format!("Logging message: {:?}", params.data.to_string()),
        ));
    }

    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
    }
    async fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
        log::info!("Resource updated: {}", params.uri);
        
        match context
            .peer
            .read_resource(ReadResourceRequestParam { uri: params.uri.clone() })
            .await
        {
            Ok(result) => {
                log::info!("Resource content read successfully for: {}", params.uri);
                
                // 更新 Registry 中的实例状态
                let registry = crate::backoffice::mcp::McpRegistry::global();
                registry.do_send(crate::backoffice::mcp::UpdateInstanceResourceContent {
                    server_id: self.id.clone(),
                    uri: params.uri.clone(),
                    contents: result.contents.clone(),
                });
                
                // 发送事件通知
                xbus::post(BoEvent::McpResourceUpdated {
                    server_id: self.id.clone(),
                    uri: params.uri,
                    contents: result.contents,
                });
            }
            Err(err) => {
                log::error!("Failed to read updated resource {}: {}", params.uri, err);
                xbus::post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to read updated resource {}: {}", params.uri, err),
                ));
            }
        }
    }
}

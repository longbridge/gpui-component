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

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
}

pub struct McpToolAdaptor {
    tool: McpTool,
    server: ServerSink,
}

impl RigTool for McpToolAdaptor {
    fn name(&self) -> String {
        self.tool.name.to_string()
    }

    fn definition(
        &self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn Future<Output = rig::completion::ToolDefinition> + Send + Sync + '_>>
    {
        Box::pin(std::future::ready(rig::completion::ToolDefinition {
            name: self.name(),
            description: self
                .tool
                .description
                .as_deref()
                .unwrap_or_default()
                .to_string(),
            parameters: self.tool.schema_as_json_value(),
        }))
    }

    fn call(
        &self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<String, rig::tool::ToolError>> + Send + Sync + '_>,
    > {
        let server = self.server.clone();
        Box::pin(async move {
            let call_mcp_tool_result = server
                .call_tool(CallToolRequestParam {
                    name: self.tool.name.clone(),
                    arguments: serde_json::from_str(&args)
                        .map_err(rig::tool::ToolError::JsonError)?,
                })
                .await
                .inspect(|result| tracing::info!(?result))
                .inspect_err(|error| tracing::error!(%error))
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;

            Ok(convert_mcp_call_tool_result_to_string(call_mcp_tool_result))
        })
    }
}

pub fn convert_mcp_call_tool_result_to_string(result: CallToolResult) -> String {
    serde_json::to_string(&result).unwrap()
}

pub async fn get_tool_set(server: ServerSink) -> anyhow::Result<ToolSet> {
    let tools = server.list_all_tools().await?;

    let mut tool_builder = ToolSet::builder();
    for tool in tools {
        tracing::info!("get tool: {}", tool.name);
        let adaptor = McpToolAdaptor {
            tool: tool.clone(),
            server: server.clone(),
        };
        tool_builder = tool_builder.static_tool(adaptor);
    }
    let tool_set = tool_builder.build();
    Ok(tool_set)
}

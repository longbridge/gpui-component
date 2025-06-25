use crate::models::{config_path, mcp_config_path};
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
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    service::{RunningService, ServerSink},
    transport::{auth::AuthClient, auth::OAuthState, SseClientTransport},
    RoleClient,
};
pub use rmcp::{
    model::{
        ClientCapabilities, ClientInfo, Implementation, Prompt as McpPrompt,
        Resource as McpResource, ResourceTemplate as McpResourceTemplate, Tool as McpTool,
    },
    ServiceExt,
};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub enum McpTransport {
    #[default]
    Stdio,
    Sse,
    Streamable,
}

impl McpTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpTransport::Stdio => "Stdio",
            McpTransport::Sse => "Sse",
            McpTransport::Streamable => "Streamable",
        }
    }

    pub fn all() -> Vec<SharedString> {
        vec!["Stdio".into(), "Sse".into(), "Streamable".into()]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum McpCapability {
    Resources,
    Tools,
    Prompts,
}

impl McpCapability {
    pub fn icon(&self) -> IconName {
        match self {
            McpCapability::Resources => IconName::Database,
            McpCapability::Tools => IconName::Wrench,
            McpCapability::Prompts => IconName::SquareTerminal,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            McpCapability::Resources => "资源",
            McpCapability::Tools => "工具",
            McpCapability::Prompts => "提示",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpProviderInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub command: String,
    // #[serde(default)]
    // pub args: Vec<String>,
    #[serde(default)]
    pub transport: McpTransport,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub capabilities: Vec<McpCapability>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub resources: Vec<ResourceDefinition>,
    #[serde(default)]
    pub resource_templates: Vec<ResourceTemplateDefinition>,
    #[serde(default)]
    pub tools: Vec<McpTool>,
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
    #[serde(skip)]
    pub client: Option<Arc<RunningService<RoleClient, rmcp::model::InitializeRequestParam>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceDefinition {
    resource: McpResource,
    pub subscribed: bool,
    pub subscribable: bool,
}

impl std::ops::Deref for ResourceDefinition {
    type Target = McpResource;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl std::ops::DerefMut for ResourceDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceTemplateDefinition {
    pub resource_templates: McpResourceTemplate,
    pub subscribed: bool,
    pub subscribable: bool,
}

impl std::ops::Deref for ResourceTemplateDefinition {
    type Target = McpResourceTemplate;

    fn deref(&self) -> &Self::Target {
        &self.resource_templates
    }
}

impl std::ops::DerefMut for ResourceTemplateDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource_templates
    }
}

impl Default for McpProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            command: String::new(),
            transport: McpTransport::Stdio,
            enabled: true,
            capabilities: vec![],
            description: String::new(),
            resources: vec![],
            tools: vec![],
            resource_templates: vec![],
            prompts: vec![],
            env_vars: std::collections::HashMap::new(),
            client: None,
        }
    }
}

impl McpProviderInfo {
    pub async fn get_tool_set(&self) -> anyhow::Result<ToolSet> {
        let mut tool_set = ToolSet::default();
        if let Some(client) = self.client.as_ref() {
            let server = client.peer().clone();
            tool_set = get_tool_set(server).await?;
        }
        Ok(tool_set)
    }
}

impl McpProviderInfo {
    pub async fn start(&mut self) -> anyhow::Result<&mut Self> {
        match self.transport {
            McpTransport::Stdio => self.start_stdio().await,
            McpTransport::Sse => self.start_sse().await,
            McpTransport::Streamable => self.start_streamable().await,
        }
    }

    async fn start_stdio(&mut self) -> anyhow::Result<&mut Self> {
        let mut command = self.command.split(" ");
        println!(
            "Starting MCP provider with command: program({:?}) args({:?})",
            command.clone().nth(0),
            command.clone().skip(1).collect::<Vec<_>>()
        );
        let command = Command::new(command.nth(0).unwrap_or_default()).configure(|cmd| {
            cmd.args(command.skip(1).collect::<Vec<_>>())
                .envs(&self.env_vars);
            // .creation_flags(0x08000000);
        });

        let transport = TokioChildProcess::new(command)?;
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "xTo-Do/mcp-client".to_string(),
                version: "0.0.1".to_string(),
            },
        };
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_sse(&mut self) -> anyhow::Result<&mut Self> {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        for (key, val) in self.env_vars.iter() {
            let key = HeaderName::from_bytes(key.as_bytes())?;
            let val = val.parse()?;
            headers.insert(key, val);
        }

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .read_timeout(Duration::from_secs(900))
            .default_headers(headers)
            .build()?;
        let transport = SseClientTransport::start_with_client(
            client,
            SseClientConfig {
                sse_endpoint: self.command.clone().into(),
                ..Default::default()
            },
        )
        .await?;
        // let transport = SseClientTransport::start(self.command.clone()).await?;
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "xTo-Do/mcp-client".to_string(),
                version: "0.0.1".to_string(),
            },
        };
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_streamable(&mut self) -> anyhow::Result<&mut Self> {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        for (key, val) in self.env_vars.iter() {
            let key = HeaderName::from_bytes(key.as_bytes())?;
            let val = val.parse()?;
            headers.insert(key, val);
        }

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .read_timeout(Duration::from_secs(900))
            .default_headers(headers)
            .build()?;
        let transport = StreamableHttpClientTransport::with_client(
            client,
            StreamableHttpClientTransportConfig {
                uri: self.command.clone().into(),
                ..Default::default()
            },
        );
        //let transport = StreamableHttpClientTransport::from_uri(self.command.clone());
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "test sse client".to_string(),
                version: "0.0.1".to_string(),
            },
        };
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_serve(
        &mut self,
        client: RunningService<RoleClient, rmcp::model::InitializeRequestParam>,
    ) -> anyhow::Result<&mut Self> {
        let server_info = client.peer_info().cloned().unwrap_or_default();
        let tools = client.list_all_tools().await?;
        let prompts = client.list_all_prompts().await?;
        let resources = client.list_all_resources().await?;
        let resource_templates = client.list_all_resource_templates().await?;
        self.client = Some(Arc::new(client));
        self.tools = tools;
        self.prompts = prompts;
        self.resources = resources
            .into_iter()
            .map(|r| ResourceDefinition {
                resource: r,
                subscribed: false,
                subscribable: server_info
                    .capabilities
                    .resources
                    .clone()
                    .unwrap_or_default()
                    .subscribe
                    .unwrap_or_default(),
            })
            .collect();
        self.resource_templates = resource_templates
            .into_iter()
            .map(|r| ResourceTemplateDefinition {
                resource_templates: r,
                subscribed: false,
                subscribable: server_info
                    .capabilities
                    .resources
                    .clone()
                    .unwrap_or_default()
                    .subscribe
                    .unwrap_or_default(),
            })
            .collect();
        if !self.tools.is_empty() {
            self.capabilities.push(McpCapability::Tools);
        }
        if !self.prompts.is_empty() {
            self.capabilities.push(McpCapability::Prompts);
        }
        if !self.resources.is_empty() || !self.resource_templates.is_empty() {
            self.capabilities.push(McpCapability::Resources);
        }
        Ok(self)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct McpProviderManager {
    #[serde(default)]
    pub providers: Vec<McpProviderInfo>,
}

impl McpProviderManager {
    /// 从文件加载配置
    pub fn load() -> Self {
        let config_path = mcp_config_path();
        if !config_path.exists() {
            return Self::default();
        }
        let content = std::fs::read_to_string(config_path).unwrap_or_default();
        let providers = serde_yaml::from_str::<Vec<McpProviderInfo>>(&content).unwrap_or_default();
        // for provider in providers.iter_mut() {
        //     *provider = provider.clone().start().await?;
        // }
        Self { providers }
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = mcp_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(&self.providers)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有提供商列表
    pub fn list_providers(&self) -> Vec<McpProviderInfo> {
        self.providers.clone()
    }

    /// 根据ID查询提供商
    pub fn get_provider(&self, id: &str) -> Option<&McpProviderInfo> {
        self.providers.iter().find(|p| p.id == id)
    }
}

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

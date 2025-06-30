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
use rmcp::model::{CreateMessageRequestMethod, CreateMessageRequestParam, CreateMessageResult, ListRootsResult, LoggingLevel, ProtocolVersion, ReadResourceRequestParam, ResourceUpdatedNotificationParam};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult,Content},
    service::{RunningService, ServerSink},
    transport::{auth::AuthClient, auth::OAuthState, SseClientTransport},
    RoleClient,Peer, ClientHandler
};
pub use rmcp::{Error as McpError,
    model::{Root,
        ClientCapabilities, ClientInfo, Implementation, Prompt as McpPrompt,
        Resource as McpResource, ResourceTemplate as McpResourceTemplate, Tool as McpTool,
    },
    ServiceExt,
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

#[derive(Debug, Clone,  Deserialize, Serialize)]
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
    pub client: Option<Arc<RunningService<RoleClient,McpClientHandler>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceDefinition {
    pub resource: McpResource,
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
    pub async fn start(mut self) -> anyhow::Result<Self> {
        match self.transport {
            McpTransport::Stdio => self.start_stdio().await,
            McpTransport::Sse => self.start_sse().await,
            McpTransport::Streamable => self.start_streamable().await,
        }
    }

    async fn start_stdio(mut self) -> anyhow::Result<Self> {
        let mut command = self.command.split(" ");
        let command = Command::new(command.nth(0).unwrap_or_default()).configure(|cmd| {
            let args = command.skip(0).collect::<Vec<_>>();
            cmd.args(args).envs(&self.env_vars);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);
        });
        println!("Starting MCP provider with command: {:?}", command);
        let transport = TokioChildProcess::new(command)?;
        let client_info = McpClientHandler::new( self.id.clone());
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_sse(mut self) -> anyhow::Result<Self> {
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
        let client_info = McpClientHandler::new( self.id.clone());
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_streamable(mut self) -> anyhow::Result<Self> {
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
        let client_info =McpClientHandler::new( self.id.clone());
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_serve(
        mut self,
        client: RunningService<RoleClient, McpClientHandler>,
    ) -> anyhow::Result<Self> {
        let server_info = client.peer_info().cloned().unwrap_or_default();
        println!("Server info: {:#?}", server_info);
        if let Some(capability) = server_info.capabilities.tools {
            let tools = client.list_all_tools().await?;
            self.tools = tools;
            if let Some(list_changed) = capability.list_changed {
                if list_changed {
                    println!("Server supports tool list changes.");
                } else {
                    println!("Server does not support tool list changes.");
                }
            }
        }

        if let Some(capability) = server_info.capabilities.prompts {
            let prompts = client.list_all_prompts().await?;
            self.prompts = prompts;
            if let Some(list_changed) = capability.list_changed {
                if list_changed {
                    println!("Server supports prompt list changes.");
                } else {
                    println!("Server does not support prompt list changes.");
                }
            }
        }
        if let Some(capability) = server_info.capabilities.resources {
            let resources = client.list_all_resources().await?;
            let resource_templates = client.list_all_resource_templates().await?;
            let subscribable = capability.subscribe.unwrap_or_default();
            self.resources = resources
                .iter()
                .map(|r| ResourceDefinition {
                    resource: r.clone(),
                    subscribed: false,
                    subscribable,
                })
                .collect();
            self.resource_templates = resource_templates
                .into_iter()
                .map(|r| ResourceTemplateDefinition {
                    resource_templates: r,
                    subscribed: false,
                    subscribable,
                })
                .collect();
            if let Some(list_changed) = capability.list_changed {
                if list_changed {
                    println!("Server supports resource list changes.");
                } else {
                    println!("Server does not support resource list changes.");
                }
            }
            if let Some(subscribe) = capability.subscribe {
                if subscribe {
                    println!("Server supports resource subscription.");
                } else {
                    println!("Server does not support resource subscription.");
                }
            }
        }
        self.client = Some(Arc::new(client));
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
pub struct McpProviderConfig;

impl McpProviderConfig {
    pub fn config_path() -> PathBuf {
        mcp_config_path()
    }
    /// 从文件加载所有提供商配置
    pub fn load_providers() -> anyhow::Result<Vec<McpProviderInfo>> {
        let config_path = mcp_config_path();
        if !config_path.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(config_path)?;
        let providers = serde_yaml::from_str::<Vec<McpProviderInfo>>(&content)?;
        Ok(providers)
    }

    /// 保存所有提供商配置到文件
    pub fn save_providers(providers: &[McpProviderInfo]) -> anyhow::Result<()> {
        let config_path = mcp_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(providers)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 根据ID查询单个提供商配置
    pub fn get_provider(id: &str) -> anyhow::Result<Option<McpProviderInfo>> {
        let providers = Self::load_providers()?;
        Ok(providers.into_iter().find(|p| p.id == id))
    }

    /// 添加新的提供商配置
    pub fn add_provider(provider: McpProviderInfo) -> anyhow::Result<()> {
        let mut providers = Self::load_providers()?;
        providers.push(provider);
        Self::save_providers(&providers)
    }

    /// 更新提供商配置
    pub fn update_provider(id: &str, updated_provider: McpProviderInfo) -> anyhow::Result<bool> {
        let mut providers = Self::load_providers()?;

        if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
            *provider = updated_provider;
            Self::save_providers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 删除提供商配置
    pub fn remove_provider(id: &str) -> anyhow::Result<bool> {
        let mut providers = Self::load_providers()?;
        let original_len = providers.len();

        providers.retain(|p| p.id != id);

        if providers.len() != original_len {
            Self::save_providers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 启用/禁用提供商
    pub fn set_provider_enabled(id: &str, enabled: bool) -> anyhow::Result<bool> {
        let mut providers = Self::load_providers()?;

        if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
            provider.enabled = enabled;
            Self::save_providers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取所有启用的提供商
    pub fn get_enabled_providers() -> anyhow::Result<Vec<McpProviderInfo>> {
        let providers = Self::load_providers()?;
        Ok(providers.into_iter().filter(|p| p.enabled).collect())
    }

    /// 启动指定提供商（返回启动后的实例）
    pub async fn start_provider(id: &str) -> anyhow::Result<Option<McpProviderInfo>> {
        if let Some( provider) = Self::get_provider(id)? {
            Ok(Some(provider.start().await?))
        } else {
            Ok(None)
        }
    }

    /// 启动所有启用的提供商
    pub async fn start_enabled_providers() -> anyhow::Result<Vec<McpProviderInfo>> {
        let providers = Self::get_enabled_providers()?;
        let mut started_providers = Vec::new();

        for  provider in providers {
            if let Ok(provider) = provider.start().await {
                started_providers.push(provider);
            }
        }

        Ok(started_providers)
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
            protocol_version:Default::default(),
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

    async fn on_prompt_list_changed(&self, ctx: NotificationContext<RoleClient>) {
        log::info!("Prompt list changed");
        
        match ctx.peer.list_prompts(None).await {
            Ok(prompts) => {
                log::info!("Prompt list: {prompts:#?}");
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
                xbus::post(BoEvent::McpResourceListUpdated(self.id.clone(), resources));
            },
        );
    }
    async fn on_tool_list_changed(&self, context: NotificationContext<RoleClient>) {
        log::info!("Tool list changed");
        match context.peer.list_tools(None).await {
            Ok(tools) => {
                log::info!("Tool list: {tools:#?}");
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
        
        match context
            .peer
            .read_resource(ReadResourceRequestParam { uri: params.uri })
            .await
        {
            Ok(result) => {
                log::info!("Resource updated: {result:#?}");
                xbus::post(BoEvent::McpResourceResult(self.id.clone(), result));
            }
            Err(err) => {
                log::error!("Failed to read resource: {err}");
                xbus::post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to read resource: {err}"),
                ));
            }
        }
    }
}

use crate::backoffice::mcp::client::McpClientHandler;
use crate::config::mcp_config::{McpServerConfig, McpTransport};
use gpui_component::IconName;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION};
use rig::tool::ToolSet;
use rmcp::model::{ClientRequest, PingRequestMethod, ReadResourceRequestParam, ResourceContents};
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    service::RunningService,
    transport::SseClientTransport,
    RoleClient,
};
pub use rmcp::{
    model::{
        Prompt as McpPrompt, Resource as McpResource, ResourceTemplate as McpResourceTemplate,
        Tool as McpTool,
    },
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::boxed::Box;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

// 如果需要在实例中缓存资源内容，可以添加这个字段
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceDefinition {
    pub resource: McpResource,
    pub subscribed: bool,
    pub subscribable: bool,
    pub cached_contents: Option<Vec<ResourceContents>>, // 新增：缓存的资源内容
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>, // 新增：最后更新时间
}

impl ResourceDefinition {
    pub fn new(resource: McpResource, subscribable: bool) -> Self {
        Self {
            resource,
            subscribed: false,
            subscribable,
            cached_contents: None,
            last_updated: None,
        }
    }

    pub fn update_contents(&mut self, contents: Vec<ResourceContents>) {
        self.cached_contents = Some(contents);
        self.last_updated = Some(chrono::Utc::now());
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceTemplateDefinition {
    pub resource_template: McpResourceTemplate,
    pub subscribed: bool,
    pub subscribable: bool,
}

impl std::ops::Deref for ResourceTemplateDefinition {
    type Target = McpResourceTemplate;

    fn deref(&self) -> &Self::Target {
        &self.resource_template
    }
}

impl std::ops::DerefMut for ResourceTemplateDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource_template
    }
}

// 运行时状态和实例
#[derive(Debug, Clone)]
pub struct McpServerInstance {
    pub config: McpServerConfig,
    pub capabilities: Vec<McpCapability>,
    pub resources: Vec<ResourceDefinition>,
    pub resource_templates: Vec<ResourceTemplateDefinition>,
    pub tools: Vec<McpTool>,
    pub prompts: Vec<McpPrompt>,
    pub client: Option<Arc<RunningService<RoleClient, McpClientHandler>>>,
    pub keepalive: bool, // 是否保持连接
}

impl McpServerInstance {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            capabilities: vec![],
            resources: vec![],
            resource_templates: vec![],
            tools: vec![],
            prompts: vec![],
            client: None,
            keepalive: false, // 默认不保持连接
        }
    }

    pub async fn start(self) -> anyhow::Result<Self> {
        match self.config.transport {
            McpTransport::Stdio => self.start_stdio().await,
            McpTransport::Sse => self.start_sse().await,
            McpTransport::Streamable => self.start_streamable().await,
        }
    }

    async fn start_stdio(self) -> anyhow::Result<Self> {
        let mut command = self.config.command.split(" ");
        let command = Command::new(command.nth(0).unwrap_or_default()).configure(|cmd| {
            let args = command.skip(0).collect::<Vec<_>>();
            cmd.args(args).envs(&self.config.env_vars);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);
        });

        println!("Starting MCP provider with command: {:?}", command);
        let transport = TokioChildProcess::new(command)?;
        let client_info = McpClientHandler::new(self.config.id.clone());
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_sse(self) -> anyhow::Result<Self> {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        for (key, val) in self.config.env_vars.iter() {
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
                sse_endpoint: self.config.command.clone().into(),
                ..Default::default()
            },
        )
        .await?;
        let client_info = McpClientHandler::new(self.config.id.clone());
        let client = client_info.serve(transport).await?;
        self.start_serve(client).await
    }

    async fn start_streamable(self) -> anyhow::Result<Self> {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        for (key, val) in self.config.env_vars.iter() {
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
                uri: self.config.command.clone().into(),
                ..Default::default()
            },
        );

        //let transport = StreamableHttpClientTransport::from_uri(self.command.clone());
        let client_info = McpClientHandler::new(self.config.id.clone());
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
                    self.keepalive = true;
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
                    self.keepalive = true;
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
                    cached_contents: None,
                    last_updated: None,
                })
                .collect();
            self.resource_templates = resource_templates
                .into_iter()
                .map(|r| ResourceTemplateDefinition {
                    resource_template: r,
                    subscribed: false,
                    subscribable,
                })
                .collect();
            if let Some(list_changed) = capability.list_changed {
                if list_changed {
                    self.keepalive = true;
                    println!("Server supports resource list changes.");
                } else {
                    println!("Server does not support resource list changes.");
                }
            }
            if let Some(subscribe) = capability.subscribe {
                if subscribe {
                    self.keepalive = true;
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

    pub async fn ping(&self) -> anyhow::Result<()> {
        if let Some(client) = &self.client {
            let req = ClientRequest::PingRequest(rmcp::model::PingRequest {
                method: PingRequestMethod,
                extensions: Default::default(),
            });
            client.send_request(req).await?;
        }
        Ok(())
    }

    /// 停止服务器实例
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(client) = self.client.take() {
            // 优雅关闭客户端连接
            drop(client);
            log::info!("McpServerInstance {} stopped", self.config.id);
        }
        Ok(())
    }

    /// 检查实例是否正在运行
    pub fn is_running(&self) -> bool {
        self.client.is_some()
    }

    /// 重启实例
    pub async fn restart(mut self) -> anyhow::Result<Self> {
        self.stop().await?;
        tokio::time::sleep(Duration::from_millis(100)).await; // 短暂等待
        self.start().await
    }

    /// 获取实例状态
    pub fn status(&self) -> McpServerStatus {
        match &self.client {
            Some(_) => McpServerStatus::Running,
            None => McpServerStatus::Stopped,
        }
    }

    /// 获取工具集（如果正在运行）
    pub async fn get_tool_set(&self) -> anyhow::Result<ToolSet> {
        let mut tool_set = ToolSet::default();
        if let Some(client) = self.client.as_ref() {
            let server = client.peer().clone();
            tool_set = super::adaptor::get_tool_set(server).await?;
        }
        Ok(tool_set)
    }

    /// 调用工具
    pub async fn call_tool(&self, name: &str, args: &str) -> anyhow::Result<CallToolResult> {
        if let Some(client) = &self.client {
            let server = client.peer().clone();
            println!(
                "Calling tool '{}' on server instance '{}'",
                name, self.config.id
            );
            let call_mcp_tool_result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                server.call_tool(CallToolRequestParam {
                    name: name.to_string().into(),
                    arguments: serde_json::from_str(args)
                        .map_err(rig::tool::ToolError::JsonError)?,
                }),
            )
            .await?
            .inspect(|result| println!("{:?}", result))
            .inspect_err(|error| println!("{:?}", error))
            .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            Ok(call_mcp_tool_result)
        } else {
            Err(anyhow::anyhow!("Server instance not connected"))
        }
    }

    pub async fn list_tools(&self) -> anyhow::Result<Vec<McpTool>> {
        if let Some(client) = &self.client {
            let server = client.peer().clone();

            match server.list_tools(None).await {
                Ok(result) => Ok(result.tools),
                Err(err) => Err(anyhow::anyhow!("Failed to list tools: {}", err)),
            }
        } else {
            Err(anyhow::anyhow!("Server instance not connected"))
        }
    }

    /// 读取资源
    pub async fn read_resource(&self, uri: &str) -> anyhow::Result<Vec<ResourceContents>> {
        if let Some(client) = &self.client {
            let server = client.peer().clone();

            let request = ReadResourceRequestParam {
                uri: uri.to_string(),
            };

            match server.read_resource(request).await {
                Ok(result) => Ok(result.contents),
                Err(err) => Err(anyhow::anyhow!("Failed to read resource: {}", err)),
            }
        } else {
            Err(anyhow::anyhow!("Server instance not connected"))
        }
    }

    /// 获取可用资源列表
    pub async fn list_resources(&self) -> anyhow::Result<Vec<McpResource>> {
        if let Some(client) = &self.client {
            let server = client.peer().clone();

            match server.list_resources(None).await {
                Ok(result) => Ok(result.resources),
                Err(err) => Err(anyhow::anyhow!("Failed to list resources: {}", err)),
            }
        } else {
            Err(anyhow::anyhow!("Server instance not connected"))
        }
    }

    /// 获取可用提示列表
    pub async fn list_prompts(&self) -> anyhow::Result<Vec<McpPrompt>> {
        if let Some(client) = &self.client {
            let server = client.peer().clone();
            match server.list_prompts(None).await {
                Ok(result) => Ok(result.prompts),
                Err(err) => Err(anyhow::anyhow!("Failed to list prompts: {}", err)),
            }
        } else {
            Err(anyhow::anyhow!("Server instance not connected"))
        }
    }

    /// 获取缓存的资源内容
    pub fn get_cached_resource_content(&self, uri: &str) -> Option<&Vec<ResourceContents>> {
        self.resources
            .iter()
            .find(|r| r.resource.uri == uri)
            .and_then(|r| r.cached_contents.as_ref())
    }

    /// 获取资源的最后更新时间
    pub fn get_resource_last_updated(&self, uri: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        self.resources
            .iter()
            .find(|r| r.resource.uri == uri)
            .and_then(|r| r.last_updated)
    }

    /// 检查资源是否有缓存内容
    pub fn has_cached_resource(&self, uri: &str) -> bool {
        self.resources
            .iter()
            .any(|r| r.resource.uri == uri && r.cached_contents.is_some())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum McpServerStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

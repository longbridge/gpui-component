use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::mcp::client_handler::McpClientHandler;
use crate::backoffice::mcp::loader::McpServerLoader;
use crate::backoffice::mcp::{
    ExitFromRegistry, McpRegistry, ToolCallRequest, ToolCallResult, UpdateServerCache,
};
use crate::backoffice::BoEvent;
use crate::config::mcp_config::{McpServerConfig, McpTransport};
use actix::prelude::*;
use gpui_component::IconName;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION};
use rmcp::model::{ClientRequest, PingRequestMethod, ResourceContents, SubscribeRequestParam};
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::CallToolRequestParam, service::RunningService, transport::SseClientTransport, RoleClient,
};
pub use rmcp::{
    model::{
        Content, Prompt as McpPrompt, Resource as McpResource,
        ResourceTemplate as McpResourceTemplate, Tool as McpTool,
    },
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum McpCapability {
    Resources,
    ResourceTemplate,
    Tools,
    Prompts,
}

impl McpCapability {
    pub fn icon(&self) -> IconName {
        match self {
            McpCapability::Resources => IconName::Database,
            McpCapability::ResourceTemplate => IconName::Database,
            McpCapability::Tools => IconName::Wrench,
            McpCapability::Prompts => IconName::SquareTerminal,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            McpCapability::Resources => "资源",
            McpCapability::ResourceTemplate => "资源模板",
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerSnapshot {
    pub config: McpServerConfig,
    pub status: McpServerStatus,
    pub capabilities: Vec<McpCapability>,
    pub tools: Vec<McpTool>,
    pub prompts: Vec<McpPrompt>,
    pub resources: Vec<ResourceDefinition>,
    pub resource_templates: Vec<ResourceTemplateDefinition>,
    pub keepalive: bool,
}

impl McpServerSnapshot {
    /// 创建一个空的快照（仅包含配置）
    pub fn empty(config: McpServerConfig) -> Self {
        Self {
            config,
            status: McpServerStatus::Stopped,
            capabilities: Vec::new(),
            tools: Vec::new(),
            prompts: Vec::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
            keepalive: false,
        }
    }

    /// 从服务器信息更新快照
    pub fn from_server_info(
        config: McpServerConfig,
        capabilities: Vec<McpCapability>,
        tools: Vec<McpTool>,
        prompts: Vec<McpPrompt>,
        resources: Vec<ResourceDefinition>,
        resource_templates: Vec<ResourceTemplateDefinition>,
        keepalive: bool,
    ) -> Self {
        Self {
            config,
            status: McpServerStatus::Running,
            capabilities,
            tools,
            prompts,
            resources,
            resource_templates,
            keepalive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum McpServerStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

pub struct McpServer {
    config: McpServerConfig,
    client: Option<Arc<RunningService<RoleClient, McpClientHandler>>>,
    status: McpServerStatus,
    keepalive: bool,

    // MCP 能力数据
    capabilities: Vec<McpCapability>,
    tools: Vec<McpTool>,
    prompts: Vec<McpPrompt>,
    resources: Vec<ResourceDefinition>,
    resource_templates: Vec<ResourceTemplateDefinition>,

    // Actor 管理
    tick_handle: Option<SpawnHandle>,
}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            client: None,
            status: McpServerStatus::Stopped,
            keepalive: false,
            capabilities: Vec::new(),
            tools: Vec::new(),
            prompts: Vec::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
            tick_handle: None,
        }
    }
}

impl McpServer {
    /// 连接到 MCP 服务器
    fn connect(&mut self, ctx: &mut Context<Self>) {
        tracing::info!("Connecting to MCP Server: {}", self.config.id);
        self.status = McpServerStatus::Starting;

        let config = self.config.clone();
        let server_addr = ctx.address();

        // 使用 McpServerLoader 进行连接
        McpServerLoader::load_server(config, server_addr)
            .into_actor(self)
            .then(|res, act, _ctx| {
                match res {
                    Ok((client, snapshot)) => {
                        tracing::info!("MCP Server {} connected successfully", act.config.id);

                        // 设置客户端和状态
                        act.client = Some(client);
                        act.status = snapshot.status;
                        act.capabilities = snapshot.capabilities;
                        act.tools = snapshot.tools;
                        act.prompts = snapshot.prompts;
                        act.resources = snapshot.resources;
                        act.resource_templates = snapshot.resource_templates;
                        act.keepalive = snapshot.keepalive;

                        // 发送事件通知
                        CrossRuntimeBridge::global()
                            .emit(BoEvent::McpServerStarted(act.config.clone()));

                        // 通知 Registry 更新缓存
                        let registry = McpRegistry::global();
                        registry.do_send(UpdateServerCache {
                            server_id: act.config.id.clone(),
                            snapshot: Some(act.create_snapshot()),
                        });
                    }
                    Err(err) => {
                        tracing::error!(
                            "Failed to connect to MCP Server {}: {}",
                            act.config.id,
                            err
                        );
                        act.status = McpServerStatus::Error(err.to_string());

                        CrossRuntimeBridge::global().emit(BoEvent::Notification(
                            crate::backoffice::NotificationKind::Error,
                            format!("Failed to connect to MCP Server {}: {}", act.config.id, err),
                        ));
                    }
                }
                fut::ready(())
            })
            .spawn(ctx);
    }
}
impl McpServer {
    /// 创建快照用于跨运行时传递
    pub fn create_snapshot(&self) -> McpServerSnapshot {
        McpServerSnapshot {
            config: self.config.clone(),
            status: self.status.clone(),
            capabilities: self.capabilities.clone(),
            tools: self.tools.clone(),
            prompts: self.prompts.clone(),
            resources: self.resources.clone(),
            resource_templates: self.resource_templates.clone(),
            keepalive: self.keepalive,
        }
    }

    /// 定时检查连接状态
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Some(client) = &self.client {
            if self.keepalive || self.config.transport == McpTransport::Sse {
                let client = client.clone();
                let server_name = self.config.name.clone();

                async move {
                    tokio::time::timeout(
                        Duration::from_secs(3),
                        client.send_request(ClientRequest::PingRequest(rmcp::model::PingRequest {
                            method: PingRequestMethod,
                            extensions: Default::default(),
                        })),
                    )
                    .await
                }
                .into_actor(self)
                .then(move |res, act, ctx| {
                    match res {
                        Ok(Ok(_)) => {
                            tracing::trace!("MCP Server {} is keeping alive", server_name);
                        }
                        _ => {
                            tracing::warn!("MCP Server {} ping failed, reconnecting", server_name);
                            act.connect(ctx);
                        }
                    }
                    fut::ready(())
                })
                .spawn(ctx);
            }
        }
    }
}

impl Actor for McpServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let tick_handle = ctx.run_interval(Duration::from_secs(15), Self::tick);
        self.tick_handle = Some(tick_handle);
        self.connect(ctx);
    }
}

impl Handler<ExitFromRegistry> for McpServer {
    type Result = ();

    fn handle(&mut self, _msg: ExitFromRegistry, ctx: &mut Self::Context) -> Self::Result {
        tracing::info!("MCP Server {} exiting", self.config.id);
        let server_id = self.config.id.clone();
        // 异步停止实例
        McpRegistry::global().do_send(UpdateServerCache {
            server_id,
            snapshot: None,
        });
        ctx.stop();
    }
}

impl Handler<ToolCallRequest> for McpServer {
    type Result = ResponseActFuture<Self, ToolCallResult>;

    fn handle(&mut self, msg: ToolCallRequest, _ctx: &mut Self::Context) -> Self::Result {
        let server_id = self.config.id.clone();
        let tool_name = msg.name.clone();
        let arguments = msg.arguments.clone();
        let client = self.client.clone();

        async move {
            let result = if let Some(client) = &client {
                let server = client.peer().clone();
                match tokio::time::timeout(
                    Duration::from_secs(30),
                    server.call_tool(CallToolRequestParam {
                        name: tool_name.clone().into(),
                        arguments: serde_json::from_str(&arguments).unwrap_or_default(),
                    }),
                )
                .await
                {
                    Ok(Ok(result)) => ToolCallResult {
                        id: server_id,
                        name: tool_name,
                        content: result.content,
                        is_error: false,
                    },
                    Ok(Err(err)) => ToolCallResult {
                        id: server_id,
                        name: tool_name,
                        content: vec![Content::text(format!("Tool execution error: {}", err))],
                        is_error: true,
                    },
                    Err(_) => ToolCallResult {
                        id: server_id,
                        name: tool_name,
                        content: vec![Content::text("Tool execution timeout".to_string())],
                        is_error: true,
                    },
                }
            } else {
                ToolCallResult {
                    id: server_id,
                    name: tool_name,
                    content: vec![Content::text("MCP Server not connected".to_string())],
                    is_error: true,
                }
            };
            result
        }
        .into_actor(self)
        .boxed_local()
    }
}

/// 工具列表更新通知
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstanceTools {
    pub server_id: String,
    pub tools: Vec<McpTool>,
}

impl Handler<UpdateInstanceTools> for McpServer {
    type Result = ();

    fn handle(&mut self, msg: UpdateInstanceTools, _ctx: &mut Self::Context) -> Self::Result {
        // 检查是否是针对当前服务器的更新
        if msg.server_id == self.config.id {
            tracing::debug!("Updating tools for server: {}", self.config.id);

            // 直接更新自己的工具列表
            self.tools = msg.tools.clone();

            // 更新能力列表
            if !self.tools.is_empty() && !self.capabilities.contains(&McpCapability::Tools) {
                self.capabilities.push(McpCapability::Tools);
            }

            // 通知 Registry 更新缓存快照
            let registry = McpRegistry::global();
            registry.do_send(UpdateServerCache {
                server_id: self.config.id.clone(),
                snapshot: Some(self.create_snapshot()),
            });

            // 发送全局事件通知
            CrossRuntimeBridge::global().emit(BoEvent::McpToolListUpdated(
                self.config.id.clone(),
                msg.tools,
            ));
        }
    }
}

/// 提示列表更新通知
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstancePrompts {
    pub server_id: String,
    pub prompts: Vec<McpPrompt>,
}

impl Handler<UpdateInstancePrompts> for McpServer {
    type Result = ();

    fn handle(&mut self, msg: UpdateInstancePrompts, _ctx: &mut Self::Context) -> Self::Result {
        // 检查是否是针对当前服务器的更新
        if msg.server_id == self.config.id {
            tracing::debug!("Updating prompts for server: {}", self.config.id);

            // 直接更新自己的提示列表
            self.prompts = msg.prompts.clone();

            // 更新能力列表
            if !self.prompts.is_empty() && !self.capabilities.contains(&McpCapability::Prompts) {
                self.capabilities.push(McpCapability::Prompts);
            }

            // 通知 Registry 更新缓存快照
            let registry = McpRegistry::global();
            registry.do_send(UpdateServerCache {
                server_id: self.config.id.clone(),
                snapshot: Some(self.create_snapshot()),
            });

            // 发送全局事件通知
            CrossRuntimeBridge::global().emit(BoEvent::McpPromptListUpdated(
                self.config.id.clone(),
                msg.prompts,
            ));
        }
    }
}

/// 资源列表更新通知
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstanceResources {
    pub server_id: String,
    pub resources: Vec<McpResource>,
}

impl Handler<UpdateInstanceResources> for McpServer {
    type Result = ();

    fn handle(&mut self, msg: UpdateInstanceResources, _ctx: &mut Self::Context) -> Self::Result {
        // 检查是否是针对当前服务器的更新
        if msg.server_id == self.config.id {
            tracing::debug!("Updating resources for server: {}", self.config.id);

            // 保留原有的订阅状态和能力设置
            let subscribable = self
                .resources
                .first()
                .map(|r| r.subscribable)
                .unwrap_or(false);

            // 更新资源列表，保留已有的缓存内容和订阅状态
            let mut new_resources = Vec::new();
            for new_resource in msg.resources.iter() {
                if let Some(existing) = self
                    .resources
                    .iter()
                    .find(|r| r.resource.uri == new_resource.uri)
                {
                    // 保留现有资源的状态和缓存
                    let mut updated = existing.clone();
                    updated.resource = new_resource.clone();
                    new_resources.push(updated);
                } else {
                    // 新资源
                    new_resources.push(ResourceDefinition::new(new_resource.clone(), subscribable));
                }
            }

            self.resources = new_resources;

            // 更新能力列表
            if (!self.resources.is_empty() || !self.resource_templates.is_empty())
                && !self.capabilities.contains(&McpCapability::Resources)
            {
                self.capabilities.push(McpCapability::Resources);
            }

            // 通知 Registry 更新缓存快照
            let registry = McpRegistry::global();
            registry.do_send(UpdateServerCache {
                server_id: self.config.id.clone(),
                snapshot: Some(self.create_snapshot()),
            });

            // 发送全局事件通知
            CrossRuntimeBridge::global().emit(BoEvent::McpResourceListUpdated(
                self.config.id.clone(),
                self.resources.clone(),
            ));
        }
    }
}

/// 资源内容更新通知
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstanceResourceContent {
    pub server_id: String,
    pub uri: String,
    pub contents: Vec<ResourceContents>,
}

impl Handler<UpdateInstanceResourceContent> for McpServer {
    type Result = ();

    fn handle(
        &mut self,
        msg: UpdateInstanceResourceContent,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        // 检查是否是针对当前服务器的更新
        if msg.server_id == self.config.id {
            tracing::debug!(
                "Updating resource content for server: {} uri: {}",
                self.config.id,
                msg.uri
            );

            // 更新对应资源的缓存内容
            if let Some(resource) = self
                .resources
                .iter_mut()
                .find(|r| r.resource.uri == msg.uri)
            {
                resource.update_contents(msg.contents.clone());

                // 通知 Registry 更新缓存快照
                let registry = McpRegistry::global();
                registry.do_send(UpdateServerCache {
                    server_id: self.config.id.clone(),
                    snapshot: Some(self.create_snapshot()),
                });
            }

            // 发送全局事件通知
            CrossRuntimeBridge::global().emit(BoEvent::McpResourceUpdated {
                server_id: self.config.id.clone(),
                uri: msg.uri.clone(),
                contents: msg.contents.clone(),
            });

            CrossRuntimeBridge::global().emit(BoEvent::Notification(
                crate::backoffice::NotificationKind::Info,
                serde_json::to_string_pretty(&BoEvent::McpResourceUpdated {
                    server_id: msg.server_id.clone(),
                    uri: msg.uri,
                    contents: msg.contents,
                })
                .map_err(|err| format!("Failed to serialize resource update: {}", err))
                .unwrap_or_default(),
            ));
        }
    }
}

use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::mcp::client_handler::McpClientHandler;
use crate::backoffice::mcp::{ExitFromRegistry, McpCallToolRequest, McpCallToolResult, McpRegistry, UpdateServerCache};
use crate::backoffice::BoEvent;
use crate::config::mcp_config::{McpServerConfig, McpTransport};
use gpui_component::IconName;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION};
use rmcp::model::{
    ClientRequest, PingRequestMethod,  ResourceContents,
    SubscribeRequestParam,
};
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::{CallToolRequestParam},
    service::RunningService,
    transport::SseClientTransport,
    RoleClient,
};
pub use rmcp::{
    model::{
        Prompt as McpPrompt, Resource as McpResource, ResourceTemplate as McpResourceTemplate,
        Tool as McpTool,Content
    },
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use actix::prelude::*;


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
    /// 启动连接，返回 client 和快照
    async fn start_connection(
        config: McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<(Arc<RunningService<RoleClient, McpClientHandler>>, McpServerSnapshot)> {
        let client = match config.transport {
            McpTransport::Stdio => Self::start_stdio_connection(&config, server_addr).await?,
            McpTransport::Sse => Self::start_sse_connection(&config, server_addr).await?,
            McpTransport::Streamable => Self::start_streamable_connection(&config, server_addr).await?,
        };
        
        let (client_arc, snapshot) = Self::initialize_server_capabilities(config, client).await?;
        Ok((client_arc, snapshot))
    }

    /// 启动 STDIO 连接
    async fn start_stdio_connection(
        config: &McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<RunningService<RoleClient, McpClientHandler>> {
        let mut command = config.command.split(" ");
        let command = Command::new(command.nth(0).unwrap_or_default()).configure(|cmd| {
            let args = command.skip(0).collect::<Vec<_>>();
            cmd.kill_on_drop(true)
                .args(args)
                .envs(&config.env_vars);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);
        });
        
        println!("Starting MCP provider with command: {:?}", command);
        let transport = TokioChildProcess::new(command)?;
        
        #[cfg(target_family = "windows")]
        {
            use windows::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectA, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            };
            use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};
            transport
                .id()
                .and_then(|pid: u32| unsafe { OpenProcess(PROCESS_ALL_ACCESS, true, pid).ok() })
                .and_then(|hprocess| unsafe {
                    CreateJobObjectA(None, windows::core::s!("x-todo-mcp-job"))
                        .ok()
                        .and_then(|hjob| {
                            let mut jobobjectinformation =
                                JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                            jobobjectinformation.BasicLimitInformation.LimitFlags =
                                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                            SetInformationJobObject(
                                hjob,
                                JobObjectExtendedLimitInformation,
                                &jobobjectinformation as *const _ as *const std::ffi::c_void,
                                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                            )
                            .ok()
                            .and_then(|_| AssignProcessToJobObject(hjob, hprocess).ok())
                        })
                });
        }
        let client = McpClientHandler::new(config.id.clone(),server_addr).serve(transport).await?;
        Ok(client)
    }

    /// 启动 SSE 连接
    async fn start_sse_connection(
        config: &McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<RunningService<RoleClient, McpClientHandler>> {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        for (key, val) in config.env_vars.iter() {
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
                sse_endpoint: config.command.clone().into(),
                ..Default::default()
            },
        ).await?;
        let client =  McpClientHandler::new(config.id.clone(),server_addr).serve(transport).await?;
        Ok(client)
    }

    /// 启动 Streamable 连接
    async fn start_streamable_connection(
        config: &McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<RunningService<RoleClient, McpClientHandler>> {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        for (key, val) in config.env_vars.iter() {
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
                uri: config.command.clone().into(),
                ..Default::default()
            },
        );
        let client =   McpClientHandler::new(config.id.clone(),server_addr).serve(transport).await?;
        Ok(client)
    }

    /// 初始化服务器能力，返回 client 和快照
    async fn initialize_server_capabilities(
        config: McpServerConfig,
        client: RunningService<RoleClient, McpClientHandler>,
    ) -> anyhow::Result<(Arc<RunningService<RoleClient, McpClientHandler>>, McpServerSnapshot)> {
        let server_info = client.peer_info().cloned().unwrap_or_default();
        println!("Server info: {:#?}", server_info);
        client.notify_initialized().await?;
        
        let mut capabilities = Vec::new();
        let mut tools = Vec::new();
        let mut prompts = Vec::new();
        let mut resources = Vec::new();
        let mut resource_templates = Vec::new();
        let mut keepalive = false;

        // 处理工具能力
        if let Some(capability) = server_info.capabilities.tools {
            tools = client.list_all_tools().await?;
            capabilities.push(McpCapability::Tools);
            if capability.list_changed.unwrap_or(false) {
                keepalive = true;
                println!("Server supports tool list changes.");
            }
        }

        // 处理提示能力
        if let Some(capability) = server_info.capabilities.prompts {
            prompts = client.list_all_prompts().await?;
            capabilities.push(McpCapability::Prompts);
            if capability.list_changed.unwrap_or(false) {
                keepalive = true;
                println!("Server supports prompt list changes.");
            }
        }

        // 处理资源能力
        if let Some(capability) = server_info.capabilities.resources {
            let resource_list = client.list_all_resources().await?;
            let template_list = client.list_all_resource_templates().await?;
            let subscribable = capability.subscribe.unwrap_or_default();
            
            resources = resource_list
                .iter()
                .map(|r| ResourceDefinition::new(r.clone(), subscribable))
                .collect();
                
            resource_templates = template_list
                .into_iter()
                .map(|r| ResourceTemplateDefinition {
                    resource_template: r,
                    subscribed: false,
                    subscribable,
                })
                .collect();
                
            capabilities.push(McpCapability::Resources);
            
            if capability.list_changed.unwrap_or(false) {
                keepalive = true;
                println!("Server supports resource list changes.");
            }

            // 处理资源订阅
            if capability.subscribe.unwrap_or(false) {
                keepalive = true;
                for name in config.subscribed_resources.iter() {
                    if let Some(resource) = resources.iter_mut().find(|r| r.resource.name == *name) {
                        println!("Subscribing to resource: {} ({})", resource.resource.name, resource.resource.uri);
                        match client.subscribe(SubscribeRequestParam {
                            uri: resource.resource.uri.clone(),
                        }).await {
                            Ok(_) => {
                                resource.subscribed = true;
                            }
                            Err(e) => {
                                log::error!("Failed to subscribe to resource {}: {}", resource.resource.name, e);
                            }
                        }
                    }
                }
            }
        }

        // 创建 client 的 Arc 包装
        let client_arc = Arc::new(client);
        
        // 创建快照
        let snapshot = McpServerSnapshot::from_server_info(
            config,
            capabilities,
            tools,
            prompts,
            resources,
            resource_templates,
            keepalive,
        );

        Ok((client_arc, snapshot))
    }

    /// 连接到 MCP 服务器
    fn connect(&mut self, ctx: &mut Context<Self>) {
        log::info!("Connecting to MCP Server: {}", self.config.id);
        self.status = McpServerStatus::Starting;
        
        let config = self.config.clone();
        let server_addr = ctx.address();
        
        Self::start_connection(config, server_addr).into_actor(self)
        .then(|res, act, _ctx| {
            match res {
                Ok((client, snapshot)) => {
                    log::info!("MCP Server {} connected successfully", act.config.id);
                    
                    // 设置 client - 这是关键的修复！
                    act.client = Some(client);
                    
                    // 更新状态
                    act.status = snapshot.status;
                    act.capabilities = snapshot.capabilities;
                    act.tools = snapshot.tools;
                    act.prompts = snapshot.prompts;
                    act.resources = snapshot.resources;
                    act.resource_templates = snapshot.resource_templates;
                    act.keepalive = snapshot.keepalive;
                    
                    // 发送事件通知
                    CrossRuntimeBridge::global().post(BoEvent::McpServerStarted(act.config.clone()));
                    
                    // 通知 Registry 更新缓存
                    let registry = McpRegistry::global();
                    registry.do_send(UpdateServerCache {
                        server_id: act.config.id.clone(),
                        snapshot: Some(act.create_snapshot()),
                    });
                }
                Err(err) => {
                    log::error!("Failed to connect to MCP Server {}: {}", act.config.id, err);
                    act.status = McpServerStatus::Error(err.to_string());
                    
                    CrossRuntimeBridge::global().post(BoEvent::Notification(
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
impl McpServer{
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
                        }))
                    ).await
                }
                .into_actor(self)
                .then(move |res, act, ctx| {
                    match res {
                        Ok(Ok(_)) => {
                            println!("MCP Server {} is keeping alive", server_name);
                        }
                        _ => {
                            log::warn!("MCP Server {} ping failed, reconnecting", server_name);
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
        log::info!("MCP Server {} exiting", self.config.id);
        let server_id = self.config.id.clone();
        // 异步停止实例
         McpRegistry::global().do_send(UpdateServerCache {
                server_id,
                snapshot: None,
            });
            ctx.stop();
    }
}


impl Handler<McpCallToolRequest> for McpServer {
    type Result = ResponseActFuture<Self, McpCallToolResult>;

   fn handle(&mut self, msg: McpCallToolRequest, _ctx: &mut Self::Context) -> Self::Result {
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
                ).await {
                    Ok(Ok(result)) => McpCallToolResult {
                        id: server_id,
                        name: tool_name,
                        content: result.content,
                        is_error: false,
                    },
                    Ok(Err(err)) => McpCallToolResult {
                        id: server_id,
                        name: tool_name,
                        content: vec![Content::text(format!("Tool execution error: {}", err))],
                        is_error: true,
                    },
                    Err(_) => McpCallToolResult {
                        id: server_id,
                        name: tool_name,
                        content: vec![Content::text("Tool execution timeout".to_string())],
                        is_error: true,
                    },
                }
            } else {
                McpCallToolResult {
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
            log::debug!("Updating tools for server: {}", self.config.id);
            
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
            CrossRuntimeBridge::global().post(BoEvent::McpToolListUpdated(
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
            log::debug!("Updating prompts for server: {}", self.config.id);
            
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
            CrossRuntimeBridge::global().post(BoEvent::McpPromptListUpdated(
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
            log::debug!("Updating resources for server: {}", self.config.id);
            
            // 保留原有的订阅状态和能力设置
            let subscribable = self.resources
                .first()
                .map(|r| r.subscribable)
                .unwrap_or(false);

            // 更新资源列表，保留已有的缓存内容和订阅状态
            let mut new_resources = Vec::new();
            for new_resource in msg.resources.iter() {
                if let Some(existing) = self.resources.iter().find(|r| r.resource.uri == new_resource.uri) {
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
                && !self.capabilities.contains(&McpCapability::Resources) {
                self.capabilities.push(McpCapability::Resources);
            }
            
            // 通知 Registry 更新缓存快照
            let registry = McpRegistry::global();
            registry.do_send(UpdateServerCache {
                server_id: self.config.id.clone(),
                snapshot: Some(self.create_snapshot()),
            });
            
            // 发送全局事件通知
            CrossRuntimeBridge::global().post(BoEvent::McpResourceListUpdated(
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

    fn handle(&mut self, msg: UpdateInstanceResourceContent, _ctx: &mut Self::Context) -> Self::Result {
        // 检查是否是针对当前服务器的更新
        if msg.server_id == self.config.id {
            log::debug!("Updating resource content for server: {} uri: {}", self.config.id, msg.uri);
            
            // 更新对应资源的缓存内容
            if let Some(resource) = self.resources.iter_mut().find(|r| r.resource.uri == msg.uri) {
                resource.update_contents(msg.contents.clone());
                
                // 通知 Registry 更新缓存快照
                let registry = McpRegistry::global();
                registry.do_send(UpdateServerCache {
                    server_id: self.config.id.clone(),
                    snapshot: Some(self.create_snapshot()),
                });
            }
            
            // 发送全局事件通知
            CrossRuntimeBridge::global().post(BoEvent::McpResourceUpdated {
                server_id: self.config.id.clone(),
                uri: msg.uri.clone(),
                contents: msg.contents.clone(),
            });

             CrossRuntimeBridge::global().post(BoEvent::Notification(
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

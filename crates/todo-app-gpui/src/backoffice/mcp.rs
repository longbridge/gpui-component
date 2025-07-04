mod adaptor;
mod client_handler;
pub(crate) mod server;
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::mcp::server::McpServerInstance;
use crate::backoffice::{BoEvent, YamlFile};
use crate::config::mcp_config::*;
use crate::xbus::Subscription;
use actix::prelude::*;
use rmcp::model::{Content, ResourceContents};
use rmcp::model::{Prompt as McpPrompt, Resource as McpResource, Tool as McpTool};
use std::{collections::HashMap, time::Duration};

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ExitFromRegistry;

#[derive(Message)]
#[rtype(result = "()")]
pub struct McpServerConfigUpdated;

#[derive(Message)]
#[rtype(result = "McpCallToolResult")]
pub struct McpCallToolRequest {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct McpCallToolResult {
    pub id: String,
    pub name: String,
    pub content: Vec<Content>,
    pub is_error: bool,
}

pub struct McpServerWorker {
    instance: Option<McpServerInstance>,
    config: McpServerConfig,
    tick_handle: Option<SpawnHandle>,
}

impl McpServerWorker {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            instance: None,
            config,
            tick_handle: None,
        }
    }
}

impl McpServerWorker {
    fn connect(&mut self, ctx: &mut Context<Self>) {
        let config = self.config.clone();
        McpServerInstance::new(config)
            .start()
            .into_actor(self)
            .then(|res, act, _ctx| {
                match res {
                    Ok(instance) => {
                        log::info!("MCP Server {} started successfully", instance.config.id);

                        // 通知 Registry 更新实例缓存
                        let registry = McpRegistry::global();
                        registry.do_send(UpdateInstanceCache {
                            server_id: instance.config.id.clone(),
                            instance: Some(instance.clone()),
                        });

                        act.instance = Some(instance);
                        CrossRuntimeBridge::global()
                            .post(BoEvent::McpServerStarted(act.config.clone()));
                    }
                    Err(err) => {
                        log::error!("Failed to start MCP Server {}: {}", act.config.id, err);
                        CrossRuntimeBridge::global().post(BoEvent::Notification(
                            crate::backoffice::NotificationKind::Error,
                            format!("Failed to start MCP Server {}: {}", act.config.id, err),
                        ));
                    }
                }
                fut::ready(())
            })
            .spawn(ctx);
    }
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Some(instance) = self.instance.clone() {
            // 检查实例是否需要保持连接
            let name = instance.config.name.clone();
            if instance.keepalive || instance.config.transport == McpTransport::Sse {
                async move { tokio::time::timeout(Duration::from_secs(3), instance.ping()).await }
                    .into_actor(self)
                    .then(move |_res, this, ctx| {
                        if _res.is_err() {
                            this.connect(ctx);
                        } else {
                            println!("MCP Server {} is keeping alive", name);
                        }
                        fut::ready(())
                    })
                    .wait(ctx);
            }
        }
    }
}

impl Actor for McpServerWorker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let tick_handle = ctx.run_interval(Duration::from_secs(15), Self::tick);
        self.tick_handle = Some(tick_handle);
        self.connect(ctx);
    }
}

impl Handler<ExitFromRegistry> for McpServerWorker {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ExitFromRegistry, _ctx: &mut Self::Context) -> Self::Result {
        log::info!("MCP Server {} exiting", self.config.id);
        let server_id = self.config.id.clone();
        // 异步停止实例
        async move {
            // 通知 Registry 移除实例缓存
            let registry = McpRegistry::global();
            registry.do_send(UpdateInstanceCache {
                server_id,
                instance: None,
            });
        }
        .into_actor(self)
        .then(|_res, _act, ctx| {
            ctx.stop();
            fut::ready(())
        })
        .boxed_local()
    }
}

impl Handler<McpServerConfigUpdated> for McpServerWorker {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: McpServerConfigUpdated, _ctx: &mut Self::Context) -> Self::Result {
        log::info!("MCP Server {} configuration updated", self.config.id);

        fut::ready(()).into_actor(self).boxed_local()
    }
}

impl Handler<McpCallToolRequest> for McpServerWorker {
    type Result = ResponseActFuture<Self, McpCallToolResult>;

    fn handle(&mut self, msg: McpCallToolRequest, _ctx: &mut Self::Context) -> Self::Result {
        let instance = self.instance.clone();
        let server_id = self.config.id.clone();
        let tool_name = msg.name.clone();
        let arguments = msg.arguments.clone();

        async move {
            if let Some(instance) = instance {
                match instance.call_tool(&tool_name, &arguments).await {
                    Ok(result) => McpCallToolResult {
                        id: server_id,
                        name: tool_name,
                        content: result.content,
                        is_error: false,
                    },
                    Err(err) => McpCallToolResult {
                        id: server_id,
                        name: tool_name,
                        content: vec![Content::text(format!("Tool execution error: {}", err))],
                        is_error: true,
                    },
                }
            } else {
                McpCallToolResult {
                    id: server_id,
                    name: tool_name,
                    content: vec![Content::text(
                        "MCP Server instance not available".to_string(),
                    )],
                    is_error: true,
                }
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

// 移除 McpInstanceManager，将其功能整合到 McpRegistry
pub struct McpRegistry {
    servers: HashMap<String, Addr<McpServerWorker>>,
    instances: HashMap<String, McpServerInstance>, // 添加实例管理
    file: YamlFile,
    handle: Option<SpawnHandle>,
    subscriptions: Vec<Subscription>,
}

impl McpRegistry {
    /// 获取全局注册表实例
    pub fn global() -> Addr<Self> {
        McpRegistry::from_registry()
    }
    pub async fn call_tool(
        server_id: &str,
        tool_name: &str,
        arguments: &str,
    ) -> anyhow::Result<McpCallToolResult> {
        let result = McpRegistry::global()
            .send(McpCallToolRequest {
                id: server_id.to_string(),
                name: tool_name.to_string(),
                arguments: arguments.to_string(),
            })
            .await?;

        Ok(result)
    }

    pub async fn get_instance(server_id: &str) -> anyhow::Result<Option<McpServerInstance>> {
        let result = McpRegistry::global()
            .send(GetServerInstance {
                server_id: server_id.to_string(),
            })
            .await?;

        Ok(result)
    }

    pub async fn get_all_instances() -> anyhow::Result<Vec<McpServerInstance>> {
        let result = McpRegistry::global().send(GetAllInstances).await?;

        Ok(result)
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        let file = YamlFile::new(McpConfigManager::config_path());
        Self {
            servers: HashMap::new(),
            instances: HashMap::new(),
            file,
            handle: None,
            subscriptions: Vec::new(),
        }
    }
}

impl Supervised for McpRegistry {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        log::info!("McpRegistry is restarting");
    }
}
impl SystemService for McpRegistry {}

impl McpRegistry {
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Ok(false) = &self.file.exist() {
            self.servers.clear();
            return;
        }
        if let Err(err) = self.check_and_update(ctx) {
            println!("{} {err}", self.file.path.display());
        }
    }

    fn check_and_update(&mut self, _ctx: &mut Context<Self>) -> anyhow::Result<()> {
        if self.file.modified()? {
            let configs = McpConfigManager::load_servers()?;
            let enabled_ids: Vec<_> = configs
                .iter()
                .filter(|config| config.enabled)
                .map(|config| config.id.as_str())
                .collect();
            // 移除不再启用的服务器
            let servers_to_remove: Vec<String> = self
                .servers
                .keys()
                .filter(|id| !enabled_ids.contains(&id.as_str()))
                .cloned()
                .collect();

            for server_id in servers_to_remove {
                if let Some(addr) = self.servers.remove(&server_id) {
                    addr.do_send(ExitFromRegistry);
                    self.instances.remove(&server_id);
                }
            }
            // 添加新启用的服务器
            for config in configs.iter().filter(|c| c.enabled) {
                if let Some(addr) = self.servers.get(&config.id) {
                    addr.do_send(ExitFromRegistry);
                    self.instances.remove(&config.id);
                }
                let server_actor = McpServerWorker::new(config.clone());
                let addr = server_actor.start();
                self.servers.insert(config.id.clone(), addr);
            }
            self.file.open()?;
        }
        Ok(())
    }
}

impl Actor for McpRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let handle = ctx.run_interval(Duration::from_secs(1), Self::tick);
        self.handle = Some(handle);
        let addr = ctx.address();
        let addr_clone = addr.clone();

        self.subscriptions
            .push(
                CrossRuntimeBridge::global().subscribe(move |msg: &UpdateInstanceResources| {
                    addr_clone.do_send(msg.clone());
                }),
            );
        let addr_clone = addr.clone();
        self.subscriptions
            .push(
                CrossRuntimeBridge::global().subscribe(move |msg: &UpdateInstancePrompts| {
                    addr_clone.do_send(msg.clone());
                }),
            );
        let addr_clone = addr.clone();
        self.subscriptions
            .push(
                CrossRuntimeBridge::global().subscribe(move |msg: &UpdateInstanceTools| {
                    addr_clone.do_send(msg.clone());
                }),
            );
        let addr_clone = addr.clone();
        self.subscriptions
            .push(CrossRuntimeBridge::global().subscribe(
                move |msg: &UpdateInstanceResourceContent| {
                    addr_clone.do_send(msg.clone());
                },
            ));

        println!("McpRegistry started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("McpRegistry stopped");
    }
}

impl Handler<McpCallToolRequest> for McpRegistry {
    type Result = ResponseActFuture<Self, McpCallToolResult>;

    fn handle(&mut self, msg: McpCallToolRequest, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(worker) = self.servers.get(&msg.id) {
            let id = msg.id.clone();
            let name = msg.name.clone();
            worker
                .send(msg)
                .into_actor(self)
                .map(|res, _act, _ctx| match res {
                    Ok(result) => result,
                    Err(err) => McpCallToolResult {
                        id: id,
                        name: name,
                        content: vec![Content::text(format!("Error: {}", err))],
                        is_error: true,
                    },
                })
                .boxed_local()
        } else {
            async move {
                McpCallToolResult {
                    id: msg.id.clone(),
                    name: msg.name.clone(),
                    content: vec![Content::text("Server not found".to_string())],
                    is_error: true,
                }
            }
            .into_actor(self)
            .boxed_local()
        }
    }
}

// 添加一个新的消息类型来更新实例缓存
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateInstanceCache {
    pub server_id: String,
    pub instance: Option<McpServerInstance>,
}

impl Handler<UpdateInstanceCache> for McpRegistry {
    type Result = ();

    fn handle(&mut self, msg: UpdateInstanceCache, _ctx: &mut Self::Context) -> Self::Result {
        println!("Updating instance cache for server_id: {}", msg.server_id);
        if let Some(instance) = msg.instance {
            self.instances.insert(msg.server_id, instance);
        } else {
            self.instances.remove(&msg.server_id);
        }
    }
}

// 定义获取实例的消息
#[derive(Message)]
#[rtype(result = "Option<McpServerInstance>")]
pub struct GetServerInstance {
    pub server_id: String,
}

impl Handler<GetServerInstance> for McpRegistry {
    type Result = Option<McpServerInstance>;

    fn handle(&mut self, msg: GetServerInstance, _ctx: &mut Self::Context) -> Self::Result {
        println!("Getting instance for server_id: {}", msg.server_id);
        let server_id = msg.server_id;
        self.instances.get(&server_id).cloned()
    }
}

// 定义获取所有实例的消息
#[derive(Message)]
#[rtype(result = "Vec<McpServerInstance>")]
pub struct GetAllInstances;

impl Handler<GetAllInstances> for McpRegistry {
    type Result = Vec<McpServerInstance>;

    fn handle(&mut self, _msg: GetAllInstances, _ctx: &mut Self::Context) -> Self::Result {
        let mut instances_vec: Vec<_> = self.instances.values().cloned().collect();
        instances_vec.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        instances_vec
    }
}

// 添加更新工具列表的消息
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstanceTools {
    pub server_id: String,
    pub tools: Vec<McpTool>,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstancePrompts {
    pub server_id: String,
    pub prompts: Vec<McpPrompt>,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstanceResources {
    pub server_id: String,
    pub resources: Vec<McpResource>,
}

// 为 McpRegistry 实现处理器
impl Handler<UpdateInstanceTools> for McpRegistry {
    type Result = ();

    fn handle(&mut self, msg: UpdateInstanceTools, _ctx: &mut Self::Context) -> Self::Result {
        println!("Updating tools for server_id: {}", msg.server_id);
        if let Some(instance) = self.instances.get_mut(&msg.server_id) {
            instance.tools = msg.tools.clone();
            log::info!("Updated tools for server: {}", msg.server_id);
            // 发送事件通知
            CrossRuntimeBridge::global().post(BoEvent::McpToolListUpdated(
                msg.server_id.clone(),
                msg.tools,
            ));
        }
    }
}

impl Handler<UpdateInstancePrompts> for McpRegistry {
    type Result = ();

    fn handle(
        &mut self,
        UpdateInstancePrompts { server_id, prompts }: UpdateInstancePrompts,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        println!("Updating prompts for server_id: {}", server_id);
        if let Some(instance) = self.instances.get_mut(&server_id) {
            instance.prompts = prompts.clone();
            log::info!("Updated prompts for server: {}", server_id);
            CrossRuntimeBridge::global().post(BoEvent::McpPromptListUpdated(server_id, prompts));
        }
    }
}

impl Handler<UpdateInstanceResources> for McpRegistry {
    type Result = ();

    fn handle(
        &mut self,
        UpdateInstanceResources {
            server_id,
            resources,
        }: UpdateInstanceResources,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        println!("Updating resources for server_id: {}", server_id);
        if let Some(instance) = self.instances.get_mut(&server_id) {
            // 更新资源定义
            let subscribable = instance
                .resources
                .first()
                .map(|r| r.subscribable)
                .unwrap_or(false);

            instance.resources = resources
                .iter()
                .map(|r| crate::backoffice::mcp::server::ResourceDefinition {
                    resource: r.clone(),
                    subscribed: false,
                    subscribable,
                    cached_contents: None,
                    last_updated: None,
                })
                .collect();

            log::info!("Updated resources for server: {}", server_id);
            CrossRuntimeBridge::global().post(BoEvent::McpResourceListUpdated(
                server_id,
                instance.resources.clone(),
            ));
        }
    }
}

// 添加资源更新的消息
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UpdateInstanceResourceContent {
    pub server_id: String,
    pub uri: String,
    pub contents: Vec<ResourceContents>,
}

impl Handler<UpdateInstanceResourceContent> for McpRegistry {
    type Result = ();

    fn handle(
        &mut self,
        msg: UpdateInstanceResourceContent,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        println!(
            "Handling resource update for server_id: {}, uri: {} {} {}",
            msg.server_id,
            msg.uri,
            self.instances.len(),
            self.instances.contains_key(&msg.server_id)
        );
        CrossRuntimeBridge::global().post(BoEvent::McpResourceUpdated {
            server_id: msg.server_id.clone(),
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
        // if let Some(instance) = self.instances.get_mut(&msg.server_id) {
        //     // 查找并更新对应的资源
        //     // for resource_def in &mut instance.resources {
        //     //     if resource_def.resource.uri == msg.uri {
        //     //         resource_def.update_contents(msg.contents.clone());
        //     //         println!(
        //     //             "Updated cached content for resource {} in server {}",
        //     //             msg.uri, msg.server_id
        //     //         );
        //     //         // 发送事件通知

        //     //         break;
        //     //     }
        //     // }
        //     xbus::post(BoEvent::McpResourceUpdated {
        //         server_id: msg.server_id.clone(),
        //         uri: msg.uri,
        //         contents: msg.contents,
        //     });
        // } else {
        //     log::warn!(
        //         "Server instance {} not found for resource update",
        //         msg.server_id
        //     );
        // }
    }
}

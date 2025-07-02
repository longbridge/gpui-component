mod adaptor;
mod client;
pub(crate) mod server;
use crate::backoffice::mcp::server::McpServerInstance;
use crate::config::mcp_config::*;
use crate::{
    backoffice::{BoEvent, YamlFile},
    xbus,
};
use actix::prelude::*;
use rmcp::model::{Content, ResourceContents};
use rmcp::model::{Prompt as McpPrompt, Resource as McpResource, Tool as McpTool};
use std::any::Any;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ExitFromRegistry;

///登记
#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterFnForAgent(
    pub String,
    pub dyn Fn(dyn Any) -> anyhow::Result<Box<dyn Any>> + Send + 'static,
);

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
    pub result: Vec<Content>,
    pub is_error: bool,
}

pub struct McpServerActor {
    instance: Option<McpServerInstance>,
    config: McpServerConfig,
}

impl McpServerActor {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            instance: None,
            config,
        }
    }
}

impl Actor for McpServerActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let config = self.config.clone();

        // 异步启动实例
        async move { McpServerInstance::new(config).start().await }
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
                        xbus::post(BoEvent::McpServerStarted(act.config.clone()));
                    }
                    Err(err) => {
                        log::error!("Failed to start MCP Server {}: {}", act.config.id, err);
                        xbus::post(BoEvent::Notification(
                            crate::backoffice::NotificationKind::Error,
                            format!("Failed to start MCP Server {}: {}", act.config.id, err),
                        ));
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Handler<ExitFromRegistry> for McpServerActor {
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

// 移除 McpInstanceManager，将其功能整合到 McpRegistry
pub struct McpRegistry {
    servers: HashMap<String, Addr<McpServerActor>>,
    instances: Arc<RwLock<HashMap<String, McpServerInstance>>>, // 添加实例管理
    file: YamlFile,
    handle: Option<SpawnHandle>,
}

impl McpRegistry {
    /// 获取全局注册表实例
    pub fn global() -> Addr<Self> {
        McpRegistry::from_registry()
    }

    /// 静态方法：调用工具
    pub async fn call_tool(
        server_id: &str,
        tool_name: &str,
        args: &str,
    ) -> anyhow::Result<McpCallToolResult> {
        let registry = Self::global();
        let result = registry
            .send(McpCallToolRequest {
                id: server_id.to_string(),
                name: tool_name.to_string(),
                arguments: args.to_string(),
            })
            .await?;
        Ok(result)
    }

    // /// 静态方法：获取服务器实例
    // pub async fn get_instance(server_id: &str) -> anyhow::Result<Option<McpServerInstance>> {
    //     let registry = Self::global();

    //     let result = registry
    //         .send(GetServerInstance {
    //             server_id: server_id.to_string(),
    //         })
    //         .await?;
    //     println!("获取到实例: {:?}", result);
    //     Ok(result)
    // }

    // /// 静态方法：获取所有实例
    // pub async fn get_all_instances() -> anyhow::Result<HashMap<String, McpServerInstance>> {
    //     let registry = Self::global();

    //     let result = registry.send(GetAllInstances).await?;

    //     Ok(result)
    // }

    fn check_and_update(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
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
                    // 同时从实例管理中移除
                    let instances = self.instances.clone();
                    let id = server_id.clone();
                    ctx.spawn(
                        async move {
                            let mut instances = instances.write().await;
                            instances.remove(&id);
                        }
                        .into_actor(self),
                    );
                }
            }
            // 添加新启用的服务器
            for config in configs.iter().filter(|c| c.enabled) {
                if !self.servers.contains_key(&config.id) {
                    let server_actor = McpServerActor::new(config.clone());
                    let addr = server_actor.start();
                    self.servers.insert(config.id.clone(), addr);
                }
            }
        }
        Ok(())
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        let file = YamlFile::new(McpConfigManager::config_path());
        Self {
            servers: HashMap::new(),
            instances: Arc::new(RwLock::new(HashMap::new())),
            file,
            handle: None,
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
}

impl Actor for McpRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let handle = ctx.run_interval(Duration::from_secs(1), Self::tick);
        self.handle = Some(handle);
        println!("McpRegistry started");
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        log::info!("McpRegistry stopped");
        // if let Some(handle) = self.handle.take() {
        //     ctx.cancel_future(handle);
        // }
        // // 停止所有服务器实例
        // for addr in self.servers.values() {
        //     addr.do_send(ExitFromRegistry);
        // }
        // self.servers.clear();
        // // 清空实例缓存
        // let instances = self.instances.clone();
        // async move {
        //     let mut instances = instances.write().await;
        //     instances.clear();
        // }
        // .into_actor(self)
        // .wait(ctx);
    }
}

impl Handler<McpCallToolRequest> for McpRegistry {
    type Result = ResponseActFuture<Self, McpCallToolResult>;

    fn handle(&mut self, msg: McpCallToolRequest, _ctx: &mut Self::Context) -> Self::Result {
        let instances = self.instances.clone();
        let server_id = msg.id.clone();
        let tool_name = msg.name.clone();
        let arguments = msg.arguments.clone();

        async move {
            // 从实例缓存中获取服务器实例
            let instances = instances.read().await;
            if let Some(instance) = instances.get(&server_id) {
                // 调用工具
                match instance.call_tool(&tool_name, &arguments).await {
                    Ok(result) => McpCallToolResult {
                        id: server_id.clone(),
                        name: tool_name,
                        result: result.content,
                        is_error: false,
                    },
                    Err(err) => McpCallToolResult {
                        id: server_id.clone(),
                        name: tool_name,
                        result: vec![Content::text(format!("Tool execution error: {}", err))],
                        is_error: true,
                    },
                }
            } else {
                // 服务器实例不存在
                McpCallToolResult {
                    id: server_id.clone(),
                    name: tool_name,
                    result: vec![Content::text(format!(
                        "MCP Server instance '{}' not found",
                        server_id
                    ))],
                    is_error: true,
                }
            }
        }
        .into_actor(self)
        .boxed_local()
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
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: UpdateInstanceCache, _ctx: &mut Self::Context) -> Self::Result {
        let instances = self.instances.clone();

        async move {
            let mut instances = instances.write().await;
            if let Some(instance) = msg.instance {
                instances.insert(msg.server_id, instance);
            } else {
                instances.remove(&msg.server_id);
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

// 定义获取实例的消息
#[derive(Message)]
#[rtype(result = "Option<McpServerInstance>")]
pub struct GetServerInstance {
    pub server_id: String,
}

impl Handler<GetServerInstance> for McpRegistry {
    type Result = ResponseActFuture<Self, Option<McpServerInstance>>;

    fn handle(&mut self, msg: GetServerInstance, _ctx: &mut Self::Context) -> Self::Result {
        println!("Getting instance for server_id: {}", msg.server_id);
        let instances = self.instances.clone();
        let server_id = msg.server_id;

        async move {
            let instances = instances.read().await;
            println!("Instances: {:?}", instances.keys());
            instances.get(&server_id).cloned()
        }
        .into_actor(self)
        .boxed_local()
    }
}

// 定义获取所有实例的消息
#[derive(Message)]
#[rtype(result = "Vec<McpServerInstance>")]
pub struct GetAllInstances;

impl Handler<GetAllInstances> for McpRegistry {
    type Result = ResponseActFuture<Self, Vec<McpServerInstance>>;

    fn handle(&mut self, _msg: GetAllInstances, _ctx: &mut Self::Context) -> Self::Result {
        let instances = self.instances.clone();

        async move {
            let instances = instances.read().await;
            let mut instances_vec: Vec<_> = instances.values().cloned().collect();
            instances_vec.sort_by(|a, b| a.config.name.cmp(&b.config.name));
            instances_vec
        }
        .into_actor(self)
        .boxed_local()
    }
}

// 添加更新工具列表的消息
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateInstanceTools {
    pub server_id: String,
    pub tools: Vec<McpTool>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateInstancePrompts {
    pub server_id: String,
    pub prompts: Vec<McpPrompt>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateInstanceResources {
    pub server_id: String,
    pub resources: Vec<McpResource>,
}

// 为 McpRegistry 实现处理器
impl Handler<UpdateInstanceTools> for McpRegistry {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: UpdateInstanceTools, _ctx: &mut Self::Context) -> Self::Result {
        let instances = self.instances.clone();

        async move {
            let mut instances = instances.write().await;
            if let Some(instance) = instances.get_mut(&msg.server_id) {
                instance.tools = msg.tools.clone();
                log::info!("Updated tools for server: {}", msg.server_id);
                // 发送事件通知
                xbus::post(BoEvent::McpToolListUpdated(
                    msg.server_id.clone(),
                    msg.tools,
                ));
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<UpdateInstancePrompts> for McpRegistry {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        UpdateInstancePrompts { server_id, prompts }: UpdateInstancePrompts,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let instances = self.instances.clone();

        async move {
            let mut instances = instances.write().await;
            if let Some(instance) = instances.get_mut(&server_id) {
                instance.prompts = prompts.clone();
                log::info!("Updated prompts for server: {}", server_id);
                xbus::post(BoEvent::McpPromptListUpdated(server_id, prompts));
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<UpdateInstanceResources> for McpRegistry {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        UpdateInstanceResources {
            server_id,
            resources,
        }: UpdateInstanceResources,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let instances = self.instances.clone();

        async move {
            let mut instances = instances.write().await;
            if let Some(instance) = instances.get_mut(&server_id) {
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
                xbus::post(BoEvent::McpResourceListUpdated(
                    server_id,
                    instance.resources.clone(),
                ));
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

// 添加资源更新的消息
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateInstanceResourceContent {
    pub server_id: String,
    pub uri: String,
    pub contents: Vec<ResourceContents>,
}

impl Handler<UpdateInstanceResourceContent> for McpRegistry {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        msg: UpdateInstanceResourceContent,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let instances = self.instances.clone();
        async move {
            let mut instances = instances.write().await;
            if let Some(instance) = instances.get_mut(&msg.server_id) {
                // 查找并更新对应的资源
                for resource_def in &mut instance.resources {
                    if resource_def.resource.uri == msg.uri {
                        resource_def.update_contents(msg.contents.clone());
                        log::info!(
                            "Updated cached content for resource {} in server {}",
                            msg.uri,
                            msg.server_id
                        );
                        // 发送事件通知
                        xbus::post(BoEvent::McpResourceUpdated {
                            server_id: msg.server_id.clone(),
                            uri: msg.uri,
                            contents: msg.contents,
                        });
                        break;
                    }
                }
            } else {
                log::warn!(
                    "Server instance {} not found for resource update",
                    msg.server_id
                );
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

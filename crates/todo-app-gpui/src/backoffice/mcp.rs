mod adaptor;
mod client_handler;
mod loader;
pub(crate) mod server;

use crate::backoffice::mcp::server::{McpServer, McpServerSnapshot};
use crate::backoffice::YamlFile;
use crate::config::mcp_config::*;
use actix::prelude::*;
use rmcp::model::Content;
use std::{collections::HashMap, time::Duration};

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ExitFromRegistry;

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

/// 简化的 MCP Registry - 移除 McpServerInstance，使用快照缓存
pub struct McpRegistry {
    /// MCP 服务器 Actor 地址映射
    servers: HashMap<String, Addr<McpServer>>,
    /// 服务器快照缓存，用于快速查询和跨运行时传递
    snapshots: HashMap<String, McpServerSnapshot>,
    /// 配置文件监控
    file: YamlFile,
    /// 定时检查句柄
    handle: Option<SpawnHandle>,
}

impl McpRegistry {
    /// 获取全局注册表实例
    pub fn global() -> Addr<Self> {
        McpRegistry::from_registry()
    }

    /// 异步工具调用
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

    /// 异步获取服务器快照
    pub async fn get_snapshot(server_id: &str) -> anyhow::Result<Option<McpServerSnapshot>> {
        let result = McpRegistry::global()
            .send(GetServerSnapshot {
                server_id: server_id.to_string(),
            })
            .await?;

        Ok(result)
    }

    /// 异步获取所有服务器快照
    pub async fn get_all_snapshots() -> anyhow::Result<Vec<McpServerSnapshot>> {
        let result = McpRegistry::global().send(GetAllSnapshots).await?;
        Ok(result)
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        let file = YamlFile::new(McpConfigManager::config_path());
        Self {
            servers: HashMap::new(),
            snapshots: HashMap::new(), // 只保留快照缓存
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
    /// 定时检查配置文件变化
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Ok(false) = &self.file.exist() {
            self.servers.clear();
            self.snapshots.clear();
            return;
        }
        if let Err(err) = self.check_and_update(ctx) {
            tracing::error!("{} {err}", self.file.path.display());
        }
    }

    /// 检查配置文件更新并同步服务器状态
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
                    self.snapshots.remove(&server_id); // 同时移除快照缓存
                }
            }
            for config in configs.iter().filter(|c| c.enabled) {
                self.servers.remove(&config.id).map(|addr| {
                    addr.do_send(ExitFromRegistry);
                });
                let addr = McpServer::new(config.clone()).start();
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
        log::info!("McpRegistry started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("McpRegistry stopped");
    }
}

impl Handler<McpCallToolRequest> for McpRegistry {
    type Result = ResponseActFuture<Self, McpCallToolResult>;

    fn handle(&mut self, msg: McpCallToolRequest, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(server) = self.servers.get(&msg.id) {
            let id = msg.id.clone();
            let name = msg.name.clone();
            server
                .send(msg)
                .into_actor(self)
                .map(|res, _act, _ctx| match res {
                    Ok(result) => result,
                    Err(err) => McpCallToolResult {
                        id,
                        name,
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

/// 更新服务器缓存快照
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateServerCache {
    pub server_id: String,
    pub snapshot: Option<McpServerSnapshot>,
}

impl Handler<UpdateServerCache> for McpRegistry {
    type Result = ();

    fn handle(&mut self, msg: UpdateServerCache, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(snapshot) = msg.snapshot {
            log::debug!("Updating snapshot cache for server: {}", msg.server_id);
            self.snapshots.insert(msg.server_id, snapshot);
        } else {
            log::debug!("Removing snapshot cache for server: {}", msg.server_id);
            self.snapshots.remove(&msg.server_id);
        }
    }
}

/// 获取所有服务器快照
#[derive(Message)]
#[rtype(result = "Vec<McpServerSnapshot>")]
pub struct GetAllSnapshots;

impl Handler<GetAllSnapshots> for McpRegistry {
    type Result = Vec<McpServerSnapshot>;

    fn handle(&mut self, _msg: GetAllSnapshots, _ctx: &mut Self::Context) -> Self::Result {
        let mut snapshots: Vec<_> = self.snapshots.values().cloned().collect();
        snapshots.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        snapshots
    }
}

/// 兼容性消息 - 获取实例（实际返回快照）
#[derive(Message)]
#[rtype(result = "Option<McpServerSnapshot>")]
pub struct GetServerSnapshot {
    pub server_id: String,
}

impl Handler<GetServerSnapshot> for McpRegistry {
    type Result = Option<McpServerSnapshot>;

    fn handle(&mut self, msg: GetServerSnapshot, _ctx: &mut Self::Context) -> Self::Result {
        log::debug!("Getting snapshot for server_id: {}", msg.server_id);
        self.snapshots.get(&msg.server_id).cloned()
    }
}

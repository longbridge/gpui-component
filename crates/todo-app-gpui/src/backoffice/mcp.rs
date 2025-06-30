use actix::{prelude::*, WeakAddr};
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo, ClientRequest,
        Content, CreateMessageRequestMethod, CreateMessageRequestParam, CreateMessageResult,
        GetMeta, Implementation, JsonObject, ListRootsResult, LoggingLevel, Meta,
        PingRequestMethod, ProtocolVersion, ReadResourceRequestParam,
        ResourceUpdatedNotificationParam, Root, SubscribeRequest, SubscribeRequestParam,
    },
    service::{NotificationContext, RequestContext},
    ClientHandler, Error as McpError, Peer, RoleClient,
};
use std::any::Any;
use std::{collections::HashMap, time::Duration};

use crate::models::mcp_config::{McpProviderConfig, McpProviderInfo};
use crate::{
    backoffice::{BoEvent, YamlFile},
    xbus,
};

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
    id: String,
    name: String,
    arguments: String,
}

pub struct McpCallToolResult {
    id: String,
    name: String,
    result: Vec<Content>,
    is_error: bool,
}

///     type Result = Result<u8,u8>;
/// }

pub struct McpRegistry {
    providers: HashMap<String, Addr<McpProvider>>,
    file: YamlFile,
}

impl Default for McpRegistry {
    fn default() -> Self {
        let file = YamlFile::new(McpProviderConfig::config_path());
        Self {
            providers: HashMap::new(),
            file,
        }
    }
}

impl Supervised for McpRegistry {}
impl SystemService for McpRegistry {}

impl McpRegistry {
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Ok(false) = &self.file.exist() {
            self.providers.clear();
            return;
        }
        if let Err(err) = self.check_and_update(ctx) {
            log::error!("{} {err}", self.file.path.display());
        }
    }
    fn check_and_update(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
        if self.file.modified()? {
            let providers = McpProviderConfig::load_providers()?;
            let names = providers
                .iter()
                .filter(|provider| provider.enabled)
                .map(|provider| provider.id.as_str())
                .collect::<Vec<&str>>();
            self.providers.retain(|name, addr| {
                if !names.contains(&name.as_str()) {
                    addr.do_send(ExitFromRegistry);
                    false
                } else {
                    true
                }
            });
            for provider in providers.iter().filter(|p| p.enabled) {
                if !self.providers.contains_key(&provider.id) {
                    let mcp_provider = McpProvider::new(provider.clone());
                    let addr = mcp_provider.start();
                    self.providers.insert(provider.id.clone(), addr);
                }
            }
        }
        Ok(())
    }
}

impl Actor for McpRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), Self::tick);
    }
}

pub struct McpProvider {
    info: McpProviderInfo,
}

impl McpProvider {
    pub fn new(info: McpProviderInfo) -> Self {
        Self { info }
    }

    pub fn info(&self) -> &McpProviderInfo {
        &self.info
    }
}
impl Actor for McpProvider {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("McpProvider {}/{} started", self.info.id, self.info.name);
    }
}

impl Handler<ExitFromRegistry> for McpProvider {
    type Result = ();

    fn handle(&mut self, _msg: ExitFromRegistry, ctx: &mut Self::Context) -> Self::Result {
        log::info!("McpProvider {} exit", self.info.id);
        ctx.stop();
    }
}

impl Handler<McpCallToolRequest> for McpProvider {
    type Result = MessageResult<McpCallToolRequest>;

    fn handle(&mut self, msg: McpCallToolRequest, _ctx: &mut Self::Context) -> Self::Result {
        log::info!(
            "McpCallToolRequest: id={}, name={}, arguments={}",
            msg.id,
            msg.name,
            msg.arguments
        );
        // Here you would implement the logic to call the tool and return the result.
        // For now, we return an empty result.
        MessageResult(McpCallToolResult {
            id: msg.id,
            name: msg.name,
            result: vec![],
            is_error: false,
        })
    }
}

pub struct McpClient {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
    pub peer: Option<Peer<RoleClient>>,
    pub id: String,
}

impl ClientHandler for McpClient {
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

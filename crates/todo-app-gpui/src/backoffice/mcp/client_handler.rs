use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::mcp::server::*;
use crate::backoffice::BoEvent;
use rmcp::model::{
    CreateMessageRequestMethod, CreateMessageRequestParam, CreateMessageResult, ListRootsResult,
    LoggingLevel, ProtocolVersion, ReadResourceRequestParam, ResourceUpdatedNotificationParam,
};
use rmcp::service::{NotificationContext, RequestContext};
pub use rmcp::{
    model::{ClientCapabilities, Implementation, Root},
    Error as McpError,
};
use rmcp::{ClientHandler, RoleClient};

#[derive(Debug, Clone)]
pub struct McpClientHandler {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
    pub id: String,
    server: actix::Addr<McpServer>,
}

impl McpClientHandler {
    pub fn new(id: String, server: actix::Addr<McpServer>) -> Self {
        Self {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "xTo-Do/mcp-client".into(),
                version: "0.1.0".into(),
            },
            id,
            server,
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

    async fn on_tool_list_changed(&self, context: NotificationContext<RoleClient>) {
        log::info!("Tool list changed");
        match context.peer.list_tools(None).await {
            Ok(tools) => {
                log::info!("Tool list: {tools:#?}");
                self.server.do_send(UpdateInstanceTools {
                    server_id: self.id.clone(),
                    tools: tools.tools.clone(),
                });
            }
            Err(err) => {
                log::error!("Failed to list tools: {err}");
                CrossRuntimeBridge::global().post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to list tools: {err}"),
                ));
            }
        }
    }

    async fn on_prompt_list_changed(&self, ctx: NotificationContext<RoleClient>) {
        log::info!("Prompt list changed");

        match ctx.peer.list_prompts(None).await {
            Ok(prompts) => {
                log::info!("Prompt list: {prompts:#?}");
                self.server.do_send(UpdateInstancePrompts {
                    server_id: self.id.clone(),
                    prompts: prompts.prompts.clone(),
                });
            }
            Err(err) => {
                log::error!("Failed to list prompts: {err}");
                CrossRuntimeBridge::global().post(BoEvent::Notification(
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
                CrossRuntimeBridge::global().post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to list resources: {err}"),
                ));
            },
            |resources| {
                log::info!("Resource list changed: {resources:#?}");
                self.server.do_send(UpdateInstanceResources {
                    server_id: self.id.clone(),
                    resources: resources.clone(),
                });
            },
        );
    }
    async fn on_cancelled(
        &self,
        params: rmcp::model::CancelledNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        log::info!("Cancelled: {params:#?}");
        CrossRuntimeBridge::global().post(BoEvent::Notification(
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
            CrossRuntimeBridge::global().post(BoEvent::Notification(
                crate::backoffice::NotificationKind::Error,
                format!("Logging error: {}", params.data.to_string()),
            ));
        } else if params.level == LoggingLevel::Warning {
            CrossRuntimeBridge::global().post(BoEvent::Notification(
                crate::backoffice::NotificationKind::Warning,
                format!("Logging warning: {}", params.data.to_string()),
            ));
        } else {
            CrossRuntimeBridge::global().post(BoEvent::Notification(
                crate::backoffice::NotificationKind::Info,
                format!("Logging info: {}", params.data.to_string()),
            ));
        }
        CrossRuntimeBridge::global().post(BoEvent::Notification(
            crate::backoffice::NotificationKind::Info,
            format!("Logging message: {:?}", params.data.to_string()),
        ));
    }

    async fn on_progress(
        &self,
        _params: rmcp::model::ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
    }
    async fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
        tracing::trace!("Resource updated: {}", params.uri);

        match context
            .peer
            .read_resource(ReadResourceRequestParam {
                uri: params.uri.clone(),
            })
            .await
        {
            Ok(result) => {
                tracing::trace!("Resource content read successfully for: {}", params.uri);
                //
                self.server.do_send(UpdateInstanceResourceContent {
                    server_id: self.id.clone(),
                    uri: params.uri.clone(),
                    contents: result.contents.clone(),
                });
            }
            Err(err) => {
                log::error!("Failed to read updated resource {}: {}", params.uri, err);
                CrossRuntimeBridge::global().post(BoEvent::Notification(
                    crate::backoffice::NotificationKind::Error,
                    format!("Failed to read updated resource {}: {}", params.uri, err),
                ));
            }
        }
    }
}

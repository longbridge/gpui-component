use crate::backoffice::mcp::client_handler::McpClientHandler;
use crate::backoffice::mcp::server::{
    McpCapability, McpServer, McpServerSnapshot, ResourceDefinition, ResourceTemplateDefinition,
};
use crate::config::mcp_config::{McpServerConfig, McpTransport};
use actix::Addr;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT};
use rmcp::model::SubscribeRequestParam;
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{service::RunningService, transport::SseClientTransport, RoleClient, ServiceExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

/// MCP 服务器连接加载器
///
/// 负责建立 MCP 连接并初始化服务器能力
/// 与 McpServer Actor 分离，专注于连接逻辑
pub struct McpServerLoader;

impl McpServerLoader {
    /// 启动连接并返回客户端和快照
    pub async fn load_server(
        config: McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<(
        Arc<RunningService<RoleClient, McpClientHandler>>,
        McpServerSnapshot,
    )> {
        tracing::info!(
            "Loading MCP server: {} ({:?})",
            config.name,
            config.transport
        );

        // 根据传输类型建立连接
        let client = match config.transport {
            McpTransport::Stdio => Self::connect_stdio(&config, server_addr).await?,
            McpTransport::Sse => Self::connect_sse(&config, server_addr).await?,
            McpTransport::Streamable => Self::connect_streamable(&config, server_addr).await?,
        };

        // 初始化服务器能力
        let (client_arc, snapshot) = Self::initialize_capabilities(config, client).await?;

        log::info!("MCP server loaded successfully: {}", snapshot.config.name);
        Ok((client_arc, snapshot))
    }

    /// 建立 STDIO 连接
    async fn connect_stdio(
        config: &McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<RunningService<RoleClient, McpClientHandler>> {
        log::debug!("Connecting via STDIO: {}", config.command);

        let mut command_parts = config.command.split(" ");
        let program = command_parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty command string"))?;

        let command = Command::new(program).configure(|cmd| {
            let args: Vec<_> = command_parts.collect();
            cmd.kill_on_drop(true).args(args).envs(&config.env_vars);

            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);
        });

        let transport = TokioChildProcess::new(command)?;

        // Windows 作业对象管理
        #[cfg(target_family = "windows")]
        Self::setup_windows_job_object(&transport);

        let client_handler = McpClientHandler::new(config.id.clone(), server_addr);
        let client = client_handler.serve(transport).await?;

        Ok(client)
    }

    /// 建立 SSE 连接
    async fn connect_sse(
        config: &McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<RunningService<RoleClient, McpClientHandler>> {
        log::debug!("Connecting via SSE: {}", config.command);

        let headers = Self::build_headers(&config.env_vars)?;
        let http_client = Self::build_http_client(headers)?;

        let transport = SseClientTransport::start_with_client(
            http_client,
            SseClientConfig {
                sse_endpoint: config.command.clone().into(),
                ..Default::default()
            },
        )
        .await?;

        let client_handler = McpClientHandler::new(config.id.clone(), server_addr);
        let client = client_handler.serve(transport).await?;

        Ok(client)
    }

    /// 建立 Streamable HTTP 连接
    async fn connect_streamable(
        config: &McpServerConfig,
        server_addr: Addr<McpServer>,
    ) -> anyhow::Result<RunningService<RoleClient, McpClientHandler>> {
        log::debug!("Connecting via Streamable HTTP: {}", config.command);

        let headers = Self::build_headers(&config.env_vars)?;
        let http_client = Self::build_http_client(headers)?;

        let transport = StreamableHttpClientTransport::with_client(
            http_client,
            StreamableHttpClientTransportConfig {
                uri: config.command.clone().into(),
                ..Default::default()
            },
        );

        let client_handler = McpClientHandler::new(config.id.clone(), server_addr);
        let client = client_handler.serve(transport).await?;

        Ok(client)
    }

    /// 初始化服务器能力
    async fn initialize_capabilities(
        config: McpServerConfig,
        client: RunningService<RoleClient, McpClientHandler>,
    ) -> anyhow::Result<(
        Arc<RunningService<RoleClient, McpClientHandler>>,
        McpServerSnapshot,
    )> {
        log::debug!("Initializing server capabilities for: {}", config.name);

        // 获取服务器信息并初始化
        let server_info = client.peer_info().cloned().unwrap_or_default();
        client.notify_initialized().await?;

        let mut capabilities = Vec::new();
        let mut tools = Vec::new();
        let mut prompts = Vec::new();
        let mut resources = Vec::new();
        let mut resource_templates = Vec::new();
        let mut keepalive = false;

        // 检查并加载工具能力
        if let Some(tools_capability) = server_info.capabilities.tools {
            log::debug!("Loading tools for server: {}", config.name);
            tools = client.list_all_tools().await?;
            capabilities.push(McpCapability::Tools);

            if tools_capability.list_changed.unwrap_or(false) {
                keepalive = true;
                log::debug!("Server {} supports tool list changes", config.name);
            }
        }

        // 检查并加载提示能力
        if let Some(prompts_capability) = server_info.capabilities.prompts {
            log::debug!("Loading prompts for server: {}", config.name);
            prompts = client.list_all_prompts().await?;
            capabilities.push(McpCapability::Prompts);

            if prompts_capability.list_changed.unwrap_or(false) {
                keepalive = true;
                log::debug!("Server {} supports prompt list changes", config.name);
            }
        }

        // 检查并加载资源能力
        if let Some(resources_capability) = server_info.capabilities.resources {
            log::debug!("Loading resources for server: {}", config.name);

            let resource_list = client.list_all_resources().await?;
            let template_list = client.list_all_resource_templates().await?;
            let subscribable = resources_capability.subscribe.unwrap_or_default();

            // 构建资源定义
            resources = resource_list
                .into_iter()
                .map(|r| ResourceDefinition::new(r, subscribable))
                .collect();

            resource_templates = template_list
                .into_iter()
                .map(|t| ResourceTemplateDefinition {
                    resource_template: t,
                    subscribed: false,
                    subscribable,
                })
                .collect();

            capabilities.push(McpCapability::Resources);

            if resources_capability.list_changed.unwrap_or(false) {
                keepalive = true;
                log::debug!("Server {} supports resource list changes", config.name);
            }

            // 处理资源订阅
            if subscribable {
                keepalive = true;
                for resource_name in &config.subscribed_resources {
                    if let Some(resource) = resources
                        .iter_mut()
                        .find(|r| r.resource.name == *resource_name)
                    {
                        log::debug!(
                            "Subscribing to resource: {} ({})",
                            resource.resource.name,
                            resource.resource.uri
                        );

                        match client
                            .subscribe(SubscribeRequestParam {
                                uri: resource.resource.uri.clone(),
                            })
                            .await
                        {
                            Ok(_) => {
                                resource.subscribed = true;
                                log::debug!(
                                    "Successfully subscribed to resource: {}",
                                    resource.resource.name
                                );
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to subscribe to resource {}: {}",
                                    resource.resource.name,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        // 创建客户端 Arc 包装
        let client_arc = Arc::new(client);

        // 创建服务器快照
        let snapshot = McpServerSnapshot::from_server_info(
            config,
            capabilities,
            tools,
            prompts,
            resources,
            resource_templates,
            keepalive,
        );

        log::debug!(
            "Server capabilities initialized: {:?}",
            snapshot.capabilities
        );
        Ok((client_arc, snapshot))
    }

    /// 构建 HTTP 请求头
    fn build_headers(
        env_vars: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<HeaderMap<HeaderValue>> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

        for (key, val) in env_vars.iter() {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = val.parse()?;
            headers.insert(header_name, header_value);
        }

        Ok(headers)
    }

    /// 构建 HTTP 客户端
    fn build_http_client(headers: HeaderMap<HeaderValue>) -> anyhow::Result<reqwest::Client> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .read_timeout(Duration::from_secs(900))
            .default_headers(headers)
            .build()?;

        Ok(client)
    }

    /// Windows 作业对象设置
    #[cfg(target_family = "windows")]
    fn setup_windows_job_object(transport: &TokioChildProcess) {
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectA, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

        let _ = transport
            .id()
            .and_then(|pid: u32| unsafe { OpenProcess(PROCESS_ALL_ACCESS, true, pid).ok() })
            .and_then(|hprocess| unsafe {
                CreateJobObjectA(None, windows::core::s!("x-todo-mcp-job"))
                    .ok()
                    .and_then(|hjob| {
                        let mut job_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                        job_info.BasicLimitInformation.LimitFlags =
                            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                        SetInformationJobObject(
                            hjob,
                            JobObjectExtendedLimitInformation,
                            &job_info as *const _ as *const std::ffi::c_void,
                            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                        )
                        .ok()
                        .and_then(|_| AssignProcessToJobObject(hjob, hprocess).ok())
                    })
            });
    }
}

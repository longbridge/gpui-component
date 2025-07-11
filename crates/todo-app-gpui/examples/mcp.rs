#![windows_subsystem = "windows"]
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
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    service::{RunningService, ServerSink},
    transport::{auth::AuthClient, auth::OAuthState, SseClientTransport},
    RoleClient,
};
pub use rmcp::{
    model::{
        ClientCapabilities, ClientInfo, Implementation, Prompt as McpPrompt,
        Resource as McpResource, ResourceTemplate as McpResourceTemplate, Tool as McpTool,
    },
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, env::home_dir};
use tokio::process::Command;

use anyhow::{Ok, Result};
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_vars = HashMap::from([("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(),"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".to_string())]);
    let command = Command::new("github-mcp-server.exe").configure(|cmd| {
        cmd.args(&["stdio"]).envs(&env_vars);
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000);
        }
    });

    let transport = TokioChildProcess::new(command)?;
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "xTo-Do/mcp-client".to_string(),
            version: "0.0.1".to_string(),
        },
    };
    let client = client_info.serve(transport).await?;
    let server_info = client.peer_info().cloned().unwrap_or_default();
    println!("Server info: {:#?}", server_info);
    if let Some(capability) = server_info.capabilities.tools {
        let tools = client.list_all_tools().await?;
        println!("Tools: {:#?}", tools);
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
        println!("Prompts: {:#?}", prompts);
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
        println!("Resources: {:#?}", resources);
        println!("Resource templates: {:#?}", resource_templates);

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
    loop {}
}

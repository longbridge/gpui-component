pub mod agentic;
mod builtin;
// pub mod crb;
pub mod cross_runtime;
pub mod llm;
pub mod mcp;
mod todo;

use actix::prelude::*;
use anyhow::Ok;
use gpui_component::notification::Notification;
use rmcp::model::{Prompt, ReadResourceResult, Resource, ResourceContents, Tool};
use std::fs::File;

use crate::{
    backoffice::{
        cross_runtime::CrossRuntimeBridge,
        llm::LlmRegistry,
        mcp::{server::ResourceDefinition, GetServerSnapshot, McpRegistry},
    },
    config::mcp_config::McpServerConfig,
};

///后台事件
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum BoEvent {
    TodoUpdated,
    LlmConfigUpdated,
    McpServerStarted(McpServerConfig),
    McpToolListUpdated(String, Vec<Tool>),
    McpResourceListUpdated(String, Vec<ResourceDefinition>),
    McpPromptListUpdated(String, Vec<Prompt>),
    McpResourceResult(String, ReadResourceResult),
    McpSamplingRequest(String, String, String),
    McpResourceUpdated {
        server_id: String,
        uri: String,
        contents: Vec<ResourceContents>,
    },
    Notification(NotificationKind, String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum NotificationKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl BoEvent {
    pub fn is_todo_updated(&self) -> bool {
        matches!(self, BoEvent::TodoUpdated)
    }

    pub fn is_llm_config_updated(&self) -> bool {
        matches!(self, BoEvent::LlmConfigUpdated)
    }

    pub fn is_mcp_tool_updated(&self) -> bool {
        matches!(self, BoEvent::McpToolListUpdated(_, _))
    }

    pub fn is_mcp_resource_updated(&self) -> bool {
        matches!(self, BoEvent::McpResourceListUpdated(_, _))
    }

    pub fn is_mcp_prompt_updated(&self) -> bool {
        matches!(self, BoEvent::McpPromptListUpdated(_, _))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, BoEvent::Notification(_, _))
    }

    pub fn to_notification(&self) -> Option<Notification> {
        match self {
            BoEvent::Notification(kind, message) => match kind {
                NotificationKind::Info => Some(Notification::info(message.clone())),
                NotificationKind::Success => Some(Notification::success(message.clone())),
                NotificationKind::Warning => Some(Notification::warning(message.clone())),
                NotificationKind::Error => Some(Notification::error(message.clone())),
            },
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct YamlFile {
    pub path: std::path::PathBuf,
    pub mtime: u64,
}

impl YamlFile {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            mtime: 0,
        }
    }

    // pub fn with<S: AsRef<std::path::Path>>(path: S) -> Self {
    //     Self {
    //         path: path.as_ref().to_path_buf(),
    //         mtime: 0,
    //     }
    // }

    pub fn open(&mut self) -> anyhow::Result<File> {
        let file = File::open(self.path.as_path())?;
        self.mtime = mtime(self.path.as_path())?;
        Ok(file)
    }

    pub fn exist(&mut self) -> anyhow::Result<bool> {
        let exist = std::fs::exists(self.path.as_path())?;
        if !exist {
            self.mtime = 0;
        }
        Ok(exist)
    }

    pub fn modified(&self) -> anyhow::Result<bool> {
        #[cfg(target_family = "unix")]
        use std::os::unix::fs::MetadataExt;
        Ok(self.mtime != mtime(self.path.as_path())?)
    }
}

fn mtime<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<u64> {
    #[cfg(target_family = "unix")]
    let mtime = {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path.as_ref())?.mtime() as u64
    };
    #[cfg(target_family = "windows")]
    let mtime = {
        use std::os::windows::fs::MetadataExt;
        std::fs::metadata(path.as_ref())?.last_write_time()
    };
    Ok(mtime)
}

pub fn start() -> anyhow::Result<()> {
    let threads = std::thread::available_parallelism()?.get();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .enable_all()
        .build()?;
    std::thread::spawn(move || {
        let sys = System::with_tokio_rt(|| rt);
        sys.block_on(async {
            CrossRuntimeBridge::global();
            McpRegistry::global();
            LlmRegistry::global();
        });
        sys.run().ok();
    });
    Ok(())
}

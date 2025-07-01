mod agentic;
mod builtin;
pub mod mcp;
mod meta;
mod todo;

use actix::prelude::*;
use gpui_component::notification::Notification;
use rmcp::model::{Prompt, ReadResourceResult, Resource, ResourceContents, Tool};
use std::fs::File;

use crate::{backoffice::mcp::McpRegistry, models::mcp_config::McpServerConfig};

///后台事件
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum BoEvent {
    TodoUpdated,
    LlmConfigUpdated,
    McpServerStarted(McpServerConfig),
    McpToolListUpdated(String, Vec<Tool>),
    McpResourceListUpdated(String, Vec<Resource>),
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

pub fn start() {
    let addr = McpRegistry::from_registry();
    println!("McpRegistry started at {:?}", addr);
}

// #[derive(Default)]
// pub struct Environment {
//     agents: HashMap<String, PathTree<WeakRecipient<PublishToAgent>>>,
//     turing: Option<Addr<TuringMachine>>,
//     perceptron: Option<Addr<Perceptron>>,
//     toolkit: Option<Addr<Toolkit>>,
//     uikit: Option<Addr<UIkit>>,
// }

// impl Environment {
//     pub fn create() {
//         //创建环境
//         Self::from_registry();
//     }
// }

// impl Actor for Environment {
//     type Context = Context<Self>;
//     fn started(&mut self, _ctx: &mut Self::Context) {
//         //启动图灵机
//         let turing = TuringMachine::default().start();
//         self.turing = Some(turing);
//         // //启动执行器
//         // Executor::from_registry();
//         //启动感知机
//         let perceptron = Perceptron::default().start();
//         self.perceptron = Some(perceptron);
//         //启动工具库
//         self.toolkit = Some(Toolkit::default().start());
//         //启动UI
//         self.uikit = Some(UIkit::default().start());
//     }
// }

// impl Supervised for Environment {}
// impl SystemService for Environment {}

// impl Handler<RegisterForAgent> for Environment {
//     type Result = ();
//     fn handle(
//         &mut self,
//         RegisterForAgent(name, tree): RegisterForAgent,
//         _ctx: &mut Self::Context,
//     ) -> Self::Result {
//         log::debug!("RegisterForAgent {name} {:?}", tree.node);
//         self.agents.insert(name, tree);
//     }
// }

// impl Handler<UnregisterForAgent> for Environment {
//     type Result = ();
//     fn handle(
//         &mut self,
//         UnregisterForAgent(name): UnregisterForAgent,
//         _ctx: &mut Self::Context,
//     ) -> Self::Result {
//         log::debug!("UnregisterForAgent {name}");
//         self.agents.remove(&name);
//     }
// }

// impl Handler<PublishToAgent> for Environment {
//     type Result = anyhow::Result<()>;
//     fn handle(&mut self, msg: PublishToAgent, ctx: &mut Self::Context) -> Self::Result {
//         log::debug!("{:?}", msg);
//         let path = msg.path.clone();
//         for (_k, tree) in self.agents.iter() {
//             if let Some((recipient, _)) = tree.find(&path) {
//                 recipient.upgrade().as_ref().map(|recipient| {
//                     recipient
//                         .send(msg.clone())
//                         .into_actor(self)
//                         .then(move |ret, _this, _ctx| {
//                             match ret {
//                                 Ok(Ok(_)) => {}
//                                 Ok(Err(err)) => {
//                                     log::error!("{err}");
//                                 }
//                                 Err(err) => {
//                                     log::error!("{err}");
//                                 }
//                             }
//                             fut::ready(())
//                         })
//                         .spawn(ctx);
//                 });
//             }
//         }
//         Ok(())
//     }
// }

// pub trait Agent
// where
//     Self: Actor,
//     <Self as Actor>::Context: AsyncContext<Self>,
// {
//     fn register<S: AsRef<str>>(&self, name: String, subscribe: &[S], ctx: &mut Self::Context)
//     where
//         Self: Handler<PublishToAgent>,
//         <Self as Actor>::Context: ToEnvelope<Self, PublishToAgent>,
//     {
//         let mut tree = PathTree::new();
//         subscribe.iter().for_each(|path| {
//             let _ = tree.insert(path.as_ref(), ctx.address().downgrade().recipient());
//         });
//         Environment::from_registry().do_send(RegisterForAgent(name, tree));
//     }

//     fn unregister(&self, name: String) {
//         Environment::from_registry().do_send(UnregisterForAgent(name));
//     }

//     fn notify<S: AsRef<str>>(&self, source: S, path: S) {
//         if let Err(err) = PublishToAgent::new(source.as_ref(), Kind::Timer).publish_to(path) {
//             log::warn!("{err}");
//         }
//     }

//     fn parser<S: AsRef<str>>(&self, path: S) -> anyhow::Result<Url> {
//         let url = Url::options()
//             .base_url(Url::parse("xagent://xinsight").ok().as_ref())
//             .parse(path.as_ref())?;
//         Ok(url)
//     }
// }

// impl<A> Agent for A
// where
//     A: Actor,
//     <A as Actor>::Context: AsyncContext<A>,
// {
// }

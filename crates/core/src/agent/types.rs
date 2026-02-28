use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::llm::manager::GlobalProviderState;
use crate::llm::{Message, ProviderConfig};
use crate::storage::StorageManager;

/// Describes an agent's identity and routing metadata.
#[derive(Debug, Clone)]
pub struct AgentDescriptor {
    /// Unique agent identifier (e.g. "general_chat").
    pub id: &'static str,
    /// Human-readable name shown in the UI.
    pub display_name: &'static str,
    /// Short description of what this agent does, used for intent routing prompts.
    pub description: &'static str,
    /// Keywords that trigger rule-based routing (matched against user input).
    pub keywords: &'static [&'static str],
    /// Optional command prefix (e.g. "/sql") for direct invocation.
    pub command_prefix: Option<&'static str>,
    /// Example prompts that would route to this agent.
    pub examples: &'static [&'static str],
    /// Capability keys that must be present in `AgentContext` for this agent to be available.
    pub required_capabilities: &'static [&'static str],
    /// Lower values = higher priority when multiple agents match equally.
    pub priority: u32,
}

/// Runtime context passed to an agent during execution.
pub struct AgentContext {
    /// The user's current input text.
    pub user_input: String,
    /// Recent chat history (may be truncated).
    pub chat_history: Vec<Message>,
    /// The provider configuration to use for LLM calls.
    pub provider_config: ProviderConfig,
    /// The global provider state for obtaining LLM providers.
    pub provider_state: GlobalProviderState,
    /// Access to persistent storage.
    pub storage_manager: StorageManager,
    /// Cancellation token for cooperative cancellation.
    pub cancel_token: CancellationToken,
    /// Dynamic capabilities map — agents can check for domain-specific resources.
    capabilities: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl AgentContext {
    pub fn new(
        user_input: String,
        chat_history: Vec<Message>,
        provider_config: ProviderConfig,
        provider_state: GlobalProviderState,
        storage_manager: StorageManager,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            user_input,
            chat_history,
            provider_config,
            provider_state,
            storage_manager,
            cancel_token,
            capabilities: HashMap::new(),
        }
    }

    /// Insert a capability value keyed by name.
    pub fn set_capability<T: Any + Send + Sync>(&mut self, key: impl Into<String>, value: T) {
        self.capabilities.insert(key.into(), Box::new(value));
    }

    /// Check whether a capability key exists.
    pub fn has_capability(&self, key: &str) -> bool {
        self.capabilities.contains_key(key)
    }

    /// Retrieve a capability value by key, downcasting to `T`.
    pub fn get_capability<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.capabilities
            .get(key)
            .and_then(|v| v.downcast_ref::<T>())
    }
}

/// Events emitted by an agent during execution, sent over an mpsc channel.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Progress indicator (e.g. "Analyzing query...").
    Progress(String),
    /// Incremental text content.
    TextDelta(String),
    /// Agent finished successfully.
    Completed(AgentResult),
    /// An error occurred.
    Error(String),
    /// The operation was cancelled via `CancellationToken`.
    Cancelled,
}

/// The final result produced by an agent.
#[derive(Debug, Clone, Default)]
pub struct AgentResult {
    /// Main text content of the response.
    pub content: String,
    /// Structured artifacts produced during execution.
    pub artifacts: Vec<Artifact>,
    /// Suggested follow-up prompts.
    pub suggested_followups: Vec<String>,
}

/// A structured artifact produced by an agent.
#[derive(Debug, Clone)]
pub enum Artifact {
    /// A SQL query.
    Sql(String),
    /// A code snippet with optional language tag.
    Code {
        language: Option<String>,
        content: String,
    },
    /// An arbitrary JSON artifact.
    Custom(Value),
}

/// The core agent trait. All agents must implement this.
#[async_trait]
pub trait Agent: Send + Sync + 'static {
    /// Return this agent's static descriptor.
    fn descriptor(&self) -> &AgentDescriptor;

    /// Check whether this agent is available given the current context.
    ///
    /// Default implementation checks that every `required_capabilities` key
    /// is present in `ctx.capabilities`.
    fn is_available(&self, ctx: &AgentContext) -> bool {
        let desc = self.descriptor();
        desc.required_capabilities
            .iter()
            .all(|cap| ctx.has_capability(cap))
    }

    /// Execute the agent. Send events (deltas, progress, completion) through `tx`.
    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>);
}

/// Convenience alias for a thread-safe agent reference.
pub type DynAgent = Arc<dyn Agent>;

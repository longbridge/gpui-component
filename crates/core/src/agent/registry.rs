use std::collections::HashMap;
use std::sync::Arc;

use gpui::{App, Global};

use super::builtin::GeneralChatAgent;
use super::types::{Agent, AgentContext, DynAgent};

/// Global registry that holds all available agents.
#[derive(Clone)]
pub struct AgentRegistry {
    agents: HashMap<&'static str, DynAgent>,
    /// Agent IDs sorted by priority (ascending = higher priority first).
    sorted_ids: Vec<&'static str>,
}

impl Global for AgentRegistry {}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            sorted_ids: Vec::new(),
        }
    }

    /// Register an agent. Replaces any existing agent with the same id.
    pub fn register(&mut self, agent: impl Agent) {
        let id = agent.descriptor().id;
        self.agents.insert(id, Arc::new(agent));
        self.rebuild_sorted_ids();
    }

    /// Register a pre-wrapped `Arc<dyn Agent>`.
    pub fn register_arc(&mut self, agent: DynAgent) {
        let id = agent.descriptor().id;
        self.agents.insert(id, agent);
        self.rebuild_sorted_ids();
    }

    /// Get an agent by id.
    pub fn get(&self, id: &str) -> Option<&DynAgent> {
        self.agents.get(id)
    }

    /// Return all registered agents.
    pub fn all(&self) -> &HashMap<&'static str, DynAgent> {
        &self.agents
    }

    /// Return agents that are available in the given context, sorted by priority.
    pub fn available_agents(&self, ctx: &AgentContext) -> Vec<&DynAgent> {
        self.sorted_ids
            .iter()
            .filter_map(|id| {
                let agent = self.agents.get(id)?;
                if agent.is_available(ctx) {
                    Some(agent)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Return sorted agent IDs (by priority).
    pub fn sorted_ids(&self) -> &[&'static str] {
        &self.sorted_ids
    }

    fn rebuild_sorted_ids(&mut self) {
        let mut entries: Vec<_> = self
            .agents
            .iter()
            .map(|(&id, agent)| (id, agent.descriptor().priority))
            .collect();
        entries.sort_by_key(|&(_, priority)| priority);
        self.sorted_ids = entries.into_iter().map(|(id, _)| id).collect();
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the agent registry with built-in agents and set it as a GPUI global.
pub fn init(cx: &mut App) {
    let mut registry = AgentRegistry::new();
    registry.register(GeneralChatAgent);
    cx.set_global(registry);
}

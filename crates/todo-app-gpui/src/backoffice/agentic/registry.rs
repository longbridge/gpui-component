use super::*;
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct Registry<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> {
    agents: HashMap<String, AiAgent<M, L>>,
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Registry<M, L> {
    pub fn register_agent(&mut self, agent: AiAgent<M, L>) {
        self.agents
            .insert(agent.execution_context().session_id.clone(), agent);
    }

    pub fn get_agent(&self, name: &str) -> Option<&AiAgent<M, L>> {
        self.agents.get(name)
    }

    pub fn all_agents(&self) -> Vec<&AiAgent<M, L>> {
        self.agents.values().collect()
    }
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Actor for Registry<M, L> {
    type Context = Context<Self>;
}

impl<M: Memory + Send + Unpin + 'static, L: LLM + Send + Unpin + 'static> Supervised
    for Registry<M, L>
{
}

impl<M: Memory + Send + Unpin + Default + 'static, L: LLM + Send + Unpin + Default + 'static>
    SystemService for Registry<M, L>
{
}

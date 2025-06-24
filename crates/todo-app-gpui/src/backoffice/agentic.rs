mod llm;
mod mcp;
mod prompts;

use actix::prelude::*;

pub struct AgenticAwareness {
    pub llm: llm::LlmRegistry,
    pub mcp: mcp::McpRegistry,
}

impl AgenticAwareness {
    pub fn new() -> Self {
        Self {
            llm: llm::LlmRegistry::new(),
            mcp: mcp::McpRegistry::new(),
        }
    }
}

impl Default for AgenticAwareness {
    fn default() -> Self {
        Self::new()
    }
}

impl Actor for AgenticAwareness {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        // Initialization logic if needed
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // Cleanup logic if needed
    }
}

impl Supervised for AgenticAwareness {}
impl SystemService for AgenticAwareness {}

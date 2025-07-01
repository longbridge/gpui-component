mod feedback;
mod llm;
mod prompts;
mod regulator;

use actix::prelude::*;

pub struct AgenticAwareness {
    pub llm: llm::LlmRegistry,
}

impl AgenticAwareness {
    pub fn new() -> Self {
        Self {
            llm: llm::LlmRegistry::new(),
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

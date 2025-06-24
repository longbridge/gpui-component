mod llm;
mod mcp;
mod prompts;

pub struct AgenticAwareness {
    pub llm: llm::LlmRegistry,
    pub mcp: mcp::McpRegistry,
}

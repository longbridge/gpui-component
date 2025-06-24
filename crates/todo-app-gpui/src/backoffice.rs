mod agentic;
mod builtin;
mod meta;
mod todo;

///后台事件
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum BoEvent {
    TodoUpdated,
    LlmConfigUpdated,
    LlmMessage,
    McpToolUpdated,
    McpResourceUpdated,
    McpMessage,
}

use gpui_component::notification::Notification;

mod agentic;
mod builtin;
mod meta;
mod todo;

///后台事件
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum BoEvent {
    TodoUpdated,
    LlmConfigUpdated,
    McpToolUpdated,
    McpResourceUpdated,
    McpPromptUpdated,
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
        matches!(self, BoEvent::McpToolUpdated)
    }

    pub fn is_mcp_resource_updated(&self) -> bool {
        matches!(self, BoEvent::McpResourceUpdated)
    }

    pub fn is_mcp_prompt_updated(&self) -> bool {
        matches!(self, BoEvent::McpPromptUpdated)
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

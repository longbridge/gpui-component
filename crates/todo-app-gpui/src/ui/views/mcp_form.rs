// filepath: todo-app-gpui/todo-app-gpui/src/views/mcp_form.rs
use crate::models::mcp_config::MCPConfig;
use gpui::*;
use gpui_component::input::TextInput;
pub struct MCPForm {
    config: MCPConfig,
}

impl MCPForm {
    pub fn new() -> Self {
        Self {
            config: MCPConfig::default(),
        }
    }

    pub fn view(&mut self, cx: &mut Context<Self>) -> Entity<Self> {
        cx.new(|cx| {
            div()
                .p_4()
                .child(
                    TextInput::new(&self.config.name)
                        .label("MCP Name")
                        .on_change(cx.listener(|this, value: &str, _, _| {
                            this.config.name = value.to_string();
                        })),
                )
                .child(
                    TextInput::new(&self.config.endpoint)
                        .label("Endpoint")
                        .on_change(cx.listener(|this, value: &str, _, _| {
                            this.config.endpoint = value.to_string();
                        })),
                )
                .child(Button::new("Save").on_click(cx.listener(|this, _, _, _| {
                    // Handle form submission logic here
                })))
        })
    }
}

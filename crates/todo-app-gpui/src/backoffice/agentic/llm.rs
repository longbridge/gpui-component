use super::*;
use crate::backoffice::agentic::mcp_tools::McpToolDelegate;
use crate::backoffice::llm::LlmRegistry;
use crate::config::todo_item::SelectedTool;
use futures::StreamExt;

/// 基于 LlmRegistry 的 LLM 适配器
pub struct LlmAdapter {
    /// 提供商ID
    pub provider_id: String,
    /// 模型ID
    pub model_id: String,
    /// 请求来源标识
    pub source: String,
    /// 工具委托
    pub tool_delegate: Option<McpToolDelegate>,
}

impl LlmAdapter {
    /// 创建新的 LLMChat 实例
    pub fn new(provider_id: String, model_id: String, source: String) -> Self {
        Self {
            provider_id,
            model_id,
            source,
            tool_delegate: None,
        }
    }

    /// 设置工具委托
    pub fn with_tools(mut self, selected_tools: Vec<SelectedTool>) -> Self {
        self.tool_delegate = Some(McpToolDelegate::new(selected_tools));
        self
    }

    /// 设置工具委托超时
    pub fn with_tool_timeout(mut self, timeout_seconds: u64) -> Self {
        if let Some(delegate) = &mut self.tool_delegate {
            delegate.timeout_seconds = timeout_seconds;
        }
        self
    }

    /// 构建系统消息，包含工具信息
    async fn build_system_message(&self, base_messages: &[ChatMessage]) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // 如果有工具，添加工具定义到系统消息
        if let Some(delegate) = &self.tool_delegate {
            let tools = delegate.available_tools().await;
            if !tools.is_empty() {
                let tool_descriptions = tools
                    .iter()
                    .map(|tool| {
                        format!(
                            "- {}: {} (参数: {})",
                            tool.name, tool.description, tool.parameters
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let system_message = format!(
                    "你是一个智能助手，可以使用以下工具来帮助用户：\n\n{}\n\n当需要使用工具时，请使用 <tool_use> 格式。",
                    tool_descriptions
                );

                messages.push(ChatMessage::system().with_text(system_message));
            }
        }

        // 添加原始消息
        messages.extend_from_slice(base_messages);
        messages
    }

    /// 处理工具调用
    async fn handle_tool_calls(&self, message: &ChatMessage) -> anyhow::Result<Vec<ChatMessage>> {
        let mut result_messages = Vec::new();

        // 检查消息中是否包含工具调用
        let content = message.get_text();
        if content.contains("<tool_use>") {
            if let Some(delegate) = &self.tool_delegate {
                // 这里应该解析工具调用并执行
                // 简化实现，实际应该使用更sophisticated的解析
                if let Some(tool_call) = self.parse_tool_call(&content) {
                    match delegate.call(&tool_call.name, tool_call.args).await {
                        Ok(result) => {
                            let result_text = format!("工具调用结果: {:?}", result);
                            result_messages.push(ChatMessage::assistant().with_text(result_text));
                        }
                        Err(e) => {
                            let error_text = format!("工具调用失败: {}", e);
                            result_messages.push(ChatMessage::assistant().with_text(error_text));
                        }
                    }
                }
            }
        }

        if result_messages.is_empty() {
            result_messages.push(message.clone());
        }

        Ok(result_messages)
    }

    /// 解析工具调用（简化实现）
    fn parse_tool_call(&self, content: &str) -> Option<ToolCall> {
        // 这里应该实现更sophisticated的XML解析
        // 简化实现，寻找 <tool_use> 标签
        if let Some(start) = content.find("<tool_use>") {
            if let Some(end) = content.find("</tool_use>") {
                let tool_content = &content[start + 10..end];

                // 提取工具名称和参数
                if let Some(name_start) = tool_content.find("<name>") {
                    if let Some(name_end) = tool_content.find("</name>") {
                        let name = &tool_content[name_start + 6..name_end];

                        // 提取参数
                        let args = if let Some(args_start) = tool_content.find("<arguments>") {
                            if let Some(args_end) = tool_content.find("</arguments>") {
                                &tool_content[args_start + 11..args_end]
                            } else {
                                ""
                            }
                        } else {
                            ""
                        };

                        return Some(ToolCall {
                            name: name.to_string(),
                            args: args.to_string(),
                        });
                    }
                }
            }
        }
        None
    }
}

/// 工具调用结构
#[derive(Debug, Clone)]
struct ToolCall {
    name: String,
    args: String,
}

impl LLM for LlmAdapter {
    type ToolDelegate = McpToolDelegate;

    async fn completion_stream(&self, prompts: &[ChatMessage]) -> anyhow::Result<ChatStream> {
        let messages = self.build_system_message(prompts).await;

        LlmRegistry::chat_stream(&self.provider_id, &self.model_id, &self.source, messages).await
    }

    async fn completion_with_tools_stream(
        &self,
        prompts: &[ChatMessage],
        _tools: &Self::ToolDelegate,
    ) -> anyhow::Result<ChatStream> {
        // 使用内置的工具委托
        let messages = self.build_system_message(prompts).await;

        LlmRegistry::chat_stream(&self.provider_id, &self.model_id, &self.source, messages).await
    }

    async fn chat_stream(&self, messages: &[ChatMessage]) -> anyhow::Result<ChatStream> {
        let enhanced_messages = self.build_system_message(messages).await;

        LlmRegistry::chat_stream(
            &self.provider_id,
            &self.model_id,
            &self.source,
            enhanced_messages,
        )
        .await
    }

    async fn chat_with_tools_stream(
        &self,
        messages: &[ChatMessage],
        _tools: &Self::ToolDelegate,
    ) -> anyhow::Result<ChatStream> {
        // 使用内置的工具委托
        let enhanced_messages = self.build_system_message(messages).await;

        LlmRegistry::chat_stream(
            &self.provider_id,
            &self.model_id,
            &self.source,
            enhanced_messages,
        )
        .await
    }

    async fn completion(&self, prompts: &[ChatMessage]) -> anyhow::Result<ChatMessage> {
        let mut stream = self.completion_stream(prompts).await?;
        let mut accumulated_text = String::new();
        let mut final_message = None;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(message) => {
                    accumulated_text.push_str(&message.get_text());
                    final_message = Some(message);
                }
                Err(e) => return Err(e),
            }
        }

        if let Some(mut message) = final_message {
            // 处理工具调用
            if accumulated_text.contains("<tool_use>") {
                let tool_results = self.handle_tool_calls(&message).await?;
                if let Some(result) = tool_results.first() {
                    message = result.clone();
                }
            }
            Ok(message)
        } else {
            Ok(ChatMessage::assistant().with_text(accumulated_text))
        }
    }

    async fn chat(&self, messages: &[ChatMessage]) -> anyhow::Result<ChatMessage> {
        let mut stream = self.chat_stream(messages).await?;
        let mut accumulated_text = String::new();
        let mut final_message = None;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(message) => {
                    accumulated_text.push_str(&message.get_text());
                    final_message = Some(message);
                }
                Err(e) => return Err(e),
            }
        }

        if let Some(mut message) = final_message {
            // 处理工具调用
            if accumulated_text.contains("<tool_use>") {
                let tool_results = self.handle_tool_calls(&message).await?;
                if let Some(result) = tool_results.first() {
                    message = result.clone();
                }
            }
            Ok(message)
        } else {
            Ok(ChatMessage::assistant().with_text(accumulated_text))
        }
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        _tools: &Self::ToolDelegate,
    ) -> anyhow::Result<ChatMessage> {
        // 使用内置的工具委托
        self.chat(messages).await
    }

    async fn analyze(&self, data: &str) -> anyhow::Result<ChatMessage> {
        let messages = vec![
            ChatMessage::system().with_text("你是一个数据分析专家，请分析提供的数据并提供深入见解。"),
            ChatMessage::user().with_text(format!("请分析以下数据并提供详细分析报告：\n\n{}", data)),
        ];
        self.completion(&messages).await
    }

    async fn summarize(&self, content: &str) -> anyhow::Result<ChatMessage> {
        let messages = vec![
            ChatMessage::system().with_text("你是一个内容总结专家，请简洁准确地总结内容要点。"),
            ChatMessage::user().with_text(format!("请总结以下内容的核心要点：\n\n{}", content)),
        ];
        self.completion(&messages).await
    }

    async fn extract_knowledge(&self, raw_data: &str) -> anyhow::Result<ChatMessage> {
        let messages = vec![
            ChatMessage::system().with_text("你是一个知识提取专家，请从数据中提取关键信息和知识点。"),
            ChatMessage::user().with_text(format!(
                "请从以下数据中提取关键知识点和有价值的信息：\n\n{}",
                raw_data
            )),
        ];
        self.completion(&messages).await
    }
}

/// 便捷的构建器
pub struct LlmAdapterBuilder {
    provider_id: String,
    model_id: String,
    source: String,
    tools: Vec<SelectedTool>,
    tool_timeout: Option<u64>,
}

impl LlmAdapterBuilder {
    pub fn new(provider_id: String, model_id: String) -> Self {
        Self {
            provider_id,
            model_id,
            source: "agentic".to_string(),
            tools: Vec::new(),
            tool_timeout: None,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn with_tools(mut self, tools: Vec<SelectedTool>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_tool_timeout(mut self, timeout_seconds: u64) -> Self {
        self.tool_timeout = Some(timeout_seconds);
        self
    }

    pub fn build(self) -> LlmAdapter {
        let mut chat = LlmAdapter::new(self.provider_id, self.model_id, self.source);

        if !self.tools.is_empty() {
            chat = chat.with_tools(self.tools);
        }

        if let Some(timeout) = self.tool_timeout {
            chat = chat.with_tool_timeout(timeout);
        }

        chat
    }
}

/// 默认的 LLM 实现工厂
pub struct LLMFactory;

impl LLMFactory {
    /// 创建默认的 LLM 实现
    pub fn create_default() -> anyhow::Result<LlmAdapter> {
        // 这里应该从配置中读取默认的 provider 和 model
        // 简化实现，使用硬编码值
        Ok(LlmAdapter::new(
            "openai".to_string(),
            "gpt-4".to_string(),
            "agentic_default".to_string(),
        ))
    }

    /// 从配置创建 LLM 实例
    pub fn create_from_config(
        provider_id: &str,
        model_id: &str,
        source: &str,
        tools: Option<Vec<SelectedTool>>,
    ) -> LlmAdapter {
        let mut builder = LlmAdapterBuilder::new(provider_id.to_string(), model_id.to_string())
            .with_source(source.to_string());

        if let Some(tools) = tools {
            builder = builder.with_tools(tools);
        }

        builder.build()
    }

    /// 创建带工具的 LLM 实例
    pub fn create_with_tools(
        provider_id: &str,
        model_id: &str,
        tools: Vec<SelectedTool>,
    ) -> LlmAdapter {
        LlmAdapterBuilder::new(provider_id.to_string(), model_id.to_string())
            .with_tools(tools)
            .with_tool_timeout(30) // 30秒超时
            .build()
    }
}

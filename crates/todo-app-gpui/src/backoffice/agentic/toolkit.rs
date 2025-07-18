use crate::{
    backoffice::{
        agentic::{Operator, ToolDefinition},
        mcp::{McpRegistry, ToolCallResult},
    },
    config::todo_item::SelectedTool,
};
use serde_json::Value;

/// 基于 MCP 的工具委托实现
#[derive(Debug, Clone)]
pub struct Toolkit {
    /// 可用的工具配置
    pub selected_tools: Vec<SelectedTool>,
    /// 工具调用超时时间（秒）
    pub timeout_seconds: u64,
}

impl Toolkit {
    /// 创建新的 MCP 工具委托
    pub fn new(selected_tools: Vec<SelectedTool>) -> Self {
        Self {
            selected_tools,
            timeout_seconds: 30, // 默认30秒超时
        }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// 从工具名称获取对应的 SelectedTool
    fn find_tool(&self, name: &str) -> Option<&SelectedTool> {
        // 支持两种格式的工具名称：
        // 1. "provider_id@tool_name" 格式
        // 2. 直接的 "tool_name" 格式
        if let Some((provider_id, tool_name)) = name.split_once('@') {
            self.selected_tools
                .iter()
                .find(|tool| tool.provider_id == provider_id && tool.tool_name == tool_name)
        } else {
            // 如果没有 @ 分隔符，就按工具名称查找第一个匹配的
            self.selected_tools
                .iter()
                .find(|tool| tool.tool_name == name)
        }
    }
}

impl Operator for Toolkit {
    type Output = ToolCallResult;
    type Args = String; // JSON 字符串或键值对字符串

    async fn call(&self, name: &str, args: Self::Args) -> anyhow::Result<Self::Output> {
        // 查找对应的工具配置
        let tool = self
            .find_tool(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found in selected tools", name))?;

        // 使用 McpRegistry 调用工具，支持超时
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_seconds),
            McpRegistry::call_tool(&tool.provider_id, &tool.tool_name, &args),
        )
        .await??;
        Ok(result)
    }

    async fn available_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();
        for tool in &self.selected_tools {
            if let Ok(Some(snapshot)) = McpRegistry::get_snapshot(&tool.provider_id).await {
                tools.extend(
                    snapshot
                        .tools
                        .into_iter()
                        .filter(|t| t.name == tool.tool_name)
                        .map(|t| ToolDefinition {
                            name: ToolDefinition::format_tool_name(
                                &tool.provider_id,
                                &tool.tool_name,
                            ),
                            description: t.description.unwrap_or_default().to_string(),
                            parameters: Value::Object(t.input_schema.as_ref().clone()).to_string(),
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }

        tools
    }
}

/// 批量工具调用委托 - 支持同时调用多个工具
#[derive(Debug)]
pub struct BatchMcpToolDelegate {
    inner: Toolkit,
    max_concurrent_calls: usize,
}

impl BatchMcpToolDelegate {
    pub fn new(selected_tools: Vec<SelectedTool>) -> Self {
        Self {
            inner: Toolkit::new(selected_tools),
            max_concurrent_calls: 5,
        }
    }

    pub fn with_max_concurrent(mut self, max_concurrent_calls: usize) -> Self {
        self.max_concurrent_calls = max_concurrent_calls;
        self
    }

    /// 批量调用多个工具
    pub async fn batch_call(
        &self,
        calls: Vec<(String, String)>, // (tool_name, args) pairs
    ) -> anyhow::Result<Vec<ToolCallResult>> {
        use futures::stream::{self, StreamExt};

        let results = stream::iter(calls)
            .map(|(name, args)| async move { self.inner.call(&name, args).await })
            .buffer_unordered(self.max_concurrent_calls)
            .collect::<Vec<_>>()
            .await;

        results.into_iter().collect()
    }
}

impl Operator for BatchMcpToolDelegate {
    type Output = ToolCallResult;
    type Args = String;

    async fn call(&self, name: &str, args: Self::Args) -> anyhow::Result<Self::Output> {
        self.inner.call(name, args).await
    }

    async fn available_tools(&self) -> Vec<ToolDefinition> {
        self.inner.available_tools().await
    }
}

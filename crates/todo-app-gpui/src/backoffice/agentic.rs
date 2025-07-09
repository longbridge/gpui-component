use super::llm::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
mod insight;
mod knowledge;
// pub(crate) mod llm;
pub(crate) mod mcp_tools;
mod memex;
pub(crate) mod prompts;
// mod rig_llm;

use crate::backoffice::mcp::McpCallToolResult;

/// 记忆类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
}

/// 记忆条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub memory_type: MemoryType,
    pub timestamp: Option<u64>,
}

/// 记忆体定义，为LLM提供短期和长期记忆存储和检索功能。
pub trait Memory: Send + Sync {
    /// 存储记忆，带有类型标识
    async fn store(
        &mut self,
        key: &str,
        value: &str,
        memory_type: MemoryType,
    ) -> anyhow::Result<()>;

    /// 获取记忆
    async fn get(&self, key: &str, memory_type: MemoryType) -> anyhow::Result<Option<String>>;

    /// 清空指定类型的记忆
    async fn clear(&mut self, memory_type: MemoryType) -> anyhow::Result<()>;

    /// 搜索相关记忆，可以指定搜索范围
    async fn search(
        &self,
        query: &str,
        memory_type: Option<MemoryType>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// 列出所有记忆键
    async fn list_keys(&self, memory_type: MemoryType) -> anyhow::Result<Vec<String>>;

    // 为了向后兼容，保留原有方法
    async fn store_short_term(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        self.store(key, value, MemoryType::ShortTerm).await
    }

    async fn get_short_term(&self, key: &str) -> anyhow::Result<Option<String>> {
        self.get(key, MemoryType::ShortTerm).await
    }

    async fn store_long_term(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        self.store(key, value, MemoryType::LongTerm).await
    }

    async fn get_long_term(&self, key: &str) -> anyhow::Result<Option<String>> {
        self.get(key, MemoryType::LongTerm).await
    }

    async fn clear_short_term(&mut self) -> anyhow::Result<()> {
        self.clear(MemoryType::ShortTerm).await
    }

    async fn search_memory(&self, query: &str) -> anyhow::Result<Vec<(String, String)>> {
        let entries = self.search(query, None).await?;
        Ok(entries.into_iter().map(|e| (e.key, e.value)).collect())
    }
}

/// LLM特性，定义了LLM的基本交互方法，包含了洞察和知识处理能力。
pub trait LLM: Send + Sync {
    type ToolDelegate: ToolDelegate<Output = McpCallToolResult, Args = String>;
    /// 基础对话能力 - 流式响应
    async fn completion_stream(&self, prompts: &[ChatMessage]) -> anyhow::Result<ChatStream>;

    /// 带工具调用的对话 - 流式响应
    async fn completion_with_tools_stream(
        &self,
        prompts: &[ChatMessage],
        tools: &Self::ToolDelegate,
    ) -> anyhow::Result<ChatStream>;

    /// 对话接口 - 流式响应
    async fn chat_stream(&self, messages: &[ChatMessage]) -> anyhow::Result<ChatStream>;

    /// 带工具调用的对话 - 流式响应
    async fn chat_with_tools_stream(
        &self,
        messages: &[ChatMessage],
        tools: &Self::ToolDelegate,
    ) -> anyhow::Result<ChatStream>;

    /// 基础对话能力 - 一次性响应（便捷方法）
    async fn completion(&self, prompts: &[ChatMessage]) -> anyhow::Result<ChatMessage> {
        let mut stream = self.completion_stream(prompts).await?;
        let mut final_message = ChatMessage::assistant_text("");

        // 收集流式响应并合并
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(message) => {
                    let text = message.get_text();
                    // 修复：正确处理 MessageContent 枚举
                    // match &mut final_message.content {
                    //     MessageContent::Parts(parts) => {
                    //         if let Some(MediaContent {
                    //             data: MediaData::Text(existing_text),
                    //             ..
                    //         }) = parts.get_mut(0)
                    //         {
                    //             existing_text.push_str(&text);
                    //         }
                    //     }
                    //     MessageContent::Part(part) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::Chunk(_) => {
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::ToolCall(_) => {
                    //         // // 如果是工具调用，创建新的文本内容
                    //         // final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::ToolDefinitions(_) => {
                    //         // // 如果是工具调用，创建新的文本内容
                    //         // final_message.content = MessageContent::text(text);
                    //     }
                    // }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(final_message)
    }

    /// 对话接口 - 一次性响应（便捷方法）
    async fn chat(&self, messages: &[ChatMessage]) -> anyhow::Result<ChatMessage> {
        let mut stream = self.chat_stream(messages).await?;
        let mut final_message = ChatMessage::assistant_text("");

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(message) => {
                    let text = message.get_text();
                    // 修复：正确处理 MessageContent 枚举
                    // match &mut final_message.content {
                    //     MessageContent::Parts(parts) => {
                    //         if let Some(MediaContent {
                    //             data: MediaData::Text(existing_text),
                    //             ..
                    //         }) = parts.get_mut(0)
                    //         {
                    //             existing_text.push_str(&text);
                    //         }
                    //     }
                    //     MessageContent::Chunk(_) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::ToolCall(_) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::ToolDefinitions(_) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::Part(part) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    // }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(final_message)
    }

    /// 带工具调用的对话 - 一次性响应（便捷方法）
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &Self::ToolDelegate,
    ) -> anyhow::Result<ChatMessage> {
        let mut stream = self.chat_with_tools_stream(messages, tools).await?;
        let mut final_message = ChatMessage::assistant_text("");

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(message) => {
                    let text = message.get_text();
                    // 修复：正确处理 MessageContent 枚举
                    // match &mut final_message.content {
                    //     MessageContent::Parts(parts) => {
                    //         if let Some(MediaContent {
                    //             data: MediaData::Text(existing_text),
                    //             ..
                    //         }) = parts.get_mut(0)
                    //         {
                    //             existing_text.push_str(&text);
                    //         }
                    //     }
                    //     MessageContent::Chunk(_) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::ToolCall(_) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::ToolDefinitions(_) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    //     MessageContent::Part(part) => {
                    //         // 如果是工具调用，创建新的文本内容
                    //         final_message.content = MessageContent::text(text);
                    //     }
                    // }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(final_message)
    }

    /// 数据分析和洞察能力 - 通常不需要流式，直接返回结果
    async fn analyze(&self, data: &str) -> anyhow::Result<ChatMessage> {
        let messages = vec![
            ChatMessage::system_text("你是一个数据分析专家，请分析提供的数据。"),
            ChatMessage::user_text(format!("请分析以下数据：\n{}", data)),
        ];
        self.completion(&messages).await
    }
    async fn summarize(&self, content: &str) -> anyhow::Result<ChatMessage> {
        let messages = vec![
            ChatMessage::system_text("你是一个内容总结专家，请简洁地总结内容要点。"),
            ChatMessage::user_text(format!("请总结以下内容：\n{}", content)),
        ];
        self.completion(&messages).await
    }
    /// 知识处理能力 - 通常不需要流式，直接返回结果  
    async fn extract_knowledge(&self, raw_data: &str) -> anyhow::Result<ChatMessage> {
        let messages = vec![
            ChatMessage::system_text("你是一个知识提取专家，请提取关键信息和知识点。"),
            ChatMessage::user_text(format!("请从以下数据中提取关键知识点：\n{}", raw_data)),
        ];
        self.completion(&messages).await
    }
}

/// 学习配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// 是否自动学习用户交互
    pub auto_learn_interactions: bool,
    /// 学习触发的最小信息长度
    pub min_info_length: usize,
    /// 知识存储的最大数量
    pub max_knowledge_entries: usize,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            auto_learn_interactions: true,
            min_info_length: 50,
            max_knowledge_entries: 1000,
        }
    }
}

/// 反思配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    /// 反思的时间间隔（秒）
    pub reflection_interval: u64,
    /// 分析的最大交互数量
    pub max_interactions_to_analyze: usize,
    /// 是否包含长期知识背景
    pub include_long_term_context: bool,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            reflection_interval: 3600, // 1小时
            max_interactions_to_analyze: 20,
            include_long_term_context: true,
        }
    }
}

/// 学习统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_learned_items: usize,
    pub recent_learning_rate: f32,
    pub last_reflection_time: Option<u64>,
    pub knowledge_categories: Vec<String>,
}

/// 任务状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

/// 任务步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub step_id: String,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub timestamp: u64,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub task_id: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub created_at: u64,
    pub updated_at: u64,
    pub metadata: HashMap<String, String>,
    pub steps: Vec<TaskStep>,
    pub current_step: usize,
}

/// 执行上下文 - 单次对话或任务的临时状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// 会话ID
    pub session_id: String,
    /// 当前对话历史
    pub conversation_history: Vec<ChatMessage>,
    /// 当前任务状态
    pub current_task: Option<TaskState>,
    /// 临时变量
    pub variables: HashMap<String, String>,
    /// 上下文创建时间
    pub created_at: u64,
    /// 最后更新时间
    pub last_updated: u64,
}

/// Agent行为模式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BehaviorMode {
    /// 保守模式：更依赖已有知识，减少创新
    Conservative,
    /// 探索模式：更愿意学习新信息和尝试新方法
    Exploratory,
    /// 平衡模式：在稳定性和创新性之间平衡
    Balanced,
    /// 专家模式：在特定领域深度工作
    Expert { domain: String },
}

/// 记忆管理策略
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryStrategy {
    /// 积极记忆：记录所有交互
    Aggressive,
    /// 选择性记忆：只记录重要信息
    Selective { importance_threshold: f32 },
    /// 最小记忆：只记录核心信息
    Minimal,
    /// 智能记忆：根据上下文动态调整
    Adaptive,
}

/// 工具使用策略
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolUsageStrategy {
    /// 优先使用工具
    ToolFirst,
    /// 优先使用LLM能力
    LLMFirst,
    /// 自适应选择
    Adaptive,
    /// 禁用工具
    Disabled,
}

/// Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 学习配置
    pub learning_config: LearningConfig,
    /// 反思配置
    pub reflection_config: ReflectionConfig,
    /// Agent的行为模式
    pub behavior_mode: BehaviorMode,
    /// 记忆管理策略
    pub memory_strategy: MemoryStrategy,
    /// 上下文窗口大小
    pub context_window_size: usize,
    /// 工具使用策略
    pub tool_usage_strategy: ToolUsageStrategy,
}

/// 反思触发原因
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReflectionTrigger {
    /// 定期反思
    Periodic,
    /// 错误触发
    Error { error_type: String },
    /// 手动触发
    Manual,
    /// 任务完成后
    TaskCompletion { task_id: String },
    /// 学习阈值达到
    LearningThreshold,
}

/// 反思记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionEntry {
    pub timestamp: u64,
    pub trigger: ReflectionTrigger,
    pub content: String,
    pub insights: Vec<String>,
    pub action_items: Vec<String>,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 成功率
    pub success_rate: f32,
    /// 记忆检索效率
    pub memory_retrieval_efficiency: f32,
    /// 学习效率
    pub learning_efficiency: f32,
    /// 工具使用成功率
    pub tool_success_rate: f32,
}

/// 全局状态
#[derive(Debug)]
pub struct GlobalState {
    /// Agent启动时间
    pub startup_time: u64,
    /// 总交互次数
    pub total_interactions: usize,
    /// 活跃会话列表
    pub active_sessions: HashSet<String>,
    /// 全局统计
    pub stats: LearningStats,
    /// 错误统计
    pub error_count: usize,
    /// 最后活跃时间
    pub last_active_time: u64,
}

/// 运行时上下文 - Agent的持久化状态和能力
pub struct RuntimeContext<M: Memory, L: LLM> {
    /// 记忆系统（持久化）
    pub memory: M,
    /// LLM接口（能力）
    pub llm: L,
    /// Agent配置
    pub config: AgentConfig,
    /// 全局状态
    pub global_state: GlobalState,
    /// 反思历史
    pub reflection_history: Vec<ReflectionEntry>,
    /// 性能指标
    pub performance_metrics: PerformanceMetrics,
}

impl ExecutionContext {
    pub fn new(session_id: String) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            session_id,
            conversation_history: Vec::new(),
            current_task: None,
            variables: HashMap::new(),
            created_at: now,
            last_updated: now,
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.conversation_history.push(message);
        self.last_updated = chrono::Utc::now().timestamp() as u64;

        // 限制历史长度
        if self.conversation_history.len() > 50 {
            self.conversation_history.drain(0..10);
        }
    }

    pub fn get_recent_messages(&self, count: usize) -> &[ChatMessage] {
        let start = self.conversation_history.len().saturating_sub(count);
        &self.conversation_history[start..]
    }

    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
        self.last_updated = chrono::Utc::now().timestamp() as u64;
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn set_task(&mut self, task: TaskState) {
        self.current_task = Some(task);
        self.last_updated = chrono::Utc::now().timestamp() as u64;
    }

    pub fn update_task_progress(&mut self, progress: f32) {
        if let Some(task) = &mut self.current_task {
            task.progress = progress;
            if progress >= 1.0 {
                task.status = TaskStatus::Completed;
            }
            task.updated_at = chrono::Utc::now().timestamp() as u64;
        }
        self.last_updated = chrono::Utc::now().timestamp() as u64;
    }

    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        now - self.last_updated > timeout_seconds
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            learning_config: LearningConfig::default(),
            reflection_config: ReflectionConfig::default(),
            behavior_mode: BehaviorMode::Balanced,
            memory_strategy: MemoryStrategy::Selective {
                importance_threshold: 0.6,
            },
            context_window_size: 4096,
            tool_usage_strategy: ToolUsageStrategy::Adaptive,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 1000.0,
            success_rate: 1.0,
            memory_retrieval_efficiency: 0.8,
            learning_efficiency: 0.7,
            tool_success_rate: 0.9,
        }
    }
}

impl<M: Memory, L: LLM> RuntimeContext<M, L> {
    pub fn new(memory: M, llm: L) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;

        Self {
            memory,
            llm,
            config: AgentConfig::default(),
            global_state: GlobalState {
                startup_time: now,
                total_interactions: 0,
                active_sessions: HashSet::new(),
                stats: LearningStats {
                    total_learned_items: 0,
                    recent_learning_rate: 0.0,
                    last_reflection_time: None,
                    knowledge_categories: Vec::new(),
                },
                error_count: 0,
                last_active_time: now,
            },
            reflection_history: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
        }
    }

    pub fn update_active_time(&mut self) {
        self.global_state.last_active_time = chrono::Utc::now().timestamp() as u64;
    }

    pub fn increment_interactions(&mut self) {
        self.global_state.total_interactions += 1;
        self.update_active_time();
    }

    pub fn record_error(&mut self, error_type: &str) {
        self.global_state.error_count += 1;

        // 如果错误过多，触发反思
        if self.global_state.error_count % 10 == 0 {
            let reflection_entry = ReflectionEntry {
                timestamp: chrono::Utc::now().timestamp() as u64,
                trigger: ReflectionTrigger::Error {
                    error_type: error_type.to_string(),
                },
                content: format!("检测到错误模式，错误类型: {}", error_type),
                insights: vec![],
                action_items: vec!["调整策略以减少错误".to_string()],
            };
            self.reflection_history.push(reflection_entry);
        }
    }

    pub fn add_reflection(&mut self, reflection: ReflectionEntry) {
        // 保存时间戳，避免移动后访问
        let timestamp = reflection.timestamp;

        self.reflection_history.push(reflection);

        // 保持反思历史在合理大小
        if self.reflection_history.len() > 100 {
            self.reflection_history.drain(0..10);
        }

        // 更新最后反思时间
        self.global_state.stats.last_reflection_time = Some(timestamp);
    }

    pub fn update_performance_metrics(&mut self, response_time_ms: f64, success: bool) {
        let metrics = &mut self.performance_metrics;

        // 更新平均响应时间（简单移动平均）
        metrics.avg_response_time_ms =
            (metrics.avg_response_time_ms * 0.9) + (response_time_ms * 0.1);

        // 更新成功率（简单移动平均）
        let success_value = if success { 1.0 } else { 0.0 };
        metrics.success_rate = (metrics.success_rate * 0.9) + (success_value * 0.1);
    }

    pub fn uptime_seconds(&self) -> u64 {
        chrono::Utc::now().timestamp() as u64 - self.global_state.startup_time
    }

    pub fn needs_maintenance(&self) -> bool {
        self.global_state.error_count > 100
            || self.performance_metrics.success_rate < 0.8
            || self.performance_metrics.avg_response_time_ms > 5000.0
    }

    pub async fn cleanup_expired_data(&mut self) -> anyhow::Result<()> {
        let now = chrono::Utc::now().timestamp() as u64;
        let cleanup_threshold = 24 * 3600; // 24小时

        // 清理过期的短期记忆
        self.memory.clear(MemoryType::ShortTerm).await?;

        // 清理过期的反思记录
        self.reflection_history.retain(|entry| {
            now - entry.timestamp < cleanup_threshold * 7 // 保留7天的反思
        });

        Ok(())
    }
}

/// 工具调用的委托接口，允许不同的工具实现自己的调用逻辑。
pub trait ToolDelegate: Send + Sync {
    type Output;
    type Args;

    /// 调用指定工具
    async fn call(&self, name: &str, args: Self::Args) -> anyhow::Result<Self::Output>;

    /// 获取可用工具列表
    async fn available_tools(&self) -> Vec<ToolDefinition>;
}

/// 默认的工具委托实现，什么都不做。
impl ToolDelegate for () {
    type Output = ();
    type Args = ();

    async fn call(&self, _name: &str, _args: Self::Args) -> anyhow::Result<Self::Output> {
        Ok(())
    }

    async fn available_tools(&self) -> Vec<ToolDefinition> {
        vec![]
    }
}

//TODO:实现基于MCP的ToolDelegate实现

/// 基础智能体特性
pub trait Agent {
    type Memory: Memory;
    type LLM: LLM;
    // type Tools: ToolDelegate;

    /// 获取组件实例
    fn memory(&self) -> &Self::Memory;
    fn memory_mut(&mut self) -> &mut Self::Memory;
    fn llm(&self) -> &Self::LLM;

    /// 处理用户输入并生成响应
    async fn process_input(&mut self, input: &str) -> anyhow::Result<String> {
        // 存储输入到短期记忆
        let input_key = format!("input_{}", chrono::Utc::now().timestamp());
        Agent::memory_mut(self)
            .store(&input_key, input, MemoryType::ShortTerm)
            .await?;

        // 处理输入
        self.process_with_context(input).await
    }

    /// 带上下文的处理（需要具体实现）
    async fn process_with_context(&mut self, input: &str) -> anyhow::Result<String>;

    /// 执行复杂任务的工作流
    async fn execute_task(&mut self, task: &str) -> anyhow::Result<String>;

    /// 学习新信息并更新知识库
    async fn learn(&mut self, information: &str) -> anyhow::Result<()> {
        // 提取知识点
        let knowledge = self.llm().extract_knowledge(information).await?;

        // 生成摘要
        let summary = self.llm().summarize(information).await?;

        // 存储到长期记忆
        let key = format!("knowledge_{}", chrono::Utc::now().timestamp());
        let stored_content = format!("Summary: {:?}\nKnowledge: {:?}", summary, knowledge);

        Agent::memory_mut(self)
            .store(&key, &stored_content, MemoryType::LongTerm)
            .await?;

        Ok(())
    }

    /// 反思和总结当前状态
    async fn reflect(&self) -> anyhow::Result<String> {
        // 搜索最近的交互记录
        let recent_interactions = self
            .memory()
            .search("input", Some(MemoryType::ShortTerm))
            .await?;

        // 获取长期知识作为上下文
        let knowledge_context = self
            .memory()
            .search("knowledge", Some(MemoryType::LongTerm))
            .await?;

        // 构建反思提示
        let reflection_prompt = format!(
            "基于以下信息进行反思和总结：\n\n最近交互（{}条）：\n{}\n\n知识背景（{}条）：\n{}",
            recent_interactions.len(),
            recent_interactions
                .iter()
                .take(10)
                .map(|e| format!("{}: {}", e.key, e.value))
                .collect::<Vec<_>>()
                .join("\n"),
            knowledge_context.len(),
            knowledge_context
                .iter()
                .take(5)
                .map(|e| format!("{}: {}", e.key, e.value))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // 使用LLM进行反思分析
        self.llm()
            .analyze(&reflection_prompt)
            .await
            .map(|result| format!("{:?}", result))
    }
}

/// 高级智能体特性 - 合并原来的 AdvancedAgent 和 ContextualAgent
pub trait AdvancedAgent: Agent {
    /// 获取运行时上下文
    fn runtime_context(&self) -> &RuntimeContext<Self::Memory, Self::LLM>;
    fn runtime_context_mut(&mut self) -> &mut RuntimeContext<Self::Memory, Self::LLM>;

    /// 获取执行上下文
    fn execution_context(&self) -> &ExecutionContext;
    fn execution_context_mut(&mut self) -> &mut ExecutionContext;

    /// 重写基础方法以使用运行时上下文
    fn memory(&self) -> &Self::Memory {
        &self.runtime_context().memory
    }

    fn memory_mut(&mut self) -> &mut Self::Memory {
        &mut self.runtime_context_mut().memory
    }

    fn llm(&self) -> &Self::LLM {
        &self.runtime_context().llm
    }

    /// 获取配置
    fn learning_config(&self) -> &LearningConfig {
        &self.runtime_context().config.learning_config
    }

    fn reflection_config(&self) -> &ReflectionConfig {
        &self.runtime_context().config.reflection_config
    }

    /// 带完整上下文的处理 - 核心方法
    async fn process_with_full_context(&mut self, input: &str) -> anyhow::Result<String> {
        let start_time = std::time::Instant::now();
        self.runtime_context_mut().increment_interactions();

        // 更新执行上下文
        self.execution_context_mut()
            .add_message(ChatMessage::text(MessageRole::User, input));

        // 存储到记忆
        let input_key = format!(
            "input_{}_{}",
            self.execution_context().session_id,
            chrono::Utc::now().timestamp()
        );
        Agent::memory_mut(self)
            .store(&input_key, input, MemoryType::ShortTerm)
            .await?;

        // 根据行为模式选择处理策略
        let behavior_mode = self.runtime_context().config.behavior_mode.clone();
        let result = match behavior_mode {
            BehaviorMode::Conservative => self.process_conservatively(input).await,
            BehaviorMode::Exploratory => self.process_exploratively(input).await,
            BehaviorMode::Balanced => self.process_balanced(input).await,
            BehaviorMode::Expert { domain } => self.process_as_expert(input, &domain).await,
        };

        // 更新性能指标
        let elapsed = start_time.elapsed().as_millis() as f64;
        let success = result.is_ok();
        self.runtime_context_mut()
            .update_performance_metrics(elapsed, success);

        // 处理结果
        if let Err(ref e) = result {
            self.runtime_context_mut().record_error(&format!("{:?}", e));
        } else if let Ok(ref response) = result {
            // 添加响应到上下文
            self.execution_context_mut()
                .add_message(ChatMessage::text(MessageRole::Assistant, response));

            // 智能学习
            let importance = self
                .calculate_importance(input, response)
                .await
                .unwrap_or(0.5);
            let _ = self.smart_learn(input, importance).await;
        }

        // 定期反思
        if let Ok(Some(reflection)) = self.periodic_reflect().await {
            let reflection_entry = ReflectionEntry {
                timestamp: chrono::Utc::now().timestamp() as u64,
                trigger: ReflectionTrigger::Periodic,
                content: reflection,
                insights: vec![],
                action_items: vec![],
            };
            self.runtime_context_mut().add_reflection(reflection_entry);
        }

        result
    }

    /// 智能学习：根据重要性和相关性决定是否学习
    async fn smart_learn(&mut self, information: &str, importance: f32) -> anyhow::Result<bool> {
        let config = self.learning_config();

        // 检查信息长度
        if information.len() < config.min_info_length {
            return Ok(false);
        }

        // 检查重要性阈值
        if importance < 0.5 {
            return Ok(false);
        }

        // 执行学习
        Agent::learn(self, information).await?;
        Ok(true)
    }

    /// 定期反思：基于配置自动触发反思
    async fn periodic_reflect(&mut self) -> anyhow::Result<Option<String>> {
        let config = self.reflection_config();
        let now = chrono::Utc::now().timestamp() as u64;

        // 检查是否需要反思
        if let Ok(stats) = self.learning_stats().await {
            if let Some(last_time) = stats.last_reflection_time {
                if now - last_time < config.reflection_interval {
                    return Ok(None);
                }
            }
        }

        // 执行反思
        let reflection = self.reflect().await?;
        Ok(Some(reflection))
    }

    /// 获取学习统计
    async fn learning_stats(&self) -> anyhow::Result<LearningStats> {
        let knowledge_entries = Agent::memory(self)
            .search("knowledge", Some(MemoryType::LongTerm))
            .await?;

        Ok(LearningStats {
            total_learned_items: knowledge_entries.len(),
            recent_learning_rate: 0.0,
            last_reflection_time: self
                .runtime_context()
                .global_state
                .stats
                .last_reflection_time,
            knowledge_categories: vec!["general".to_string()],
        })
    }

    /// 不同行为模式的处理方法
    async fn process_conservatively(&mut self, input: &str) -> anyhow::Result<String> {
        let relevant_memories = Agent::memory(self)
            .search(input, Some(MemoryType::LongTerm))
            .await?;

        let context_prompt = if !relevant_memories.is_empty() {
            format!(
                "基于已有知识谨慎回答：{}\n\n相关知识：\n{}",
                input,
                relevant_memories
                    .iter()
                    .take(3)
                    .map(|m| format!("- {}", m.value))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            format!("请谨慎回答：{}", input)
        };

        let result = Agent::llm(self)
            .completion(&[ChatMessage::user_text(context_prompt)])
            .await?;
        Ok(format!("{:?}", result))
    }

    async fn process_exploratively(&mut self, input: &str) -> anyhow::Result<String> {
        let exploratory_prompt = format!("请创新性地分析和回答：{}", input);
        let result = Agent::llm(self)
            .completion(&[ChatMessage::user_text(exploratory_prompt)])
            .await?;

        // 自动学习新信息
        if input.len() > self.learning_config().min_info_length {
            let _ = Agent::learn(self, input).await;
        }

        Ok(format!("{:?}", result))
    }

    async fn process_balanced(&mut self, input: &str) -> anyhow::Result<String> {
        let memories = Agent::memory(self).search(input, None).await?;
        let context_aware_prompt = if memories.len() > 2 {
            format!(
                "基于相关知识回答：{}\n\n背景：{}",
                input,
                memories
                    .iter()
                    .take(2)
                    .map(|m| m.value.clone())
                    .collect::<Vec<String>>()
                    .join("; ")
            )
        } else {
            input.to_string()
        };

        let result = Agent::llm(self)
            .completion(&[ChatMessage::user_text(context_aware_prompt)])
            .await?;

        // 选择性学习
        if input.len() > 50 && memories.len() < 3 {
            let importance = self
                .calculate_importance(input, &format!("{:?}", result))
                .await
                .unwrap_or(0.6);
            let _ = self.smart_learn(input, importance).await;
        }

        Ok(format!("{:?}", result))
    }

    async fn process_as_expert(&mut self, input: &str, domain: &str) -> anyhow::Result<String> {
        let expert_knowledge = Agent::memory(self)
            .search(&format!("{} {}", domain, input), Some(MemoryType::LongTerm))
            .await?;

        let expert_prompt = format!(
            "作为{}领域的专家，基于专业知识深度分析：{}\n\n专业背景：\n{}",
            domain,
            input,
            expert_knowledge
                .iter()
                .take(5)
                .map(|m| format!("- {}", m.value))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let result = Agent::llm(self)
            .completion(&[ChatMessage::user_text(expert_prompt)])
            .await?;

        // 专家模式下的高质量学习
        let _ = self.smart_learn(input, 0.8).await;

        Ok(format!("{:?}", result))
    }

    /// 计算信息重要性
    async fn calculate_importance(&self, input: &str, response: &str) -> anyhow::Result<f32> {
        let mut importance = 0.0;

        // 长度因素
        if input.len() > 100 {
            importance += 0.2;
        }
        if response.len() > 200 {
            importance += 0.2;
        }

        // 复杂性因素
        let complex_keywords = ["分析", "解决", "方案", "策略", "深入", "详细"];
        let keyword_count = complex_keywords
            .iter()
            .filter(|&word| input.contains(word) || response.contains(word))
            .count();
        importance += (keyword_count as f32 * 0.1).min(0.3);

        // 新颖性因素
        let similar_memories = Agent::memory(self).search(input, None).await?;
        if similar_memories.len() < 2 {
            importance += 0.3;
        }

        Ok(importance.min(1.0))
    }

    /// 开始新会话
    fn start_session(&mut self, session_id: String) -> &mut ExecutionContext {
        let context = ExecutionContext::new(session_id.clone());
        self.runtime_context_mut()
            .global_state
            .active_sessions
            .insert(session_id);
        *self.execution_context_mut() = context;
        self.execution_context_mut()
    }

    /// 系统维护
    async fn perform_maintenance(&mut self) -> anyhow::Result<()> {
        // 清理过期数据
        self.runtime_context_mut().cleanup_expired_data().await?;

        // 强制反思
        let reflection = self.reflect().await?;
        let reflection_entry = ReflectionEntry {
            timestamp: chrono::Utc::now().timestamp() as u64,
            trigger: ReflectionTrigger::Manual,
            content: format!("系统维护反思: {}", reflection),
            insights: vec!["定期维护完成".to_string()],
            action_items: vec!["继续监控性能".to_string()],
        };
        self.runtime_context_mut().add_reflection(reflection_entry);

        Ok(())
    }

    /// 定时触发的方法 - 智能体的"心跳" (移除 pub 关键字)
    async fn tick(&mut self) -> anyhow::Result<()> {
        // 简单的自主行为决策
        self.autonomous_behavior().await
    }

    /// 自主行为 - 智能体自己决定要做什么
    async fn autonomous_behavior(&mut self) -> anyhow::Result<()> {
        // 1. 检查是否需要反思
        if self.should_reflect().await? {
            let _ = self.reflect().await;
        }

        // 2. 检查是否需要清理
        if self.should_cleanup().await? {
            let _ = self.perform_maintenance().await;
        }

        // 3. 检查是否有自主思考的机会
        if self.should_think().await? {
            let _ = self.autonomous_thinking().await;
        }

        // 4. 更新内部状态
        self.runtime_context_mut().update_active_time();

        Ok(())
    }

    /// 判断是否应该反思
    async fn should_reflect(&self) -> anyhow::Result<bool> {
        let config = &self.runtime_context().config.reflection_config;
        let now = chrono::Utc::now().timestamp() as u64;

        if let Some(last_time) = self
            .runtime_context()
            .global_state
            .stats
            .last_reflection_time
        {
            Ok(now - last_time > config.reflection_interval)
        } else {
            Ok(true) // 首次反思
        }
    }

    /// 判断是否应该清理
    async fn should_cleanup(&self) -> anyhow::Result<bool> {
        Ok(self.runtime_context().needs_maintenance())
    }

    /// 判断是否应该自主思考
    async fn should_think(&self) -> anyhow::Result<bool> {
        // 简单策略：每10次心跳思考一次
        Ok(self.runtime_context().global_state.total_interactions % 10 == 0)
    }

    /// 自主思考
    async fn autonomous_thinking(&mut self) -> anyhow::Result<()> {
        let thinking_prompts = [
            "我最近学到了什么？",
            "有什么值得改进的地方？",
            "当前状态如何？",
            "有什么新的想法吗？",
        ];

        let total_interactions = self.runtime_context().global_state.total_interactions;
        let prompt = thinking_prompts[total_interactions % thinking_prompts.len()];

        // 进行自主思考（使用明确的方法调用避免歧义）
        let _ = Agent::process_input(self, prompt).await;

        Ok(())
    }

    /// 重写基础方法以使用完整上下文
    async fn process_input(&mut self, input: &str) -> anyhow::Result<String> {
        self.process_with_full_context(input).await
    }

    async fn process_with_context(&mut self, input: &str) -> anyhow::Result<String> {
        self.process_with_full_context(input).await
    }
}

/// AI智能体实现 - 包含完整的高级功能
pub struct AiAgent<M: Memory, L: LLM> {
    // /// 运行时上下文
    // runtime_context: RuntimeContext<M, L>,
    // /// 执行上下文
    // execution_context: ExecutionContext,
    context: ContextAware<M, L>,
    /// 活跃任务
    active_tasks: HashMap<String, TaskState>,
}

impl<M: Memory, L: LLM> AiAgent<M, L> {
    pub fn new(session_id: String, memory: M, llm: L) -> Self {
        Self {
            context: ContextAware {
                runtime_context: RuntimeContext::new(memory, llm),
                execution_context: ExecutionContext::new(session_id),
            },
            active_tasks: HashMap::new(),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &AgentConfig {
        &self.context.runtime_context.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: AgentConfig) {
        self.context.runtime_context.config = config;
    }

    /// 获取学习统计
    pub async fn get_learning_stats(&self) -> anyhow::Result<LearningStats> {
        self.learning_stats().await
    }

    /// 获取任务状态
    pub fn get_task_status(&self, task_id: &str) -> Option<&TaskState> {
        self.active_tasks.get(task_id)
    }

    /// 获取执行上下文
    pub fn get_execution_context(&self) -> &ExecutionContext {
        &self.context.execution_context
    }

    /// 获取性能指标
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.context.runtime_context.performance_metrics
    }

    /// 创建新任务
    pub fn create_task(&mut self, task_type: &str, description: &str) -> String {
        let task_id = format!("task_{}", chrono::Utc::now().timestamp());
        let task_state = TaskState {
            task_id: task_id.clone(),
            task_type: task_type.to_string(),
            status: TaskStatus::Pending,
            progress: 0.0,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("description".to_string(), description.to_string());
                meta
            },
            steps: vec![],
            current_step: 0,
        };
        self.active_tasks.insert(task_id.clone(), task_state);
        task_id
    }

    /// 更新任务状态
    pub fn update_task_status(&mut self, task_id: &str, status: TaskStatus) {
        if let Some(task) = self.active_tasks.get_mut(task_id) {
            task.status = status;
            task.updated_at = chrono::Utc::now().timestamp() as u64;
        }
    }

    /// 外部定时器调用的 tick 方法 - 智能体的"心跳"
    pub async fn tick(&mut self) -> anyhow::Result<()> {
        // 调用高级智能体的 tick 实现
        AdvancedAgent::tick(self).await
    }
}

impl<M: Memory, L: LLM> Agent for AiAgent<M, L> {
    type Memory = M;
    type LLM = L;

    fn memory(&self) -> &Self::Memory {
        &self.context.runtime_context.memory
    }

    fn memory_mut(&mut self) -> &mut Self::Memory {
        &mut self.context.runtime_context.memory
    }

    fn llm(&self) -> &Self::LLM {
        &self.context.runtime_context.llm
    }

    async fn process_with_context(&mut self, input: &str) -> anyhow::Result<String> {
        self.process_with_full_context(input).await
    }

    async fn execute_task(&mut self, task: &str) -> anyhow::Result<String> {
        self.process_with_full_context(&format!("执行任务: {}", task))
            .await
    }
}

impl<M: Memory, L: LLM> AdvancedAgent for AiAgent<M, L> {
    fn runtime_context(&self) -> &RuntimeContext<Self::Memory, Self::LLM> {
        &self.context.runtime_context
    }

    fn runtime_context_mut(&mut self) -> &mut RuntimeContext<Self::Memory, Self::LLM> {
        &mut self.context.runtime_context
    }

    fn execution_context(&self) -> &ExecutionContext {
        &self.context.execution_context
    }

    fn execution_context_mut(&mut self) -> &mut ExecutionContext {
        &mut self.context.execution_context
    }
}

/// 媒体处理工具 trait
pub trait MediaProcessor: Send + Sync {
    /// 处理上传的文件
    async fn process_upload(&self, file_path: &str) -> anyhow::Result<MediaContent>;

    /// 从 URL 获取媒体内容
    async fn fetch_from_url(&self, url: &str) -> anyhow::Result<MediaContent>;

    /// 压缩媒体内容
    async fn compress_media(&self, media: &MediaContent) -> anyhow::Result<MediaContent>;

    /// 提取文本内容（OCR、语音转文字等）
    async fn extract_text(&self, media: &MediaContent) -> anyhow::Result<Option<String>>;
}

pub struct ContextAware<M: Memory, L: LLM> {
    runtime_context: RuntimeContext<M, L>,
    /// 执行上下文
    execution_context: ExecutionContext,
}

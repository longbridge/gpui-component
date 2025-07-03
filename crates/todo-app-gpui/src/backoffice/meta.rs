/// 这个模型是为了提供一个通用的接口，用于处理记忆、工具调用和LLM交互。
/// 
/// 
use std::fmt::Debug;
mod insight;
mod knowledge;
mod memex;

/// 记忆类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
}

/// 记忆条目结构
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub memory_type: MemoryType,
    pub timestamp: Option<u64>,
}

/// 记忆体定义，为LLM提供短期和长期记忆存储和检索功能。
pub trait Memory {
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

/// 消息角色枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 聊天消息结构
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// 工具参数定义
#[derive(Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// 工具信息
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

/// 工具调用的委托接口，允许不同的工具实现自己的调用逻辑。
pub trait ToolDelegate {
    type Output;
    type Args;

    /// 调用指定工具
    async fn call(&self, name: &str, args: Self::Args) -> anyhow::Result<Self::Output>;

    /// 获取可用工具列表
    fn available_tools(&self) -> Vec<ToolInfo>;
}

/// 默认的工具委托实现，什么都不做。
impl ToolDelegate for () {
    type Output = ();
    type Args = ();

    async fn call(&self, _name: &str, _args: Self::Args) -> anyhow::Result<Self::Output> {
        Ok(())
    }

    fn available_tools(&self) -> Vec<ToolInfo> {
        vec![]
    }
}

/// LLM特性，定义了LLM的基本交互方法，包含了洞察和知识处理能力。
pub trait LLM {
    type Output: Debug;

    /// 基础对话能力
    async fn completion(&self, prompt: &str) -> anyhow::Result<Self::Output>;
    async fn chat(&self, messages: &[ChatMessage]) -> anyhow::Result<Self::Output>;

    /// 带工具调用的对话
    async fn chat_with_tools<T: ToolDelegate>(
        &self,
        messages: &[ChatMessage],
        tools: &T,
    ) -> anyhow::Result<Self::Output>;

    /// 数据分析和洞察能力
    async fn analyze(&self, data: &str) -> anyhow::Result<Self::Output>;
    async fn summarize(&self, content: &str) -> anyhow::Result<Self::Output>;

    /// 知识处理能力
    async fn extract_knowledge(&self, raw_data: &str) -> anyhow::Result<Self::Output>;
    async fn query_knowledge(&self, query: &str) -> anyhow::Result<Self::Output>;
}

/// 学习配置
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug)]
pub struct LearningStats {
    pub total_learned_items: usize,
    pub recent_learning_rate: f32,
    pub last_reflection_time: Option<u64>,
    pub knowledge_categories: Vec<String>,
}

/// 基础智能体特性
pub trait Agent {
    type Memory: Memory;
    type LLM: LLM;
    type Tools: ToolDelegate;

    /// 获取组件实例
    fn memory(&self) -> &Self::Memory;
    fn memory_mut(&mut self) -> &mut Self::Memory;
    fn llm(&self) -> &Self::LLM;
    fn tools(&self) -> &Self::Tools;

    /// 处理用户输入并生成响应
    async fn process_input(&mut self, input: &str) -> anyhow::Result<String> {
        // 存储输入到短期记忆
        let input_key = format!("input_{}", chrono::Utc::now().timestamp());
        self.memory_mut()
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
        let knowledge = self
            .llm()
            .extract_knowledge(information)
            .await
           ?;

        // 生成摘要
        let summary = self
            .llm()
            .summarize(information)
            .await
            ?;

        // 存储到长期记忆
        let key = format!("knowledge_{}", chrono::Utc::now().timestamp());
        let stored_content = format!("Summary: {:?}\nKnowledge: {:?}", summary, knowledge);

        self.memory_mut()
            .store(&key, &stored_content, MemoryType::LongTerm)
            .await
           ?;

        Ok(())
    }

    /// 反思和总结当前状态
    async fn reflect(&self) -> anyhow::Result<String> {
        // 搜索最近的交互记录
        let recent_interactions = self
            .memory()
            .search("input", Some(MemoryType::ShortTerm))
            .await
            ?;

        // 获取长期知识作为上下文
        let knowledge_context = self
            .memory()
            .search("knowledge", Some(MemoryType::LongTerm))
            .await
            ?;

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

/// 高级智能体特性
pub trait AdvancedAgent: Agent {
    /// 获取学习配置
    fn learning_config(&self) -> &LearningConfig;

    /// 获取反思配置  
    fn reflection_config(&self) -> &ReflectionConfig;

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
        self.learn(information).await?;
        Ok(true)
    }

    /// 定期反思：基于配置自动触发反思
    async fn periodic_reflect(&mut self) -> anyhow::Result<Option<String>> {
        let config = self.reflection_config();
        let now = chrono::Utc::now().timestamp() as u64;

        // 检查是否需要反思（简化版本，实际实现需要存储上次反思时间）
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
        let knowledge_keys = self.memory().list_keys(MemoryType::LongTerm).await?;
        let knowledge_entries = self
            .memory()
            .search("knowledge", Some(MemoryType::LongTerm))
            .await?;

        Ok(LearningStats {
            total_learned_items: knowledge_entries.len(),
            recent_learning_rate: 0.0,  // 需要根据时间计算
            last_reflection_time: None, // 需要从记忆中获取
            knowledge_categories: vec!["general".to_string()], // 需要分析知识内容
        })
    }
}

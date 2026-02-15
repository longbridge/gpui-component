# Agent & Workflow 通用调度系统设计

## 1. 目标

设计一套通用的 Agent/Workflow 注册和调度机制：

1. **可扩展**：新增 Agent 只需实现 trait + 注册，无需修改调度层代码
2. **意图驱动**：用户输入自然语言后，由 LLM 通过提示词拼接自动识别意图并路由到对应 Agent
3. **统一接口**：所有 Agent 共享输入/输出协议，面板层无需关心具体实现
4. **与现有架构兼容**：复用 `ChatEngine`、`ChatStreamProcessor`、`SessionService` 等基础设施

## 2. 架构总览

```
┌──────────────────────────────────────────────────────────────┐
│                       Chat Panel (UI)                         │
│                                                               │
│  User Input                                                   │
│      │                                                        │
│      ▼                                                        │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  AgentDispatcher                                      │    │
│  │                                                       │    │
│  │  Step 1: 快速规则匹配 (零延迟)                          │    │
│  │    ├─ 命令前缀 "/sql" → SqlAgent                      │    │
│  │    ├─ @表名 → SqlAgent                                │    │
│  │    └─ 会话亲和性 → 沿用上一轮 Agent                     │    │
│  │                                                       │    │
│  │  Step 2: 只剩一个可用 Agent → 直接使用                  │    │
│  │                                                       │    │
│  │  Step 3: LLM Prompt 意图识别                           │    │
│  │    拼接所有 Agent 描述到 prompt → LLM 回复 agent_id    │    │
│  └──────────┬───────────┬───────────┬────────────────────┘    │
│             │           │           │                          │
│             ▼           ▼           ▼                          │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│  │ SqlAgent   │ │ ChatAgent  │ │ 其他 Agent  │  ...          │
│  └─────┬──────┘ └─────┬──────┘ └─────┬──────┘               │
│        └──────────────┼──────────────┘                        │
│                       ▼                                        │
│           AgentEvent (流式输出)                                │
│           → Progress / TextDelta / Completed                  │
└──────────────────────────────────────────────────────────────┘
```

## 3. 核心类型定义

### 3.1 AgentDescriptor

每个 Agent 提供一份结构化描述，调度器据此做规则匹配和 prompt 拼接。

```rust
#[derive(Clone, Debug)]
pub struct AgentDescriptor {
    /// 唯一标识，同时也是 LLM 回复中引用此 Agent 的 key
    pub id: &'static str,

    /// 展示名称
    pub display_name: &'static str,

    /// 能力描述 —— 会被拼入意图识别 prompt
    pub description: &'static str,

    /// 触发关键词 —— 用于 Step 1 规则快速匹配
    pub keywords: &'static [&'static str],

    /// 命令前缀 (e.g., "/sql", "/explain")
    pub command_prefix: Option<&'static str>,

    /// 示例用户输入 —— 拼入 prompt 帮助 LLM 理解边界
    pub examples: &'static [&'static str],

    /// 所需上下文能力 (e.g., "database_connection")
    /// 缺少则此 Agent 不可用，不参与路由
    pub required_capabilities: &'static [&'static str],

    /// 优先级（数字越小越优先），规则匹配命中多个时排序用
    pub priority: u32,
}
```

### 3.2 AgentContext

```rust
/// Agent 执行上下文
pub struct AgentContext {
    /// 用户原始输入
    pub user_input: String,

    /// 对话历史
    pub chat_history: Vec<Message>,

    /// 当前 Provider 配置
    pub provider_config: ProviderConfig,

    /// 存储管理器
    pub storage_manager: StorageManager,

    /// 取消令牌
    pub cancel_token: CancellationToken,

    /// 能力集合 —— 当前环境提供的上下文数据
    /// key 与 AgentDescriptor.required_capabilities 对应
    pub capabilities: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl AgentContext {
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    pub fn get_capability<T: 'static>(&self, name: &str) -> Option<&T> {
        self.capabilities.get(name)?.downcast_ref::<T>()
    }
}
```

### 3.3 AgentEvent

```rust
#[derive(Clone, Debug)]
pub enum AgentEvent {
    /// 阶段进度
    Progress { stage: String, detail: Option<String> },
    /// 流式文本增量
    TextDelta(String),
    /// 最终结果
    Completed(AgentResult),
    /// 错误
    Error(String),
    /// 已取消
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct AgentResult {
    /// 主内容 (Markdown)
    pub content: String,
    /// 结构化产物
    pub artifacts: Vec<Artifact>,
    /// 建议追问
    pub suggested_followups: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Artifact {
    Sql { code: String, dialect: String },
    Code { code: String, language: String },
    Custom { kind: String, data: serde_json::Value },
}
```

### 3.4 Agent Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync + 'static {
    fn descriptor(&self) -> &AgentDescriptor;

    /// 默认实现：检查 required_capabilities 是否全部满足
    fn is_available(&self, ctx: &AgentContext) -> bool {
        self.descriptor()
            .required_capabilities
            .iter()
            .all(|cap| ctx.has_capability(cap))
    }

    /// 执行 Agent，通过 tx 流式发送事件
    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>);
}
```

## 4. AgentRegistry

```rust
/// GPUI Global 单例
pub struct AgentRegistry {
    agents: HashMap<&'static str, Arc<dyn Agent>>,
    sorted_ids: Vec<&'static str>, // 按 priority 排序，register 时重建
}

impl Global for AgentRegistry {}

impl AgentRegistry {
    pub fn new() -> Self { ... }

    pub fn register(&mut self, agent: impl Agent) {
        let id = agent.descriptor().id;
        self.agents.insert(id, Arc::new(agent));
        self.rebuild_sorted();
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Agent>> {
        self.agents.get(id)
    }

    /// 返回当前上下文中可用的 Agent（已按优先级排序）
    pub fn available_agents(&self, ctx: &AgentContext) -> Vec<&Arc<dyn Agent>> {
        self.sorted_ids.iter()
            .filter_map(|id| self.agents.get(id))
            .filter(|a| a.is_available(ctx))
            .collect()
    }
}
```

## 5. IntentRouter —— 基于提示词拼接的意图识别

### 5.1 设计思路

**不使用 function calling**。将所有可用 Agent 的描述拼接到 system prompt，要求 LLM 回复一个 JSON `{"agent_id": "xxx"}`。

优势：
- **兼容性强**：不依赖 tool use / function calling，任何 LLM 都能工作
- **可控性高**：prompt 内容完全可定制，可以精确引导选择逻辑
- **解析简单**：只需从回复中提取一个 JSON 字段
- **成本低**：输入短、输出只有一行 JSON，token 消耗极少

### 5.2 Prompt 拼接

```rust
impl IntentRouter {
    /// 构建完整的路由 prompt
    fn build_routing_prompt(
        agents: &[&Arc<dyn Agent>],
        user_input: &str,
        recent_history: &[Message],
    ) -> Vec<Message> {
        // --- system prompt ---
        let agent_list = Self::build_agent_list(agents);
        let agent_ids: Vec<_> = agents.iter().map(|a| a.descriptor().id).collect();

        let system = format!(
            "你是一个意图识别路由器。根据用户的输入，从 Agent 列表中选择最合适的一个。\n\n\
             ## 可用 Agent\n\n\
             {agent_list}\n\n\
             ## 回复格式\n\n\
             只回复一个 JSON，不要有任何其他文字：\n\
             {{\"agent_id\": \"<id>\"}}\n\n\
             可选值: {ids}\n\n\
             如果不确定，选择 \"general_chat\"。",
            agent_list = agent_list,
            ids = agent_ids.join(", "),
        );

        let mut messages = vec![Message::system(&system)];

        // --- 最近 2 轮历史（帮助理解追问上下文）---
        let history_window = recent_history.len().saturating_sub(4);
        for msg in &recent_history[history_window..] {
            messages.push(msg.clone());
        }

        // --- 当前用户输入 ---
        messages.push(Message::text(Role::User, user_input));

        messages
    }

    /// 拼接 Agent 描述列表
    fn build_agent_list(agents: &[&Arc<dyn Agent>]) -> String {
        agents.iter().map(|agent| {
            let d = agent.descriptor();
            let examples = d.examples.iter()
                .map(|e| format!("  - \"{}\"", e))
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                "### {id}\n名称: {name}\n描述: {desc}\n示例:\n{examples}",
                id = d.id,
                name = d.display_name,
                desc = d.description,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
    }
}
```

### 5.3 LLM 调用与解析

```rust
/// 路由结果
#[derive(Clone, Debug)]
pub struct RoutingDecision {
    pub agent_id: String,
}

impl IntentRouter {
    /// 调用 LLM 做意图识别
    pub async fn route(
        agents: &[&Arc<dyn Agent>],
        user_input: &str,
        chat_history: &[Message],
        provider: &dyn LlmProvider,
        cancel_token: &CancellationToken,
    ) -> Result<RoutingDecision, RouterError> {
        let messages = Self::build_routing_prompt(agents, user_input, chat_history);
        let valid_ids: Vec<&str> = agents.iter().map(|a| a.descriptor().id).collect();

        // 非流式调用，max_tokens 设小以加速
        let request = ChatRequest::new()
            .messages(messages)
            .max_tokens(50)
            .temperature(0.0); // 确定性输出

        let response = provider.chat(request).await
            .map_err(RouterError::LlmError)?;

        // 解析回复
        Self::parse_response(&response.content, &valid_ids)
    }

    /// 从 LLM 回复中提取 agent_id
    fn parse_response(
        content: &str,
        valid_ids: &[&str],
    ) -> Result<RoutingDecision, RouterError> {
        // 尝试 JSON 解析
        let trimmed = content.trim();

        // 策略 1: 直接解析 JSON
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(id) = v.get("agent_id").and_then(|v| v.as_str()) {
                if valid_ids.contains(&id) {
                    return Ok(RoutingDecision { agent_id: id.to_string() });
                }
            }
        }

        // 策略 2: 从回复中提取 JSON 片段 (LLM 可能在 JSON 前后加了文字)
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                let json_str = &trimmed[start..=end];
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(id) = v.get("agent_id").and_then(|v| v.as_str()) {
                        if valid_ids.contains(&id) {
                            return Ok(RoutingDecision { agent_id: id.to_string() });
                        }
                    }
                }
            }
        }

        // 策略 3: 直接匹配 —— 回复可能就是一个 agent_id
        for id in valid_ids {
            if trimmed == *id || trimmed.contains(id) {
                return Ok(RoutingDecision { agent_id: id.to_string() });
            }
        }

        // 全部失败
        Err(RouterError::ParseFailed(content.to_string()))
    }
}
```

### 5.4 实际 Prompt 示例

```
System:
你是一个意图识别路由器。根据用户的输入，从 Agent 列表中选择最合适的一个。

## 可用 Agent

### sql_query
名称: SQL 查询助手
描述: 根据自然语言生成 SQL 查询语句，支持 @表名 语法
示例:
  - "查询所有用户的订单数量"
  - "统计每个部门的平均工资"

### sql_explain
名称: SQL 解释优化
描述: 解释、分析、优化已有的 SQL 语句
示例:
  - "解释一下这个 SQL 在做什么"
  - "这个查询怎么优化"

### general_chat
名称: 通用对话
描述: 处理一般性问答、知识咨询、闲聊
示例:
  - "你好"
  - "什么是索引"

## 回复格式

只回复一个 JSON，不要有任何其他文字：
{"agent_id": "<id>"}

可选值: sql_query, sql_explain, general_chat

如果不确定，选择 "general_chat"。

User:
帮我查一下最近7天注册的用户数量

Assistant → `{"agent_id": "sql_query"}`
```

## 6. AgentDispatcher —— 三级调度

将规则匹配、上下文推断、LLM 路由合并为一个统一的调度流程。

### 6.1 三级路由逻辑

```
Step 1: 快速规则匹配（零延迟，零成本）
  ├─ 用户输入以命令前缀开头 → 精确匹配 agent
  ├─ 用户输入包含某 Agent 的 keywords → 匹配
  └─ 会话亲和性：上一轮绑定了 agent 且未显式切换 → 沿用

Step 2: 可用 Agent 数量判断（零延迟）
  ├─ 0 个可用 → 返回错误
  ├─ 1 个可用 → 直接使用
  └─ 多个可用 → 进入 Step 3

Step 3: LLM 提示词意图识别
  ├─ 调用 IntentRouter.route()
  ├─ 成功 → 使用返回的 agent_id
  └─ 失败 → fallback 到优先级最高的可用 Agent
```

### 6.2 实现

```rust
pub struct AgentDispatcher;

/// 会话亲和性状态
pub struct SessionAffinity {
    /// 当前绑定的 Agent ID
    pub current_agent_id: Option<String>,
    /// 连续使用同一 Agent 的轮次
    pub consecutive_rounds: u32,
}

impl AgentDispatcher {
    pub async fn dispatch(
        ctx: AgentContext,
        registry: &AgentRegistry,
        affinity: &mut SessionAffinity,
        provider: Arc<dyn LlmProvider>,
    ) -> mpsc::Receiver<AgentEvent> {
        let (tx, rx) = mpsc::channel(32);
        let available = registry.available_agents(&ctx);

        // --- Step 1: 快速规则匹配 ---
        if let Some(agent) = Self::rule_match(&ctx, &available, affinity) {
            affinity.current_agent_id = Some(agent.descriptor().id.to_string());
            affinity.consecutive_rounds += 1;
            let agent = agent.clone();
            tokio::spawn(async move { agent.execute(ctx, tx).await });
            return rx;
        }

        // --- Step 2: 数量判断 ---
        match available.len() {
            0 => {
                let _ = tx.send(AgentEvent::Error("没有可用的 Agent".into())).await;
                return rx;
            }
            1 => {
                let agent = available[0].clone();
                affinity.current_agent_id = Some(agent.descriptor().id.to_string());
                affinity.consecutive_rounds = 1;
                tokio::spawn(async move { agent.execute(ctx, tx).await });
                return rx;
            }
            _ => {}
        }

        // --- Step 3: LLM 意图识别 ---
        let decision = IntentRouter::route(
            &available,
            &ctx.user_input,
            &ctx.chat_history,
            provider.as_ref(),
            &ctx.cancel_token,
        ).await;

        let agent_id = match decision {
            Ok(d) => d.agent_id,
            Err(_) => {
                // fallback: 优先级最高的可用 Agent
                available[0].descriptor().id.to_string()
            }
        };

        affinity.current_agent_id = Some(agent_id.clone());
        affinity.consecutive_rounds = 1;

        let agent = registry.get(&agent_id)
            .cloned()
            .unwrap_or_else(|| available[0].clone());

        tokio::spawn(async move { agent.execute(ctx, tx).await });

        rx
    }

    /// Step 1: 规则匹配
    fn rule_match<'a>(
        ctx: &AgentContext,
        available: &[&'a Arc<dyn Agent>],
        affinity: &SessionAffinity,
    ) -> Option<&'a Arc<dyn Agent>> {
        let input = &ctx.user_input;
        let input_lower = input.to_lowercase();

        // 1a. 命令前缀匹配
        for agent in available {
            if let Some(prefix) = agent.descriptor().command_prefix {
                if input_lower.starts_with(prefix) {
                    return Some(agent);
                }
            }
        }

        // 1b. 关键词匹配（取优先级最高的命中者）
        for agent in available {
            let d = agent.descriptor();
            if d.keywords.iter().any(|kw| input_lower.contains(kw)) {
                return Some(agent);
            }
        }

        // 1c. 会话亲和性
        if let Some(ref current_id) = affinity.current_agent_id {
            // 同一 Agent 连续 10 轮以上时不再自动沿用，避免卡死
            if affinity.consecutive_rounds < 10 {
                return available.iter().find(|a| a.descriptor().id == current_id).copied();
            }
        }

        None
    }
}
```

## 7. 内置 Agent 示例

### 7.1 GeneralChatAgent

```rust
pub struct GeneralChatAgent {
    descriptor: AgentDescriptor,
}

impl GeneralChatAgent {
    pub fn new() -> Self {
        Self {
            descriptor: AgentDescriptor {
                id: "general_chat",
                display_name: "通用对话",
                description: "处理一般性问答、闲聊、知识咨询",
                keywords: &[],       // 无特定关键词，作为 fallback
                command_prefix: None,
                examples: &["你好", "什么是 REST API", "帮我解释一下这段代码"],
                required_capabilities: &[],
                priority: 100,        // 最低优先级
            },
        }
    }
}

#[async_trait]
impl Agent for GeneralChatAgent {
    fn descriptor(&self) -> &AgentDescriptor { &self.descriptor }

    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>) {
        // 复用 ChatStreamProcessor 流式对话
        let request = ChatRequest::new()
            .messages(/* chat_history + user_input */)
            .model(&ctx.provider_config.model);

        // 调用 provider.chat_stream()，转发为 TextDelta / Completed
    }
}
```

### 7.2 SqlQueryAgent

```rust
pub struct SqlQueryAgent {
    descriptor: AgentDescriptor,
}

impl SqlQueryAgent {
    pub fn new() -> Self {
        Self {
            descriptor: AgentDescriptor {
                id: "sql_query",
                display_name: "SQL 查询助手",
                description: "根据自然语言生成 SQL 查询语句，支持 @表名 语法指定表",
                keywords: &["@"],     // @表名 是强信号
                command_prefix: Some("/sql"),
                examples: &[
                    "查询所有用户的订单数量",
                    "帮我写一个 SQL 查 @users 表中年龄大于 18 的",
                    "统计每个部门的平均工资",
                ],
                required_capabilities: &["database_connection"],
                priority: 10,
            },
        }
    }
}

#[async_trait]
impl Agent for SqlQueryAgent {
    fn descriptor(&self) -> &AgentDescriptor { &self.descriptor }

    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>) {
        let parsed = parse_user_input(&ctx.user_input);

        let _ = tx.send(AgentEvent::Progress {
            stage: "分析查询意图...".into(),
            detail: None,
        }).await;

        // 复用现有 WorkflowExecutor
        let conn = ctx.get_capability::<DatabaseConnection>("database_connection").unwrap();
        let executor = WorkflowExecutor::new(/* from conn */);
        let action = executor.start(&parsed, /* ... */).await;

        match action {
            WorkflowAction::ReadyToGenerate { context } => {
                // 调用 LLM 生成 SQL，流式输出
            }
            WorkflowAction::RequireUserMention { message, .. } => {
                let _ = tx.send(AgentEvent::Completed(AgentResult {
                    content: message,
                    artifacts: vec![],
                    suggested_followups: vec!["请使用 @表名 指定要查询的表".into()],
                })).await;
            }
            WorkflowAction::Error(msg) => {
                let _ = tx.send(AgentEvent::Error(msg)).await;
            }
            _ => {}
        }
    }
}
```

### 7.3 Agent 一览表

| Agent ID | 名称 | 关键词 | 命令前缀 | 所需能力 | 优先级 |
|----------|------|--------|---------|---------|--------|
| `sql_query` | SQL 查询助手 | `@` | `/sql` | `database_connection` | 10 |
| `sql_explain` | SQL 解释优化 | `explain`, `优化` | `/explain` | `database_connection` | 20 |
| `redis_helper` | Redis 助手 | `redis` | `/redis` | `redis_connection` | 30 |
| `shell_helper` | Shell 助手 | `shell`, `命令行` | `/shell` | `terminal_session` | 40 |
| `general_chat` | 通用对话 | (无) | (无) | (无) | 100 |

## 8. ChatPanel 集成

### 8.1 改造后的 send_message

```rust
impl ChatPanel {
    fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        if content.trim().is_empty() || self.is_loading {
            return;
        }

        // 添加用户消息到 UI
        self.messages.push(ChatMessageUI::user(content.clone()));
        self.is_loading = true;
        cx.notify();

        // 构建 AgentContext
        let ctx = self.build_agent_context(&content, cx);
        let registry = cx.global::<AgentRegistry>().clone();
        let provider = self.get_current_provider(cx);

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut rx = AgentDispatcher::dispatch(
                ctx,
                &registry,
                &mut this.update(cx, |p, _| &mut p.session_affinity),
                provider,
            ).await;

            while let Some(event) = rx.recv().await {
                let _ = this.update(cx, |panel, cx| {
                    panel.handle_agent_event(event, cx);
                });
            }
        }).detach();
    }

    fn build_agent_context(&self, content: &str, cx: &App) -> AgentContext {
        let mut capabilities = HashMap::new();

        // 如果有数据库连接，注入 capability
        if let Some((conn_id, db, schema)) = self.ai_input.read(cx).get_connection_info() {
            capabilities.insert(
                "database_connection".to_string(),
                Box::new(DatabaseConnection { conn_id, db, schema }) as _,
            );
        }

        AgentContext {
            user_input: content.to_string(),
            chat_history: self.chat_history.clone(),
            provider_config: self.get_provider_config(cx),
            storage_manager: self.storage_manager.clone(),
            cancel_token: CancellationToken::new(),
            capabilities,
        }
    }

    fn handle_agent_event(&mut self, event: AgentEvent, cx: &mut Context<Self>) {
        match event {
            AgentEvent::Progress { stage, detail } => {
                self.update_status_message(&stage, detail.as_deref());
            }
            AgentEvent::TextDelta(delta) => {
                self.append_to_streaming_message(&delta);
            }
            AgentEvent::Completed(result) => {
                self.finalize_message(result);
                self.is_loading = false;
            }
            AgentEvent::Error(msg) => {
                self.show_error(&msg);
                self.is_loading = false;
            }
            AgentEvent::Cancelled => {
                self.handle_cancel();
            }
        }
        cx.notify();
    }
}
```

### 8.2 初始化注册

```rust
// main/src/main.rs

pub fn init_agent_system(cx: &mut App) {
    let mut registry = AgentRegistry::new();

    // 核心 Agent
    registry.register(GeneralChatAgent::new());

    // 数据库 Agent (在 db_view 初始化时注册)
    registry.register(SqlQueryAgent::new());
    registry.register(SqlExplainAgent::new());

    // 其他 Agent 按需注册...

    cx.set_global(registry);
}
```

## 9. 意图识别优化

### 9.1 三级路由的延迟分析

| 级别 | 触发条件 | 延迟 | Token 成本 |
|------|---------|------|-----------|
| Step 1 规则匹配 | 命令前缀 / 关键词 / 亲和性 | 0ms | 0 |
| Step 2 数量判断 | 只有 0-1 个可用 Agent | 0ms | 0 |
| Step 3 LLM 路由 | 多个可用 + 规则未命中 | 200-500ms | ~100 tokens |

**大部分场景在 Step 1/2 就能完成路由**：
- 用户输入包含 `@` → 直接 SqlAgent（Step 1 关键词）
- 用户输入 `/sql ...` → 直接 SqlAgent（Step 1 命令前缀）
- 追问上一轮 → 沿用同 Agent（Step 1 亲和性）
- 没连数据库 → 只有 GeneralChatAgent 可用（Step 2）

### 9.2 LLM 路由调用优化

```rust
impl IntentRouter {
    pub async fn route(/* ... */) -> Result<RoutingDecision, RouterError> {
        let request = ChatRequest::new()
            .messages(messages)
            .max_tokens(50)      // 回复只有一行 JSON，50 token 足够
            .temperature(0.0);   // 确定性输出，避免随机性

        // 非流式调用，直接拿完整回复
        let response = provider.chat(request).await?;
        Self::parse_response(&response.content, &valid_ids)
    }
}
```

关键优化点：
- `max_tokens(50)`：限制输出长度，加速响应
- `temperature(0.0)`：确定性选择，避免同一输入路由到不同 Agent
- 非流式调用：意图识别不需要流式
- 历史窗口截断：只传最近 2 轮，减少 token 输入

### 9.3 会话亲和性

```rust
pub struct SessionAffinity {
    pub current_agent_id: Option<String>,
    pub consecutive_rounds: u32,
}
```

规则：
- 上一轮路由到某 Agent 后，**后续追问默认沿用**，跳过 LLM 路由
- 用户用命令前缀（`/sql`, `/chat`）可显式切换
- 连续 10 轮后亲和性失效，防止卡死在某个 Agent
- 新会话时亲和性重置

## 10. 文件组织

```
crates/core/src/
├── agent/                          # 新增: Agent 框架
│   ├── mod.rs
│   ├── types.rs                    # AgentDescriptor, AgentContext, AgentEvent, Artifact
│   ├── registry.rs                 # AgentRegistry (Global)
│   ├── router.rs                   # IntentRouter, RoutingDecision, parse_response
│   ├── dispatcher.rs               # AgentDispatcher, SessionAffinity
│   └── builtin/
│       ├── mod.rs
│       └── general_chat.rs         # GeneralChatAgent
├── ai_chat/                        # 现有 (不变, 被 Agent 复用)
│   ├── engine.rs
│   ├── stream.rs
│   └── ...
└── llm/                            # 现有 (不变)

crates/db_view/src/chatdb/
├── agents/                         # 新增: 数据库相关 Agent
│   ├── mod.rs
│   ├── sql_query.rs                # SqlQueryAgent (封装 WorkflowExecutor)
│   └── sql_explain.rs              # SqlExplainAgent
├── chat_panel.rs                   # 改造: 接入 AgentDispatcher
└── workflow/                       # 现有 (被 SqlQueryAgent 内部复用)
```

## 11. 渐进式迁移

**Phase 1 — 基础框架**
- 实现 `Agent` trait、`AgentRegistry`、`AgentEvent`
- 实现 `GeneralChatAgent`（封装现有 `ChatStreamProcessor`）
- 在 `AiChatPanel`（通用聊天面板）中接入
- 现有 `ChatPanel`（数据库面板）暂不改动

**Phase 2 — SQL Agent + 路由**
- 实现 `SqlQueryAgent`（封装现有 `WorkflowExecutor`）
- 实现 `IntentRouter` + `AgentDispatcher`
- 在 `ChatPanel` 中接入，替换 `recognize_query_intent` 硬编码逻辑

**Phase 3 — 扩展**
- 更多 Agent：SqlExplainAgent、RedisAgent 等
- UI 增强：Agent 标签展示、手动切换下拉

## 12. 设计决策总结

| 决策 | 选择 | 理由 |
|------|------|------|
| 意图识别方式 | Prompt 拼接 + JSON 回复 | 兼容所有 provider，不依赖 function calling |
| 路由策略 | 三级：规则 → 数量 → LLM | 大部分场景零延迟，只有模糊意图才走 LLM |
| Agent 通信 | `mpsc::channel<AgentEvent>` | 异步流式，与现有 StreamEvent 模式一致 |
| 注册方式 | GPUI Global 单例 | 与 `DatabaseViewPluginRegistry` 模式一致 |
| 上下文传递 | `capabilities: HashMap<String, Box<dyn Any>>` | 灵活支持不同 Agent 的特殊需求 |
| Agent 执行 | `tokio::spawn` | 不阻塞 UI |
| 亲和性 | 自动沿用 + 命令切换 | 追问场景零延迟，用户可控 |

## 13. 端到端示例

**用户输入**: "帮我查一下最近7天注册的用户数量"

```
1. ChatPanel.send_message("帮我查一下最近7天注册的用户数量")
2. build_agent_context → capabilities 包含 database_connection

3. AgentDispatcher Step 1:
   - 命令前缀? No
   - 关键词? No ("@" 不在输入中)
   - 亲和性? No (新会话)

4. AgentDispatcher Step 2:
   - 可用 Agent = [SqlQueryAgent, GeneralChatAgent] → 2个，继续

5. AgentDispatcher Step 3: IntentRouter.route()
   - 拼接 prompt，发送给 LLM
   - LLM 回复: {"agent_id": "sql_query"}
   - 解析成功

6. SqlQueryAgent.execute()
   → AgentEvent::Progress("分析查询意图...")
   → WorkflowExecutor 获取表列表 → 选择 users 表
   → AgentEvent::Progress("获取表结构...")
   → AgentEvent::Progress("生成 SQL...")
   → AgentEvent::TextDelta("```sql\nSELECT COUNT(*)...")
   → AgentEvent::Completed(AgentResult {
       content: "...",
       artifacts: [Sql { code: "SELECT COUNT(*) ...", dialect: "postgresql" }],
       suggested_followups: ["查看详细信息", "按天统计趋势"],
     })

7. ChatPanel 渲染消息 + SQL 代码块 + 运行按钮
8. SessionAffinity.current_agent_id = "sql_query"
```

**用户追问**: "再帮我看看他们的邮箱分布"

```
1. AgentDispatcher Step 1:
   - 亲和性命中 → sql_query
   - 直接使用 SqlQueryAgent（跳过 LLM，零延迟）

2. SqlQueryAgent.execute() → 生成新的 SQL ...
```

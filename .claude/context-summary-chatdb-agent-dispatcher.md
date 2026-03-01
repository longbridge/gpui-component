## 项目上下文摘要（chatdb-agent-dispatcher）
生成时间：2026-03-01 10:22:01 +0800

### 1. 相似实现分析
- 实现1：`crates/core/src/agent/dispatcher.rs`
  - 模式：三层路由（规则匹配 → 单候选捷径 → LLM路由）
  - 可复用：`AgentDispatcher::dispatch`、`SessionAffinity`
  - 注意点：`dispatch` 内部依赖 `tokio::spawn`，调用侧必须位于 Tokio runtime

- 实现2：`crates/core/src/agent/builtin/general_chat.rs`
  - 模式：`chat_stream` + `mpsc::Sender<AgentEvent>` 增量推送
  - 可复用：`TextDelta/Completed/Cancelled` 事件契约
  - 注意点：使用 50ms 节流，完成时要 flush `pending_delta`

- 实现3：`crates/db_view/src/chatdb/agents/sql_workflow.rs`
  - 模式：多阶段 Progress + streaming 输出 + capability 驱动
  - 可复用：`CAP_DB_METADATA` 与 `DatabaseMetadataProvider`
  - 注意点：必须在 `AgentContext` 注入 capability 才能命中 SQL Agent

### 2. 项目约定
- 命名约定：Rust 结构体/trait 使用 PascalCase，函数 snake_case
- 文件组织：`chatdb/agents` 放 Agent 实现，`chat_panel` 负责 UI 编排
- 代码风格：早返回、`Option` 匹配、`cx.spawn` + `AsyncApp` 更新 UI

### 3. 可复用组件清单
- `crates/core/src/agent/registry.rs`：Agent 注册和可用性筛选
- `crates/core/src/agent/types.rs`：`AgentContext` 与 `AgentEvent` 协议
- `crates/db_view/src/chatdb/agents/db_metadata.rs`：DB metadata capability 适配器

### 4. 测试策略
- 测试框架：Rust `cargo test`/`cargo check`
- 参考测试：`crates/db_view/src/chatdb/query_workflow.rs` 内单元测试
- 本次验证：优先 `cargo check -p db_view`，确认改造不破坏编译链

### 5. 依赖和集成点
- 外部依赖：`tokio`、`tokio_util::CancellationToken`
- 内部依赖：`one_core::agent::*`、`GlobalProviderState`、`SessionService`
- 集成位置：`chat_panel::send_to_ai` -> `AgentDispatcher::dispatch`

### 6. 技术选型理由
- 采用全局 `AgentRegistry` 快照而非临时重建，减少重复与错配风险
- 保留 capability 注入，确保 SQL/通用聊天自动路由行为不变

### 7. 关键风险点
- 并发：UI 状态更新在 `cx.update_window` 中必须保持一致
- 边界：无 provider、无 session、cancel 竞争态需要保持原逻辑
- 性能：消息流高频 delta 更新需继续依赖节流策略

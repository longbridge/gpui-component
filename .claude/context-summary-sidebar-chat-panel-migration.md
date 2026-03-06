## 项目上下文摘要（sidebar-chat-panel-migration）
生成时间：2026-03-06 10:20:55 CST

### 1. 相似实现分析
- 实现1: crates/db_view/src/chatdb/chat_panel.rs:1326
  - 模式：消息列表与输入区解耦（`render_messages` + `render_input`）
  - 可复用：完整 AI/SQL 消息发送与 SQL 执行链路
  - 需注意：当前默认带会话历史侧栏

- 实现2: crates/db_view/src/sidebar/mod.rs:57
  - 模式：侧栏只负责面板容器与 AskAi 转发
  - 可复用：`toggle_panel` / `ask_ai` / 工具栏渲染
  - 需注意：当前绑定的是 one_core 通用 `AiChatPanel`

- 实现3: crates/db_view/src/chatdb/db_connection_selector.rs:167
  - 模式：连接/数据库/Schema 三级选择 + 懒加载
  - 可复用：`handle_connection_selected`、`load_databases`、`emit_selection`
  - 需注意：当前从仓库全量加载连接，不区分单连接与工作区

### 2. 项目约定
- 命名约定：结构体 PascalCase，方法/变量 snake_case
- 文件组织：功能模块内聚（`chatdb/*` + `sidebar/*`）
- 导入顺序：标准库、第三方、本地模块分组
- 代码风格：早返回、事件驱动、`cx.notify()` 通知刷新

### 3. 可复用组件清单
- `crates/db_view/src/chatdb/chat_panel.rs`: AI 消息与 SQL 执行主逻辑
- `crates/db_view/src/chatdb/ai_input.rs`: AI 输入、模型设置与数据源选择
- `crates/db_view/src/chatdb/db_connection_selector.rs`: 连接/数据库选择器
- `crates/db_view/src/database_tab.rs`: workspace / active_conn_id 上下文源头
### 4. 测试策略
- 测试框架：Rust `cargo check` + 编译期类型校验
- 验证模式：本次以功能改造为主，先做模块编译验证
- 参考文件：`crates/db_view/src/database_tab.rs`、`crates/db_view/src/sidebar/mod.rs`
- 覆盖要求：侧栏渲染、AskAi 转发、默认连接/数据库选中

### 5. 依赖和集成点
- 外部依赖：`gpui`、`gpui_component`
- 内部依赖：`DatabaseTabView -> DatabaseSidebar -> ChatPanel -> AIInput -> DbConnectionSelector`
- 集成方式：Entity 事件订阅 + 上下文注入
- 配置来源：`DatabaseTabView::new_with_active_conn(workspace, connections, active_conn_id)`

### 6. 技术选型理由
- 方案：用 `ChatPanel` 替换 `AiChatPanel`，通过配置切换“完整模式/侧栏模式”
- 优势：复用既有 AI/SQL 业务逻辑，避免复制粘贴
- 风险：`ChatPanel` 与 `DbConnectionSelector` 需要新增上下文配置与默认选择逻辑

### 7. 关键风险点
- 并发问题：连接切换时异步数据库加载结果可能回写旧连接
- 边界条件：工作区无连接、连接无数据库、selected_databases 为空
- 性能瓶颈：多连接下数据库列表加载频次
- 安全考虑：本次不新增安全逻辑，仅保持现有行为
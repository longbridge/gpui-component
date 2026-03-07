## 项目上下文摘要（terminal-sidebar-ai-system-prompt）
生成时间：2026-03-07 11:35:26 +0800

### 1. 相似实现分析
- 实现1: `crates/terminal_view/src/sidebar/mod.rs`
  - 模式：终端侧边栏只负责创建 `AiChatPanel` 并注册代码块动作
  - 可复用：`AiChatPanel::new`、`register_code_block_action`
  - 需注意：当前没有终端专属提示词注入

- 实现2: `crates/core/src/ai_chat/panel.rs`
  - 模式：`AiChatPanel` 持有 UI 状态与 `ChatEngine`，`send_message` 统一构建 `Vec<Message>`
  - 可复用：新增可选配置字段和 setter，由调用方注入场景上下文
  - 需注意：不能影响其他复用 `AiChatPanel` 的场景

- 实现3: `crates/core/src/ai_chat/engine.rs`
  - 模式：消息历史、会话和代码块动作统一放在共享引擎中管理
  - 可复用：保持历史消息与 UI 逻辑不变，仅在发送前补系统消息
  - 需注意：系统提示应独立于历史条数限制，避免被裁掉

### 2. 项目约定
- 命名约定：结构体 `PascalCase`，方法和字段 `snake_case`
- 文件组织：业务面板放 `panel.rs`，场景接入放业务模块 `sidebar/mod.rs`
- 导入顺序：标准库 / 第三方 / 本地模块分组
- 代码风格：早返回、小范围 setter、状态变化后按需 `cx.notify()`

### 3. 可复用组件清单
- `crates/core/src/ai_chat/panel.rs`: 通用 AI 面板与消息发送入口
- `crates/core/src/ai_chat/engine.rs`: 会话消息与代码块动作注册
- `crates/terminal_view/src/sidebar/mod.rs`: 终端侧边栏场景接入点

### 4. 测试策略
- 测试框架：Rust `cargo check`
- 验证模式：本次以编译校验和场景约束检查为主
- 参考文件：`crates/core/src/ai_chat/panel.rs`、`crates/terminal_view/src/sidebar/mod.rs`
- 覆盖要求：仅终端侧边栏注入提示词，不影响其他 `AiChatPanel` 调用方

### 5. 依赖和集成点
- 外部依赖：`gpui`、`gpui_component`
- 内部依赖：`TerminalSidebar -> AiChatPanel -> ChatStreamProcessor`
- 集成方式：在 `AiChatPanel` 暴露可选 setter，由终端侧边栏在构建后设置
- 配置来源：终端侧边栏内部常量提示词

### 6. 技术选型理由
- 方案：在 `AiChatPanel` 增加可选 `system_instruction` 字段，并在 `send_message` 构造消息时前置插入 `Role::System`
- 优势：最小改动、默认不影响其他场景、终端侧边栏可单独定制
- 风险：如果未来持久化系统消息，需要避免重复注入；当前仓库未见该行为

### 7. 关键风险点
- 并发问题：无新增共享可变状态，仅在发送前克隆字符串
- 边界条件：空字符串提示词应视为未设置
- 性能瓶颈：仅增加一条系统消息，开销极小
- 兼容性：历史条数裁剪后再插入系统消息，避免改变现有历史窗口语义

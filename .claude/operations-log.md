## 编码前检查 - release-workflow-migration
时间：2026-02-28 14:24:04 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-release-workflow-migration.md`
- 复用组件：
  - `script/bootstrap`：系统依赖安装
  - `script/bundle-macos.sh`：macOS 打包
- 命名约定：沿用 workflow/job/step 命名风格
- 代码风格：沿用现有 YAML 缩进与动作版本
- 不重复造轮子证明：基于已有 build-release/release/ci 三个 workflow 合并

## 编码后声明 - release-workflow-migration
时间：2026-02-28 14:24:04 +0800

### 1. 复用组件
- `script/bootstrap`：保持 Linux/macOS 依赖安装入口不变
- `script/bundle-macos.sh`：保持 macOS app bundle 打包逻辑不变

### 2. 遵循约定
- 命名约定：保留 `release.yml`，job 名称使用 `build/release/publish_crate`
- 代码风格：维持 actions 版本与缓存结构一致
- 文件组织：仅修改 `.github/workflows` 与 README

### 3. 相似实现对比
- 对比 build-release：迁入矩阵构建、打包、checksum、GitHub Release 上传
- 对比旧 release：保留 crates 发布职责并增加保护条件

### 4. 未重复造轮子证明
- 复用了已有脚本与矩阵配置，未新增自定义打包脚本或额外发布工具
## 编码前检查 - chatdb-agent-dispatcher
时间：2026-03-01 10:22:01 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-chatdb-agent-dispatcher.md`
- 将复用组件：
  - `one_core::agent::dispatcher::AgentDispatcher`：统一路由分发
  - `one_core::agent::registry::AgentRegistry`：全局 Agent 注册快照
  - `chatdb::agents::CAP_DB_METADATA`：DB 能力注入
- 命名约定：沿用现有 Rust snake_case / PascalCase 约定
- 代码风格：沿用 `cx.spawn + AsyncApp` 与早返回风格
- 不重复造轮子证明：保留现有 Agent 机制，仅删除 chat_panel 的临时 registry 重建中间层

## 编码后声明 - chatdb-agent-dispatcher
时间：2026-03-01 10:26:35 +0800

### 1. 复用了以下既有组件
- `one_core::agent::registry::AgentRegistry`：直接作为 dispatcher 输入的 Agent 注册表
- `one_core::agent::dispatcher::AgentDispatcher`：统一路由与执行入口
- `chatdb::agents::CAP_DB_METADATA`：数据库能力注入键

### 2. 遵循了以下项目约定
- 命名约定：新增/修改字段与函数继续使用 Rust 既有命名风格
- 代码风格：保持 `cx.spawn`、早返回和事件驱动 UI 更新模式
- 文件组织：仅修改 `core/agent` 与 `db_view/chatdb/chat_panel` 的职责边界

### 3. 对比了以下相似实现
- `crates/core/src/agent/dispatcher.rs`：保持三层路由与 affinity 机制不变
- `crates/core/src/agent/builtin/general_chat.rs`：保持事件流契约（TextDelta/Completed/Cancelled）一致
- `crates/db_view/src/chatdb/agents/sql_workflow.rs`：保持 capability 驱动路由行为不变

### 4. 未重复造轮子的证明
- 移除 chat_panel 内重复构建 local registry 的中间层
- 直接复用全局 AgentRegistry 快照，无新增并行路由实现

## 编码前检查 - chatbi-agent-chart
时间：2026-03-01 10:50:45 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-chatbi-agent-chart.md`
- 将复用组件：
  - `SqlWorkflowAgent`（Agent 事件与多阶段执行模式）
  - `chat_panel` 代码块渲染拦截（`code_block_renderer`）
  - `gpui_component::chart`（Line/Bar/Pie）
- 将遵循命名约定：Rust snake_case / PascalCase
- 将遵循代码风格：早返回 + 分层函数
- 不重复造轮子证明：基于现有 Agent/Chart 组件扩展，不引入新渲染框架

## 编码后声明 - chatbi-agent-chart
时间：2026-03-01 11:05:46 +0800

### 1. 复用了以下既有组件
- `SqlWorkflowAgent` 的选表与元数据处理模式
- `chat_panel` 的代码块级渲染拦截机制
- `gpui_component::chart::{LineChart, BarChart, PieChart}`

### 2. 遵循了以下项目约定
- 命名约定：沿用 chatdb agent 模块命名方式
- 代码风格：保持 AgentEvent 分阶段状态推送
- 文件组织：新增 `chatdb/agents/chat_bi.rs` 与 `chatdb/chart_json.rs`

### 3. 对比了以下相似实现
- `chatdb/agents/sql_workflow.rs`：沿用 capability 驱动 Agent 设计
- `chat_panel.rs`：沿用 `code_block_renderer` 渲染注入点
- `story/chart_story.rs`：沿用 chart 组件数据绑定方式

### 4. 未重复造轮子的证明
- 图表渲染直接复用现有 `gpui_component::chart`
- SQL 执行新增 `execute_script_direct` 以复用 DB session 执行链，无新 DB 客户端

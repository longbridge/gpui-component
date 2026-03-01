## 项目上下文摘要（chatbi-agent-chart）
生成时间：2026-03-01 10:50:45 +0800

### 1. 相似实现分析
- 实现1：`crates/db_view/src/chatdb/agents/sql_workflow.rs`
  - 模式：Agent 分阶段执行（Progress/TextDelta/Completed）
  - 可复用：数据库能力注入、表选择与元数据获取流程
  - 注意：必须依赖 `CAP_DB_METADATA`

- 实现2：`crates/db_view/src/chatdb/chat_panel.rs`（`code_block_renderer`）
  - 模式：在 Markdown 代码块级别做结构识别并替换渲染
  - 可复用：`SqlCodeBlock::from_code_block` 与现有代码块拦截点
  - 注意：非匹配结构必须回退 `default_element`

- 实现3：`crates/story/src/stories/chart_story/chart_story.rs`
  - 模式：`gpui_component::chart` 的 Line/Bar/Pie 组件直接渲染
  - 可复用：图表容器尺寸与 `x/y/value` 数据映射
  - 注意：图表必须提供可解析数值与字符串维度

### 2. 项目约定
- Rust 结构：Agent 放在 `chatdb/agents`，渲染逻辑在 `chat_panel`
- 事件协议：通过 `AgentEvent` 驱动 UI 状态更新
- 风格：早返回、`cx.spawn`、`tokio::select!` 取消优先

### 3. 依赖与接口
- 数据库执行：`db::GlobalDbState` 现有 `execute_script` 依赖 AsyncApp
- 渲染组件：`gpui_component::chart::{LineChart, BarChart, PieChart}`
- JSON 解析：`serde_json::Value` 动态对象读取

### 4. 测试策略
- 编译验证：`cargo check -p db_view`
- Agent 逻辑验证：新增 chart JSON 解析单测
- 回归验证：`cargo test -p db_view chatdb -- --nocapture`

### 5. 风险点
- Async 上下文：Agent 内无法直接使用 `AsyncApp`，需提供 direct DB 执行方法
- 输出格式：模型可能生成非严格 JSON，需解析失败回退到普通文本
- 渲染稳定性：图表结构匹配必须保守，避免误判普通 JSON

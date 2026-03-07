## 项目上下文摘要（db-connection-selector-clear）
生成时间：2026-03-07 21:24:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/chatdb/db_connection_selector.rs`
  - 模式：`Option` 状态驱动的三级选择器，事件通过 `emit_selection` 统一发射。
  - 可复用：`emit_selection`、`selection_label`、`render_trigger`、`render_list_item`。
  - 需注意：连接切换会级联重置 database/schema，并依赖懒加载结果回填。

- **实现2**: `crates/db_view/src/chatdb/chat_panel.rs`
  - 模式：通过 `get_connection_info()` 获取上下文，再用 `and_then` 构造数据库能力。
  - 可复用：空值兼容链路，无需调用方新增分支。
  - 需注意：若 database 为空字符串会直接跳过 capability 注入。

- **实现3**: `crates/story/src/stories/table_story.rs`
  - 模式：用显式按钮触发 `clear_selection`。
  - 可复用：辅助操作按钮表达“清除选择”意图。
  - 需注意：清除操作与主交互要分离，避免误触。

### 2. 项目约定
- **命名约定**: Rust 使用 `snake_case` 方法/字段、`PascalCase` 类型。- **文件组织**: `chatdb` 子模块内聚 UI、服务和 Agent；选择器逻辑集中在单文件内。
- **导入顺序**: 先外部 crate，再内部 crate，最后标准库；当前文件遵循该顺序。
- **代码风格**: 早返回、最小侵入修改、`cx.notify()`/`cx.emit()` 驱动 UI 更新。

### 3. 可复用组件清单
- `DbConnectionSelector::emit_selection`：统一发射选择变更事件。
- `DbConnectionSelector::selection_label`：统一生成触发器文案。
- `gpui_component::button::Button`：支持 `.icon(...)`、`.ghost()`、`.xsmall()` 图标按钮。
- `Popover::new(...).trigger(...).content(...)`：现有弹层承载方式。

### 4. 测试策略
- **测试框架**: `cargo test` + Rust 内联单元测试，仓库已启用 `proptest`。
- **测试模式**: 优先补纯状态单元测试，避免引入复杂 UI 测试依赖。
- **参考文件**: `crates/db_view/src/sql_editor_completion_tests.rs`
- **覆盖要求**: 正常流程、清空后空值状态、清空后标签占位文案、重新选择不受影响。

### 5. 依赖和集成点
- **外部依赖**: `gpui`、`gpui-component`、`rust-i18n`。
- **内部依赖**: `db::GlobalDbState`、`one_core::storage::*`、`chat_panel` 消费 `get_connection_info()`。
- **集成方式**: 通过 `DbConnectionSelectorEvent::SelectionChanged` 和公开查询方法向外提供状态。
- **配置来源**: `GlobalStorageState` 与 `GlobalDbState` 全局状态。
### 6. 技术选型理由
- **为什么用这个方案**: 当前状态本身就是 `Option`，新增统一清空方法成本最低，且能复用现有事件与展示逻辑。
- **优势**: 改动集中、下游兼容、无新增 I/O、符合现有 GPUI 交互模式。
- **劣势和风险**: 需要注意清除按钮不要误触发 Popover 打开；清空后要避免残留 schema 能力标志。

### 7. 关键风险点
- **并发问题**: 异步加载数据库/架构时，需避免旧结果在清空后覆盖新状态。
- **边界条件**: 单连接模式、schema-as-database 模式、已选 schema 但未选 database 的状态切换。
- **性能瓶颈**: 清空操作仅重置内存状态，性能风险低。
- **安全考虑**: 本任务不涉及新增安全逻辑。

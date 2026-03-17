## 项目上下文摘要（table-data-sort-crash）
生成时间：2026-03-17 14:03:28 +0800

### 1. 相似实现分析
- **实现1**: `crates/one_ui/src/edit_table/state.rs:1676`
  - 模式：`EditTableState` 在表头点击时先更新本地 `ColumnSort` 状态，再把排序动作委托给 delegate。
  - 可复用：现有排序状态机 `Default -> Descending -> Ascending -> Default` 和行号列偏移换算。
  - 需注意：delegate 回调发生在 `EditTableState` 自身的 `update` 闭包内，若回调里再次更新同一实体会触发重入保护。

- **实现2**: `crates/ui/src/table/state.rs:934`
  - 模式：通用表格也采用“先切换表头状态，再调用 delegate”的两段式处理。
  - 可复用：说明 `one_ui::edit_table` 的排序骨架与通用表格保持一致，问题不在排序状态机本身。
  - 需注意：业务 delegate 必须避免在回调里同步回写当前表格实体。

- **实现3**: `crates/db_view/src/table_data/data_grid.rs:370`
  - 模式：`DataGrid::apply_column_sort` 会先更新 `filter_editor` 的 `ORDER BY`，然后调用 `load_data_with_clauses` 触发 `self.table.update(...)` 刷新数据。
  - 可复用：排序后的真实查询链路已经完整，修复时应保留这条链路。
  - 需注意：该方法内部会同步更新 `EditTableState`，因此不能在 `EditTableState` 正在更新时直接调用。

- **实现4**: `crates/ui/src/dock/dock.rs:190`
  - 模式：项目内已有在实体更新期间通过 `window.defer(cx, ...)` 延后关联实体更新的写法。
  - 可复用：`window.defer` 适合把“当前事件引发的二次更新”延后到下一拍，规避 GPUI 的重入更新限制。
  - 需注意：延后闭包里应使用克隆后的实体句柄，并处理实体已释放的情况。

### 2. 项目约定
- **命名约定**: Rust 函数与字段使用 `snake_case`，类型使用 `PascalCase`。
- **文件组织**: 表格通用状态在 `one_ui/edit_table`，业务排序/加载逻辑在 `db_view/table_data`。
- **导入顺序**: 先标准库，再本地模块和外部 crate；沿用文件当前风格，不做无关重排。
- **代码风格**: 最小改动、早返回、优先复用既有事件循环与异步调度机制。

### 3. 可复用组件清单
- `crates/one_ui/src/edit_table/state.rs`: `EditTableState::perform_sort`
- `crates/db_view/src/table_data/data_grid.rs`: `DataGrid::apply_column_sort`
- `crates/db_view/src/table_data/results_delegate.rs`: `EditorTableDelegate::perform_sort`
- `crates/ui/src/dock/dock.rs`: `window.defer(cx, ...)` 延后更新模式

### 4. 测试策略
- **测试框架**: Rust 内置 `cargo test`
- **测试模式**: 以 `db_view` 排序相关单元测试和受影响文件编译/格式验证为主
- **参考文件**:
  - `crates/db_view/src/table_data/data_grid.rs` 现有排序 SQL 单元测试
  - `crates/db_view/src/table_data/results_delegate.rs` 现有排序解析单元测试
- **覆盖要求**:
  - 排序 SQL 生成行为不回归
  - 排序子句回填表头图标行为不回归
  - `db_view` 包级测试通过，确认本次延后更新未破坏数据加载链路

### 5. 依赖和集成点
- **外部依赖**: `gpui` 的实体更新与 `window.defer` 事件循环模型
- **内部依赖**: `EditTableState -> EditorTableDelegate -> DataGrid -> TableFilterEditor -> load_data_with_clauses`
- **集成方式**: 表头点击触发 delegate 排序，再由 `DataGrid` 更新 `ORDER BY` 并重新查询
- **配置来源**: `DataGridConfig.usage`、`DataGridConfig.database_type`

### 6. 技术选型理由
- **为什么用这个方案**: 根因是同步重入更新，不是排序 SQL 或查询逻辑错误；因此最小修复应只调整回调时机。
- **优势**: 只改 `results_delegate` 一处，不影响通用表格状态机和已有查询流程。
- **劣势和风险**: 排序动作会延后一拍执行，理论上会比同步触发多一个事件循环 tick，但对用户无可感知影响。

### 7. 关键风险点
- **并发问题**: 若实体在 defer 执行前已销毁，必须允许更新安全失败。
- **边界条件**: 点击不可排序列、缺少 `data_grid` 句柄、列索引越界时应继续早返回。
- **性能瓶颈**: 本次不改变服务端排序与重新查询策略，不新增额外请求。
- **工具说明**: 仓库规范要求优先使用 `desktop-commander`、`context7`、`github.search_code`、`sequential-thinking`，但当前会话未提供这些工具；本次使用本地源码检索与结构化分析替代，并在日志中留痕。

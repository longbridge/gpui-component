## 项目上下文摘要（table-data-sort）
生成时间：2026-03-17 12:37:28 +0800

### 1. 相似实现分析
- **实现1**: `crates/ui/src/table/state.rs:937`
  - 模式：通用表格组件在表头点击后切换 `ColumnSort`，并把排序动作委托给 delegate。
  - 可复用：`perform_sort` 的状态切换顺序 `Default -> Descending -> Ascending -> Default`。
  - 需注意：UI 层只负责切图标和分发事件，不直接处理业务查询。

- **实现2**: `crates/one_ui/src/edit_table/state.rs:1676`
  - 模式：可编辑表格复用通用表格的排序状态机，但会处理行号列偏移。
  - 可复用：`delegate_col_ix` 映射逻辑和表头排序图标渲染。
  - 需注意：如果业务 delegate 不实现 `perform_sort`，点击只会改本地状态，不会触发数据刷新。

- **实现3**: `crates/db_view/src/table_data/data_grid.rs:347`
  - 模式：表格数据浏览统一通过 `load_data_with_clauses` 读取 `WHERE` 和 `ORDER BY` 编辑器内容，再下发 `TableDataRequest`。
  - 可复用：`with_order_by_clause` 请求链路和加载后刷新 delegate 的流程。
  - 需注意：排序真正生效必须把表头事件同步到 `filter_editor.order_by_editor`。

- **实现4**: `crates/db_view/src/table_data/results_delegate.rs:384`
  - 模式：结果委托会在 `update_data` 时把每一列设为 `sortable()`。
  - 可复用：列元数据、列名和数据类型都已经在 delegate 中维护，无需新增状态对象。
  - 需注意：`update_data` 会重建列定义，因此排序后的表头状态需要在刷新后回填。

### 2. 项目约定
- **命名约定**: Rust 方法和函数使用 `snake_case`，类型使用 `PascalCase`。
- **文件组织**: 事件分发在 `data_grid.rs`，数据行为在 `results_delegate.rs`，筛选输入在 `filter_editor.rs`。
- **导入顺序**: 先本模块 `crate::...`，再外部 crate，最后标准库。
- **代码风格**: 早返回、最小范围 helper、通过现有 delegate/编辑器组件串联行为。

### 3. 可复用组件清单
- `crates/db_view/src/table_data/filter_editor.rs`: `TableFilterEditor::get_order_by_clause`
- `crates/db_view/src/table_data/data_grid.rs`: `load_data_with_clauses`
- `crates/db_view/src/table_data/results_delegate.rs`: `update_data`
- `crates/db/src/manager.rs`: `DbManager::get_plugin`
- `crates/db/src/plugin.rs`: `DatabasePlugin::quote_identifier`

### 4. 测试策略
- **测试框架**: Rust 内置 `cargo test`
- **测试模式**: 以纯函数单元测试 + `db_view` 包级回归测试为主
- **参考文件**: `crates/db_view/src/sql_editor_view.rs`、`crates/db_view/src/table_designer_tab.rs` 中已有纯函数测试模式
- **覆盖要求**:
  - 表头排序生成方言正确的 `ORDER BY`
  - 排序子句能解析回表头图标状态
  - `db_view` 整包测试通过，避免影响现有编辑/导出/SQL 结果逻辑

### 5. 依赖和集成点
- **外部依赖**: `db::DbManager` / `DatabasePlugin` 用于数据库方言引用符处理
- **内部依赖**: `EditTableState -> EditorTableDelegate -> DataGrid -> TableFilterEditor`
- **集成方式**: 表头点击触发 delegate `perform_sort`，由 `DataGrid` 写入 `ORDER BY` 编辑器并重新查询
- **配置来源**: `DataGridConfig.database_type` / `DataGridConfig.usage`

### 6. 技术选型理由
- **为什么用这个方案**: 仓库已经有 `ORDER BY` 编辑器和数据库插件方言能力，直接复用能避免重复拼 SQL。
- **优势**: 事件链短、数据库方言正确、和现有筛选/分页请求保持一致。
- **劣势和风险**: 只回填首个排序列的图标；复杂手写 `ORDER BY` 子句不会完整映射到多列表头状态。

### 7. 关键风险点
- **并发问题**: 排序会触发重新加载，若用户在加载中再次点击可能出现重复请求；当前沿用既有加载流程。
- **边界条件**: 列名包含关键字、空格、引号时必须使用数据库插件做标识符引用。
- **性能瓶颈**: 排序基于服务端重新查询，不在前端做大表本地排序。
- **工具说明**: 当前会话没有 `desktop-commander`、`context7`、`github.search_code`，本次使用本地源码检索和既有 crate 实现完成上下文分析。

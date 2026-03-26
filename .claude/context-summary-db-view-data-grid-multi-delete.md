## 项目上下文摘要（db-view-data-grid-multi-delete）
生成时间：2026-03-26 19:47:33 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/table_data/data_grid.rs:1077`
  - 模式：删除按钮入口在 `DataGrid` 层统一转发到底层 `EditTableState`。
  - 可复用：现有 `state.delete_row(row_ix, window, cx)` 删除链路已经完整。
  - 需注意：原实现只读 `selected_row()/selected_cell()`，只能删除最后活动行。

- **实现2**: `crates/one_ui/src/edit_table/state.rs:532`
  - 模式：`EditTableState` 的多选真实状态存于 `selection()`，并通过 `SelectionChanged` 事件同步。
  - 可复用：`selection().all_cells()` 可稳定展开当前多选区里的全部单元格。
  - 需注意：`sync_legacy_selection` 只把 `active` 单元格同步到旧字段，不能用来做批量删除。

- **实现3**: `crates/one_ui/src/edit_table/selection.rs:94`
  - 模式：`TableSelection` 使用 `ranges` 表示多选，`all_cells()` 会展开并去重所有坐标。
  - 可复用：从单元格集合映射到唯一行集合即可支持批量行删除。
  - 需注意：跨列多选时同一行会出现多次，删除前必须先按行去重。

- **实现4**: `crates/db_view/src/table_data/results_delegate.rs:1955`
  - 模式：新建未保存行删除时会真实移除并重建索引，已有行删除时只标记为 `Deleted`。
  - 可复用：继续复用 `on_row_deleted` 的既有语义，不改 delegate 接口。
  - 需注意：批量删除必须按降序处理，避免真实移除的新行导致后续索引前移。

- **实现5**: `crates/db_view/src/database_objects_tab.rs:601`
  - 模式：仓库已有“收集全部选中项 -> 排序 -> 批量处理”的多选操作模式。
  - 可复用：本次删除入口也沿用先收集、再统一处理的方式。
  - 需注意：批量动作前先规范化选中集合，避免重复处理。

### 2. 项目约定
- **命名约定**: Rust 私有辅助函数使用 `snake_case`，测试名直接描述行为。
- **文件组织**: 通用表格状态在 `crates/one_ui/src/edit_table`，业务删除入口在 `crates/db_view/src/table_data/data_grid.rs`。
- **导入顺序**: 沿用文件现有顺序，不为本次补丁做无关重排。
- **代码风格**: 最小改动、优先复用既有 API、边界条件用早返回或空集合处理。

### 3. 可复用组件清单
- `crates/db_view/src/table_data/data_grid.rs`: `handle_delete_row`
- `crates/one_ui/src/edit_table/state.rs`: `selection()`、`selected_row()`、`selected_cell()`、`delete_row()`
- `crates/one_ui/src/edit_table/selection.rs`: `TableSelection::all_cells()`
- `crates/db_view/src/table_data/results_delegate.rs`: `EditorTableDelegate::on_row_deleted`
- `crates/db_view/src/database_objects_tab.rs`: `build_nodes_for_selected_rows`

### 4. 测试策略
- **测试框架**: Rust 内置 `cargo test`
- **测试模式**: 文件内纯函数单元测试 + `db_view` crate 全量单元测试
- **参考文件**:
  - `crates/db_view/src/table_data/data_grid.rs:2492`
  - `crates/db_view/src/table_data/results_delegate.rs` 现有删除语义实现
- **覆盖要求**:
  - 多选行索引去重且按降序输出
  - 无多选区时正确回退到单行删除
  - 显式选区存在时不误用 fallback 行

### 5. 依赖和集成点
- **外部依赖**: 无新增外部依赖
- **内部依赖**: `DataGrid -> EditTableState -> EditorTableDelegate`
- **集成方式**: 删除按钮从 `selection().all_cells()` 提取待删行，再复用 `state.delete_row`
- **配置来源**: 无新增配置，完全复用既有表格状态

### 6. 技术选型理由
- **为什么用这个方案**: 根因是删除入口读取了单选兼容字段，而不是多选真实状态；因此应在 `DataGrid` 层修正选区到行集合的映射。
- **优势**: 改动面小，不侵入 `one_ui` 公共状态层，不改变 `results_delegate` 既有删除语义。
- **劣势和风险**: 依赖 `all_cells()` 展开选区，理论上在超大矩形选区时会遍历更多单元格，但删除操作是低频交互，可接受。

### 7. 关键风险点
- **并发问题**: 无新增异步或共享状态。
- **边界条件**: 选中多个单元格但来自同一行时必须去重；没有多选区时要保留单行删除能力。
- **性能瓶颈**: 多选删除前会做一次排序去重，复杂度约为 `O(n log n)`，仅在点击删除时触发。
- **外部资料说明**: 本次问题完全由仓库内部选区与删除语义决定，未涉及额外库 API 争议，因此未调用 Context7；仍按规范补做了 `github.search_code` 检索，用于确认“批量删除先规范化选中集合”的通用做法。

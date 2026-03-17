## 项目上下文摘要（table-data-horizontal-scroll）
生成时间：2026-03-17 14:56:57 +0800

### 1. 相似实现分析
- **实现1**: `crates/ui/src/table/state.rs:435`
  - 模式：通用表格 `set_selected_cell` 在设置活动单元格时直接同步滚动句柄，保证键盘导航和可视区域一致。
  - 可复用：选中单元格与滚动同步应该在同一条状态链路里完成，而不是依赖间接副作用。
  - 需注意：通用表格没有 `EditTable` 的多选区兼容层，因此不能原样照搬，只能借鉴“显式滚动”思路。

- **实现2**: `crates/one_ui/src/edit_table/state.rs:377`
  - 模式：`EditTable` 的键盘导航统一经过 `select_cell_for_navigation`，当前仅显式做了纵向 `scroll_to_item(..., Center)`，横向滚动依赖 `select_cell -> sync_legacy_selection -> scroll_to_col` 的间接调用。
  - 可复用：可以在导航专用路径里显式补齐横向滚动，避免依赖旧兼容层的副作用。
  - 需注意：当前用户已确认上下移动没有问题，问题集中在左右移动后的横向可视区域同步。

- **实现3**: `crates/one_ui/src/edit_table/state.rs:1639`
  - 模式：列宽拖拽时使用 `horizontal_scroll_handle.set_offset` 直接调整水平滚动偏移。
  - 可复用：说明 `EditTable` 已有“直接写入滚动偏移”的先例，可复用于键盘导航后的横向可见性保障。
  - 需注意：该逻辑依赖 `bounds` 与 `col_group.bounds` 的实时位置。

- **实现4**: `crates/ui/src/virtual_list.rs:248`
  - 模式：`VirtualListScrollHandle::scroll_to_item` 在水平方向会根据目标项边界修正 `scroll_offset.x`，前提是正确写入目标列索引。
  - 可复用：当列边界尚未可用时，仍可作为回退方案。
  - 需注意：这套逻辑属于 defer/prepaint 机制，调用方必须在状态变更后保持一次刷新通知。

### 2. 项目约定
- **命名约定**: Rust 方法和字段使用 `snake_case`，类型使用 `PascalCase`。
- **文件组织**: 表格导航和滚动状态集中在 `crates/one_ui/src/edit_table/state.rs`。
- **代码风格**: 最小改动、优先复用既有滚动句柄，不引入新的状态字段。
- **导入顺序**: 沿用文件既有顺序，不做无关重排。

### 3. 可复用组件清单
- `crates/one_ui/src/edit_table/state.rs`: `scroll_to_col`、`select_cell_for_navigation`
- `crates/one_ui/src/edit_table/state.rs`: `scroll_table_by_col_resizing`（直接写入水平偏移）
- `crates/ui/src/table/state.rs`: `set_selected_cell`
- `crates/ui/src/virtual_list.rs`: `VirtualListScrollHandle::scroll_to_item`

### 4. 测试策略
- **测试框架**: Rust 内置 `cargo test`
- **测试模式**: 以 `one-ui` 和 `db_view` 包级回归测试为主
- **参考文件**:
  - `crates/one_ui/src/edit_table/state.rs`
  - `crates/ui/src/table/state.rs`
  - `crates/ui/src/virtual_list.rs`
- **覆盖要求**:
  - `EditTable` 改动不破坏现有单元格导航与选择逻辑
  - `one-ui`、`db_view` 包级测试通过
  - 无法自动覆盖的 GUI 左右移动冒烟需在验证报告中留痕

### 5. 依赖和集成点
- **外部依赖**: `gpui` 的 `UniformListScrollHandle` 与 `VirtualListScrollHandle`
- **内部依赖**: `EditTableState -> render_table_row/render_table_header -> track_scroll`
- **集成方式**: 键盘左右移动更新活动单元格后，应显式写入水平滚动目标列
- **配置来源**: `row_number_enabled`、`fixed_left_cols_count`、`col_fixed`

### 6. 技术选型理由
- **为什么用这个方案**: `EditTable` 的水平滚动容器使用 `overflow_hidden`，`scroll_to_item` 可能无法驱动横向偏移。改为基于 `col_group.bounds` 与表格视口计算最小偏移量，直接 `set_offset`，更符合现有拖拽滚动模式。
- **优势**: 修改范围小，只影响 `EditTableState` 的滚动同步逻辑，且保留 `scroll_to_item` 作为 bounds 不可用时的回退。
- **劣势和风险**: 依赖 `bounds` 的实时性；初次渲染或列宽尚未测量时仍可能需要下一帧刷新才能准确对齐，需要桌面环境冒烟确认。

### 7. 关键风险点
- **并发问题**: 本次不新增异步任务，不涉及重入更新。
- **边界条件**: 需要兼容行号列、固定列和普通滚动列三种索引情况。
- **性能瓶颈**: 仅在活动列越界时写入一次水平偏移，不引入额外渲染或数据请求。
- **工具说明**: 仓库规范要求优先使用 `desktop-commander`、`context7`、`github.search_code`、`sequential-thinking`，但当前会话未提供这些工具；本次改用本地源码检索与结构化分析替代，并在日志中留痕。

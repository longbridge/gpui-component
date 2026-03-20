## 项目上下文摘要（table-data-tab-navigation）
生成时间：2026-03-17 11:54:39 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/table_data/data_grid.rs:182`
  - 模式：表记录页使用 `one_ui::edit_table::EditTableState<EditorTableDelegate>` 作为核心表格状态。
  - 可复用：`DataGrid` 只订阅 `EditTableEvent`，真正的键盘行为应落在 `EditTableState` 和 delegate 输入构建上。
  - 需注意：这里不是 `crates/ui/src/table`，不能直接假设已有 Tab 键逻辑可用。

- **实现2**: `crates/one_ui/src/edit_table/state.rs:1048`
  - 模式：`EditTableState` 已有上下左右对应的动作处理，但当前实现主要按“整行/整列”更新 `selected_row/selected_col`。
  - 可复用：现成的 `set_selected_cell`、`editing_cell`、`commit_cell_edit` 生命周期。
  - 需注意：当前 `action_select_prev_col/action_select_next_col` 没有按单元格模式移动，也未正确处理行号列偏移。

- **实现3**: `crates/ui/src/table/state.rs:619`
  - 模式：通用 `ui::Table` 已经在单元格模式下，把方向键与 `tab/shift-tab` 统一映射到“同一行/列内移动单元格”。
  - 可复用：按单元格模式优先、否则回退到行/列选择模式的分支写法。
  - 需注意：这是仓库内最接近目标行为的标准样板。

- **实现4**: `crates/db_view/src/table_data/results_delegate.rs:1236`
  - 模式：表格编辑器由 delegate 按字段类型创建 `InputState` / 日期时间选择器，并用订阅处理 `PressEnter` / `Blur`。
  - 可复用：在 cell editor 内通过订阅提交编辑；必要时配合外层表格动作完成“提交后切换单元格”。
  - 需注意：普通文本编辑器当前使用 `multi_line(true).rows(1)`，会把 `Tab` 解释为缩进而非导航。

### 2. 项目约定
- **命名约定**: Rust 类型使用 `PascalCase`，函数和字段使用 `snake_case`
- **文件组织**: 表格通用交互放在 `crates/one_ui/src/edit_table/`，业务侧输入策略放在 `crates/db_view/src/table_data/results_delegate.rs`
- **导入顺序**: 先标准库，再外部依赖，最后本地模块；同层 `use` 维持紧凑分组
- **代码风格**: 行为修复优先复用现有状态机和 action handler，不新增平行实现

### 3. 可复用组件清单
- `crates/one_ui/src/edit_table/state.rs`: `set_selected_cell`、`commit_cell_edit`、`editing_cell`
- `crates/ui/src/table/state.rs`: 单元格模式键盘导航参考实现
- `crates/db_view/src/table_data/results_delegate.rs`: 单元格编辑输入创建与提交订阅模式
- `crates/one_ui/src/edit_table/mod.rs`: `EditTable` 级别键盘绑定入口

### 4. 测试策略
- **测试框架**: Rust 内置单元测试（`#[cfg(test)]` / `#[test]`）
- **参考文件**: `crates/one_ui/src/edit_table/selection.rs:292`
- **覆盖要求**: 本次至少做包级编译/单测验证；图形界面的 Tab 导航需要桌面环境手工冒烟

### 5. 依赖和集成点
- **外部依赖**: `gpui::KeyBinding`、`gpui_component::input::{InputState, IndentInline, OutdentInline}`
- **内部依赖**: `EditTableState`、`EditorTableDelegate::build_input`
- **集成方式**: 非编辑态通过 `EditTable` 键绑定触发列导航；编辑态通过输入动作传播到外层表格完成提交与单元格切换
- **配置来源**: 无新增配置

### 6. 技术选型理由
- **为什么复用 `ui::Table` 模式**: 这是仓库内已验证的单元格键盘导航行为，风险最低
- **为什么同时改 `EditTableState` 与 delegate 输入**: 一个负责导航状态机，一个负责编辑态 Tab 不被输入框吞掉，缺一不可
- **为什么不改全局 Input 组件默认行为**: 全局输入框大量复用，改默认 Tab 语义会带来不必要的连锁回归

### 7. 关键风险点
- **行号列偏移风险**: `EditTable` 支持行号列，单元格列索引与 delegate 列索引不同，导航时必须跳过行号列
- **编辑态风险**: 文本输入、日期时间输入和数字输入的 Tab 行为不完全一致，必须统一到表格导航
- **验证风险**: 当前终端环境无法自动点击 UI，只能通过本地测试和代码路径分析保证正确性
- **工具约束**: 仓库要求优先用 `desktop-commander`、`context7`、`github.search_code`，但当前会话未提供这些工具，本次改用本地源码检索与依赖源码对照留痕

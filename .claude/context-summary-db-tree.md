## 项目上下文摘要（数据库节点双击添加树视图）
生成时间：2026-03-10 23:30:00

### 1. 相似实现分析
- **实现1**: crates/db_view/src/database_objects_tab.rs:200-360
    - 模式：列表行事件 -> 根据 `db_node_type` 构造 `DbNode` -> 通过 `DatabaseObjectsEvent` 通知 `DatabaseEventHandler`
    - 可复用：`handle_row_double_click` 的筛选与 `cx.emit` 机制、`DatabaseObjectsEvent` 枚举扩展点
    - 需注意：当前分支仅覆盖表/视图/查询，数据库节点被 `return`，需要遵循同样的 `DbNode` 构造逻辑避免重复
- **实现2**: crates/db_view/src/db_tree_view.rs:840-1180 & 1470-1770
    - 模式：`DbTreeView` 持有 `db_nodes` + `selected_databases`，通过 `add_database_to_selection` 持久化筛选，再用 `add_database_node`/`rebuild_tree` 更新 UI
    - 可复用：`save_database_filter` 内部 `ConnectionRepository` 写入、`lazy_load_children`/`expanded_nodes` 控制展开状态
    - 需注意：所有状态变更后都要 `cx.notify()` 或 `rebuild_tree`，否则 UniformList 不刷新
- **实现3**: crates/db_view/src/db_tree_event.rs:0-420
    - 模式：`DatabaseEventHandler` 同时订阅树视图与 objects panel 事件，集中路由到具体 handler
    - 可复用：objects -> handler -> tree_view 的调用链，可为新增事件添加对 `tree_view.update` 的封装
    - 需注意：事件 handler 需要克隆 `Entity`，并在异步 context 中保持 `global_state`/`window` 可用

### 2. 项目约定
- **命名约定**: Rust 模块/函数使用 snake_case，事件枚举使用 PascalCase；文件多以业务域命名（database_objects_tab / db_tree_view）
- **文件组织**: db_view crate 将 UI 子模块分散到独立文件，并通过 `DatabaseEventHandler` 统一胶水；`.claude/` 存放上下文/日志
- **导入顺序**: 先标准库 -> 外部 crate -> 当前 crate，且按字母或功能分组加注释
- **代码风格**: gpui fluent builder + `cx.listener`，逻辑重度依赖 `cx.spawn`、`cx.emit`、`cx.notify`，注释均为简体中文

### 3. 可复用组件清单
- `DbTreeView::add_database_to_selection`（crates/db_view/src/db_tree_view.rs:868）：更新已选数据库并触发持久化
- `DbTreeView::add_database_node`（同文件:1732）：直接向 `db_nodes` 插入数据库节点并重建树
- `DbTreeView::lazy_load_children` / `expanded_nodes`：控制节点展开与懒加载
- `DatabaseEventHandler`（crates/db_view/src/db_tree_event.rs）: 集中处理 `DatabaseObjectsEvent` 与 `DbTreeViewEvent`
- `ConnectionRepository::set_selected_databases`：通过 `GlobalStorageState` 存储筛选结果

### 4. 测试策略
- **测试框架**: 原生 Rust `#[test]`，集中放在文件末尾 `#[cfg(test)] mod tests`
- **测试模式**: 偏向纯函数/SQL 生成的断言，例如 `table_designer_tab.rs:2971+`
- **参考文件**: crates/db_view/src/table_designer_tab.rs:2971-3100 展示了 builder + helper + 多数据库断言
- **覆盖要求**: 同时验证多数据库方言、多分支逻辑，使用中文断言提示说明预期

### 5. 依赖和集成点
- **外部依赖**: `gpui` + `gpui_component` 提供 UniformList、输入框、图标等控件；`one_core::storage` 提供 `GlobalStorageState`、`ConnectionRepository`
- **内部依赖**: `GlobalDbState` 负责数据库连接配置、节点加载、SQL 执行；`DatabaseViewPluginRegistry` 根据 `DatabaseType` 构造上下文菜单与工具栏
- **集成方式**: `DatabaseTabView` 创建 `DbTreeView` + `DatabaseObjectsPanel`，`DatabaseEventHandler` 通过订阅桥接双方
- **配置来源**: 连接/筛选配置存于 `ConnectionRepository`，节点缓存通过 `db::GlobalNodeCache`

### 6. 技术选型理由
- **为什么用这个方案**: gpui/UniformList 支持高性能懒加载树与列表；事件统一走 `Entity` + `cx.emit`，便于分离 UI 与业务
- **优势**: 统一事件流、可复用的节点模型 (`DbNode`)、`GlobalDbState` 负责 I/O，UI 层无需直接操作数据库
- **劣势和风险**: 状态同步依赖多层异步（storage 写入、global_state 更新、UI rebuild），缺少事务保证；数据库筛选/树节点状态存在双份真相

### 7. 关键风险点
- **并发问题**: `add_database_to_selection` 异步保存后没有回调，若 UI 立即展开可能与持久化失败不一致
- **边界条件**: 连接未加载/children 未加载时直接添加节点可能失败，需要判空并降级
- **性能瓶颈**: `rebuild_tree` 对所有节点排序，频繁触发可能影响大型连接树
- **安全考虑**: 依据 CLAUDE.md，安全需求优先级低但仍需防止误删连接/数据库的操作链（确认提示保持一致）

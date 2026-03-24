## 项目上下文摘要（db-tree-filter-persist）
生成时间：2026-03-24 18:33:00 +0800

### 1. 相似实现分析
- 实现1：`main/src/home_tab.rs:184`
  - 模式：`HomePage` 只在收到 `ConnectionDataEvent::ConnectionUpdated` 后立即刷新内存 `connections` 列表，并异步 `load_connections` 兜底。
  - 可复用：任何后台修改连接元数据的逻辑，都应复用这条事件链而不是直接假设主页会重读仓库。
  - 需注意：不发事件时，重新打开数据库页会继续使用旧 `StoredConnection`。

- 实现2：`crates/mongodb_view/src/mongo_form_window.rs:638`
  - 模式：连接保存成功后，使用全局 notifier 发 `ConnectionUpdated`/`ConnectionCreated`。
  - 可复用：筛选状态本质也是连接元数据更新，应该沿用同样的通知机制。
  - 需注意：事件发出发生在持久化成功之后，而不是 UI 点击之后。

- 实现3：`crates/db_view/src/database_tab.rs:61`
  - 模式：数据库页构造函数 `new_with_active_conn` 直接依赖 `connections: Vec<StoredConnection>` 来创建 `DbTreeView::new`。
  - 可复用：说明重新进入数据库页时的数据源就是主页内存列表。
  - 需注意：如果主页列表没更新，筛选状态再怎么写库也不会立即反映到新页面。

- 实现4：`crates/db_view/src/db_tree_view.rs:964`
  - 模式：`save_database_filter` 已经能把 `selected_databases` 写回 `ConnectionRepository`。
  - 可复用：不需要新增存储字段或改表结构，只需补成功写库后的状态同步。
  - 需注意：当前实现缺少事件广播，且 `update_connection_info` 也没有同步 `selected_databases`。

### 2. 调用链与集成点
- 勾选切换：`crates/db_view/src/db_tree_view.rs:843`
  - `toggle_database_selection` / `select_all_databases` / `deselect_all_databases` 最终都会调用 `save_database_filter`。
- 持久化写入：`crates/db_view/src/db_tree_view.rs:964`
  - 通过 `ConnectionRepository::get -> StoredConnection::set_selected_databases -> repo.update` 更新仓库。
- 存储模型：`crates/core/src/storage/models.rs:798`
  - `StoredConnection::get_selected_databases/set_selected_databases` 已支持 JSON 字符串与 `Vec<String>` 互转。
- 重新进入数据页：`crates/db_view/src/database_tab.rs:67`
  - 使用外部传入的 `connections` 创建树视图。

### 3. 项目约定
- 命名约定：Rust 函数/变量使用 `snake_case`，事件枚举使用 `PascalCase`。
- 状态同步约定：连接配置变更优先走 `ConnectionDataEvent` 广播，主页负责维护连接内存列表。
- 文件组织：`db_tree_view.rs` 管 UI 和本地筛选状态；`home_tab.rs` 持有主页连接内存；`storage/repository.rs` 负责落库。

### 4. 可复用组件清单
- `one_core::connection_notifier::emit_connection_event`
- `ConnectionDataEvent::ConnectionUpdated`
- `StoredConnection::set_selected_databases`
- `ConnectionRepository::update`

### 5. 测试策略
- 测试框架：Rust 原生 `#[test]`
- 参考模式：`crates/db_view/src/db_tree_view.rs` 现有纯逻辑测试模块
- 本次策略：
  - 提取/复用纯函数同步 `selected_databases` 映射
  - 为“有筛选”和“恢复全选”两种状态各补 1 个单测
  - 执行最小范围 `cargo test -p db_view sync_selected_databases_from_connection --lib`

### 6. 技术选型理由
- 为什么发 `ConnectionUpdated`：
  - 事实：主页只监听连接更新事件来刷新 `connections`。
  - 事实：数据库页重建时读取的是主页 `connections`。
  - 推论：写库成功后补发连接更新事件，是最小且一致的修复方式。
- 优势：
  - 不改数据库表结构
  - 不引入新的全局状态
  - 与现有连接表单保存行为一致
- 风险：
  - 需要同时保证 `DbTreeView` 自身也能从更新事件同步 `selected_databases`，避免其他已打开标签页仍保留旧筛选。

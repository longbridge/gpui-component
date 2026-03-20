## 项目上下文摘要（db_tree_view 刷新缓存失效）
生成时间：2026-03-20 15:39:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/db_tree_view.rs:1059-1096`
  - 模式：`refresh_tree` 先定位实际刷新节点，再异步调用 `GlobalNodeCache::invalidate_node_recursive`，随后立即清本地树状态并触发 `lazy_load_children`
  - 可复用：`clear_node_descendants`、`clear_node_loading_state`、`reset_node_children`
  - 需注意：缓存失效是 `Tokio::spawn(...).detach()`，当前重新加载不会等待缓存删除完成，存在旧缓存被再次命中的竞态

- **实现2**: `crates/db_view/src/db_tree_view.rs:1778-1805`
  - 模式：`close_connection` 在 UI 状态清理前，显式同时处理 `clear_connection_cache` 和 `invalidate_connection_metadata`
  - 可复用：连接级刷新需要同时考虑节点缓存和元数据缓存两层
  - 需注意：这里同样是异步清理，但关闭连接不会马上重新加载，因此没有“删缓存前先读缓存”的竞态

- **实现3**: `crates/db/src/cache_manager.rs:445-463`
  - 模式：`process_sql_for_invalidation` 先同步失效 metadata cache，再同步清空 node cache，最后才返回 `SchemaChanged`
  - 可复用：当后续动作依赖新数据时，缓存失效必须先完成，再触发 UI 刷新
  - 需注意：这是当前仓库里最明确的“失效先于刷新”模式

- **实现4**: `crates/db/src/cache.rs:269-298`
  - 模式：`invalidate_node_recursive` 递归读取缓存节点并删除当前节点与全部后代缓存
  - 可复用：节点级刷新应沿用这个递归失效入口，避免只删父节点导致子层仍旧命中
  - 需注意：该过程包含异步文件删除；如果不等待完成，随后的读取仍可能命中文件缓存

### 2. 项目约定
- **命名约定**: Rust 函数和局部变量使用 `snake_case`，事件与类型使用 `PascalCase`
- **文件组织**: `db_tree_view.rs` 持有树 UI 状态，`cache.rs`/`cache_manager.rs` 负责节点缓存与元数据缓存
- **导入顺序**: 标准库、第三方 crate、当前 crate 分组
- **代码风格**: UI 刷新通过 `cx.spawn` / `this.update` 回到主线程，状态变更后用 `rebuild_tree` 或 `cx.notify`

### 3. 可复用组件清单
- `DbTreeView::clear_node_descendants`：清空树内存中的后代节点
- `DbTreeView::reset_node_children`：重置父节点 `children` 和 `children_loaded`
- `db::GlobalNodeCache::invalidate_node_recursive`：递归失效节点缓存
- `db::GlobalNodeCache::invalidate_database`：数据库级元数据缓存失效
- `db::GlobalNodeCache::invalidate_connection_metadata`：连接级元数据缓存失效

### 4. 测试策略
- **测试框架**: Rust 原生 `#[test]`
- **参考实现**: `crates/db_view/src/sql_editor_completion_tests.rs` 展示了在 `db_view` crate 中添加纯逻辑测试的方式
- **本次策略**: 提取不依赖 UI 上下文的刷新失效范围判断逻辑，补充纯函数测试，覆盖连接/数据库/表视图节点的缓存失效范围

### 5. 依赖和集成点
- **外部依赖**: `gpui` 负责异步 UI 更新，`moka::sync::Cache` 负责内存缓存
- **内部依赖**:
  - `GlobalDbState::load_node_children` 读取树节点缓存
  - `DatabasePlugin::load_node_children` 负责真正查询数据库
  - `GlobalNodeCache` 同时承载节点缓存和元数据缓存
- **集成方式**: 右键菜单触发 `DbTreeView::refresh_tree`，自动 DDL 刷新也走同一入口

### 6. 技术选型理由
- **为什么用这个方案**: 保持现有 `refresh_tree` 作为唯一刷新入口，避免把手工刷新和 DDL 自动刷新分叉成两套逻辑
- **优势**: 改动集中、行为一致、不会影响正常懒加载路径
- **劣势和风险**: 需要小心避免在异步失效期间重复触发加载，导致状态抖动

### 7. 外部参考
- **Context7 / moka 文档**: `sync::Cache::invalidate` 失效后可立即 `get == None`，说明内存层失效可同步观察；但仓库当前还有文件缓存层，所以手动刷新仍需等待文件缓存删除完成
- **GitHub 代码搜索**: 搜索 `language:Rust "invalidate" "reload" cache refresh`，结果较分散，但公共模式一致是“先失效，再重新装载”，未见“后台失效同时立即 reload”作为推荐做法

### 8. 关键风险点
- **竞态问题**: 刷新后立即调用 `lazy_load_children`，可能在缓存文件删完前重新读到旧节点
- **边界条件**: 连接节点、数据库节点、表/视图节点需要不同的元数据失效范围
- **性能影响**: 连接级刷新若一律清全连接缓存，会比节点级更重，但符合用户显式刷新预期

## 项目上下文摘要（db-tree-refresh-tokio-runtime）
生成时间：2026-03-25 10:45:01

### 1. 相似实现分析
- **实现1**: [db_tree_view.rs](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L969)
  - 模式：`cx.spawn` 内通过 `Tokio::spawn_result` 执行存储层后台任务，再回 UI 线程发事件。
  - 可复用：`one_core::gpui_tokio::Tokio`
  - 需注意：GPUI 异步上下文不等于 Tokio runtime。

- **实现2**: [db_connection_form.rs](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L1350)
  - 模式：数据库连通性检测放进 `Tokio::spawn_result`，避免在 UI executor 直接执行依赖 Tokio 的异步 I/O。
  - 可复用：`Tokio::spawn_result` + `cx.update(...)`
  - 需注意：后台任务结束后再更新界面状态。

- **实现3**: [gpui_tokio.rs](/Users/hufei/RustroverProjects/onetcli/crates/core/src/gpui_tokio.rs#L52)
  - 模式：项目统一 Tokio runtime 包装，使用 `GlobalTokio` 持有 runtime handle。
  - 可复用：`Tokio::spawn`、`Tokio::spawn_result`
  - 需注意：所有依赖 Tokio scheduler/reactor 的 future 都应通过这里调度。

- **实现4**: [cache.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/cache.rs#L277)
  - 模式：`NodeCache` 内部用 `tokio::fs::remove_file/remove_dir_all` 处理缓存文件。
  - 可复用：无需改接口，直接修正调用侧运行时。
  - 需注意：直接在非 Tokio runtime 上 await 会 panic。

### 2. 项目约定
- **命名约定**: Rust `snake_case`，日志信息使用简体中文。
- **文件组织**: UI 调度放在 `db_view`，Tokio runtime 封装在 `one_core`，缓存实现放在 `db`。
- **导入顺序**: 先标准库，再外部 crate，再当前 crate。
- **代码风格**: 保持最小侵入，优先复用现有 helper，不重构无关模块。

### 3. 可复用组件清单
- `one_core::gpui_tokio::Tokio`：把 future 调度到项目共享 Tokio runtime。
- `db::GlobalNodeCache`：缓存和元数据失效入口。
- `gpui::Context::spawn` / `AsyncApp::update`：后台任务结束后回 UI 线程更新树。

### 4. 测试策略
- **测试框架**: Cargo 单元测试。
- **测试模式**: 先做 `cargo check -p db_view`，再跑 `db_tree_view` 现有回归测试。
- **参考文件**: [db_tree_view.rs](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L2673)
- **覆盖要求**: 至少确认编译通过，且已有筛选相关逻辑测试无回归。

### 5. 依赖和集成点
- **内部依赖**: `db_tree_view -> GlobalNodeCache -> NodeCache(tokio::fs)`。
- **集成方式**: 外层 GPUI task 协调 UI 状态，内层 Tokio task 执行缓存 I/O。
- **配置来源**: `GlobalDbState.get_config(connection_id)` 构造 `CacheContext`。

### 6. 技术选型理由
- **为什么用这个方案**: 问题根因是运行时上下文错误，而不是缓存 API 设计错误；修调用侧最小、最稳。
- **优势**: 复用现有双执行器模式，不改缓存层接口。
- **劣势和风险**: 若未来还有其他在 GPUI executor 中直接 await Tokio I/O 的路径，仍需继续排查。

### 7. 关键风险点
- **边界条件**: `cache` 或 `cache_ctx` 为空时不能影响后续 UI 刷新。
- **性能瓶颈**: 递归失效可能涉及较多文件删除，但已在后台执行，不阻塞 UI。
- **并发问题**: 刷新期间多次点击可能触发并发任务，但属于原有行为，本次不扩散修改。

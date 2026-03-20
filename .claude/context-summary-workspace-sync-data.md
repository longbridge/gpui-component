## 项目上下文摘要（workspace-sync-data）
生成时间：2026-03-20 16:04:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/core/src/cloud_sync/engine.rs:72-82`
  - 模式：`SyncEngine::new` 固定注册 `WorkspaceSyncType` 和 `ConnectionSyncHandler`
  - 结论：同步引擎设计上明确支持工作区和连接两类数据同时进入同步流程

- **实现2**: `crates/core/src/cloud_sync/workspace_sync.rs:13-111`
  - 模式：`WorkspaceSyncType::data_type()` 返回 `"workspace"`，并通过 `prepare_workspace_sync_data_upload` / `decrypt_sync_data_workspace` 处理工作区 blob
  - 结论：云端 `sync_data` 中工作区对应的 `data_type` 是 `workspace`

- **实现3**: `main/src/home_tab.rs:216-240`
  - 模式：连接事件会在已登录且主密钥已解锁时自动 `trigger_sync(cx)`，但工作区事件原先只 `load_workspaces(cx)`
  - 结论：工作区缺的是“自动触发同步入口”，不是 `sync_data` 模型缺失

### 2. 关键事实
- `CloudSyncData` 文档和结构定义明确说明 `sync_data` 用 `data_type` 区分 `"connection"` 与 `"workspace"`
- `generic_sync` 会按 `handler.data_type()` 过滤云端数据，因此工作区和连接分别走独立同步批次
- `home_tab` 当前已存在 `refresh_local_home_data()`，说明首页本地展示本身依赖工作区和连接双重刷新

### 3. 根因判断
- **事实**：工作区事件未像连接事件那样自动调用 `trigger_sync(cx)`
- **推论**：如果用户依赖自动同步而没有手动点击同步，`sync_data` 中很可能看不到新增或修改后的 `workspace` 记录

### 4. 修复策略
- 仅在 `main/src/home_tab.rs` 的 `WorkspaceCreated/Updated/Deleted` 分支复用连接事件现有的自动同步条件和日志
- 不改 `SyncEngine`、`WorkspaceSyncType`、`CloudSyncData` 或仓储层

### 5. 验证策略
- 本地执行 `cargo check -p main`
- 在最终说明中明确：如需验证云端结果，应在工作区新增/重命名后观察是否自动产生 `data_type=workspace` 的 `sync_data` 记录

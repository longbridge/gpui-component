## 项目上下文摘要（home-sync-refresh）
生成时间：2026-03-08 11:55:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/home_tab.rs:232`
  - 模式：`load_workspaces` 异步读取仓库后通过 `this.update(..., cx.notify())` 刷新 UI
  - 可复用：工作区重载入口 `load_workspaces`
  - 需注意：刷新动作封装在 UI 层，不依赖同步引擎回调事件

- **实现2**: `main/src/home_tab.rs:252`
  - 模式：`load_connections` 与工作区重载同构，作为连接列表统一重载入口
  - 可复用：连接重载入口 `load_connections`
  - 需注意：连接创建/更新/删除事件都先局部更新，再走该入口兜底一致性

- **实现3**: `main/src/home_tab.rs:1611`
  - 模式：手动刷新按钮同时调用 `load_workspaces(cx)` 与 `load_connections(cx)`
  - 可复用：首页完整刷新应同时重载工作区和连接
  - 需注意：该模式证明首页显示依赖两份本地数据，而非仅连接列表

- **实现4**: `main/src/home_tab.rs:176`
  - 模式：连接事件触发后立即更新内存，再异步重载列表确保最终一致性
  - 可复用：UI 使用“轻量即时更新 + 异步全量重载”的一致性策略
  - 需注意：工作区事件单独调用 `load_workspaces(cx)`，说明工作区是独立显示维度

### 2. 同步链路证据
- **同步引擎顺序**: `crates/core/src/cloud_sync/engine.rs:52`
  - `SyncEngine::new` 固定注册 `WorkspaceSyncHandler` 与 `ConnectionSyncHandler`
  - 说明一次同步可能同时改动工作区与连接

- **部分失败语义**: `crates/core/src/cloud_sync/engine.rs:124`
  - 各 handler 即使失败也只把错误写入 `result.errors`，最终仍返回 `Ok(SyncResult)`
  - 说明“有错误”不等于“本地没有成功落库”

- **当前缺口**: `main/src/home_tab.rs:342` 与 `main/src/home_tab.rs:540`
  - 常规同步成功、冲突解决成功后都只调用 `load_connections(cx)`
  - 与手动刷新按钮的“双重载”模式不一致

### 3. 项目约定
- **命名约定**: `HomePage` 内部使用 `load_*` 表示异步重载动作，布尔状态使用 `syncing`、`logging_in` 这类清晰命名
- **文件组织**: UI 刷新逻辑放 `main/src/home_tab.rs`，同步执行放 `crates/core/src/cloud_sync/engine.rs`
- **代码风格**: 通过 `cx.spawn(async move |this, cx| ...)` 异步执行，落回 `this.update(cx, |this, cx| ...)` 修改状态并 `cx.notify()`
- **导入顺序**: 现有文件按标准库、三方 crate、项目模块分组

### 4. 根因判断
- **事实**：手动刷新会同时重载工作区和连接；同步引擎会同时同步工作区和连接；部分失败仍可能已有成功落库的数据。
- **推论**：同步成功但 `stats.errors` 非空时，首页只重载连接而不重载工作区，导致 UI 仍基于旧工作区状态过滤/分组，所以看起来只显示部分数据。

### 5. 测试策略
- **测试框架**: 仓库以 Rust 原生 `cargo test`/`cargo check` 为主
- **参考模式**: `crates/core/src/cloud_sync/service.rs` 存在内联测试；本文件暂无现成 UI 单测
- **本次验证**: 以 `cargo fmt --check` + 针对 `main` crate 的 `cargo check -p main` 作为本地可重复验证；若缺少 UI 场景测试，在验证报告中记录补偿方案

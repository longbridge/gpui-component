## 项目上下文摘要（issue5-macos-window-restore）
生成时间：2026-03-28 17:35:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/onetcli_app.rs`
  - 模式：应用级动作统一在 `cx.on_action(...)` 注册，并通过 `cx.defer + window.update(...)` 执行。
  - 可复用：`build_window_toggle_plan`、`minimize_window`、`toggle_fullscreen`。
  - 需注意：现有快捷键动作默认假设存在活动窗口。

- **实现2**: `main/src/main.rs`
  - 模式：应用入口统一在 `app.run(...)` 内注册全局行为和首个主窗口。
  - 可复用：`Application::new()`、`cx.open_window(...)`、入口级事件注册点。
  - 需注意：当前是单主窗口模型，`cx.windows().next()` 可作为恢复回退目标。

- **实现3**: `gpui/src/app.rs`
  - 模式：应用级平台事件通过 `App::on_reopen(...)` 注入。
  - 可复用：`on_reopen`，用于 macOS Dock 激活或重新打开应用时执行回调。
  - 需注意：这条链路不依赖已有窗口继续接收键盘事件。

- **实现4**: `gpui/src/window.rs` + `gpui/src/platform/mac/window.rs`
  - 模式：`Window::activate_window()` 仅调用平台层 `activate()`；macOS 下实际是 `makeKeyAndOrderFront:`。
  - 可复用：`Window::window_handle()` 暴露原生窗口句柄。
  - 需注意：`activate_window()` 不等于 `deminiaturize`，不能单独完成最小化恢复。

### 2. 项目约定
- **命名约定**: 枚举/结构体使用 `PascalCase`，helper 使用 `snake_case`。
- **文件组织**: 应用级快捷键和窗口动作集中在 `main/src/onetcli_app.rs`，应用入口事件集中在 `main/src/main.rs`。
- **导入顺序**: 先普通依赖，再 `cfg(target_os = "macos")` 条件导入。
- **代码风格**: 小粒度 helper + `cx.defer` 异步调度，不把平台分支散落到业务 tab。

### 3. 可复用组件清单
- `main/src/onetcli_app.rs`: `WindowTogglePlan`、`build_window_toggle_plan`
- `main/src/main.rs`: `app.run(...)` 入口注册
- `gpui::App::on_reopen(...)`: 应用重开事件
- `gpui::Window::window_handle()`: 原生句柄访问入口

### 4. 测试策略
- **测试框架**: Rust 单元测试
- **测试模式**: 纯逻辑 helper 测试优先
- **参考文件**: `main/src/onetcli_app.rs` 内嵌测试模块
- **覆盖要求**:
  - 无活动窗口但存在窗口时，恢复计划应命中 fallback 窗口
  - 已有活动窗口时，reopen 不应重复恢复
  - macOS 快捷键仍只绑定 `cmd-m`

### 5. 依赖和集成点
- **外部依赖**: `raw-window-handle`、`cocoa`、`objc`
- **内部依赖**: `main.rs -> onetcli_app.rs -> gpui`
- **集成方式**: `main.rs` 注册 `cx.on_reopen(...)`，回调转发给 `onetcli_app::reopen_last_window`
- **配置来源**: `main/Cargo.toml` 的 macOS target dependencies

### 6. 技术选型理由
- **为什么用这个方案**: `on_reopen` 直接对应 macOS Dock 重新激活语义，绕开“最小化窗口无法接收快捷键”的根因。
- **优势**: 入口稳定、平台语义正确、无需把同一快捷键硬做成全局热键。
- **劣势和风险**: 只能保证“重新激活应用时恢复”，不能保证“同一快捷键在已最小化状态下再次触发”。

### 7. 关键风险点
- **事件触发**: 如果用户不通过 Dock/重新打开应用，而坚持在无活动窗口状态下仅靠快捷键恢复，当前 UI 框架仍可能收不到动作。
- **平台边界**: `deminiaturize` 仅在 macOS 分支使用，其他平台继续走 `activate_window()`。
- **验证缺口**: 当前主要是纯逻辑测试和编译验证，仍缺真实 Dock 点击的桌面冒烟。

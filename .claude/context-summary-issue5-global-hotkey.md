## 项目上下文摘要（issue5-global-hotkey）
生成时间：2026-03-28 18:28:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/onetcli_app.rs`
  - 模式：应用级快捷键集中在 `init(cx)` 中通过 `cx.bind_keys(...)` 注册，并用 `cx.on_action(...)` 转到窗口动作。
  - 可复用：`build_window_toggle_plan`、`restore_window`、测试组织方式。
  - 需注意：这是窗口级输入链路，窗口最小化后不再可靠。

- **实现2**: `main/src/main.rs`
  - 模式：macOS 平台旁路逻辑集中在入口模块，直接用 AppKit 通知恢复最小化窗口。
  - 可复用：`macos_activation_restore` 的 `deminiaturize` / `makeKeyAndOrderFront` 恢复链路。
  - 需注意：入口层更适合承接系统级行为，而不是继续塞进业务视图。

- **实现3**: `examples/system_monitor/src/main.rs`
  - 模式：应用入口按平台分支绑定系统语义快捷键，例如 macOS 用 `cmd-q`，非 macOS 用 `alt-f4`。
  - 可复用：平台差异放在入口初始化期处理的风格。
  - 需注意：仓库本身已经接受“系统动作按平台语义分支”。

- **实现4**: `crates/ui/src/input/state.rs`
  - 模式：大量快捷键通过 `#[cfg(target_os = "macos")]` / `#[cfg(not(target_os = "macos"))]` 成对定义。
  - 可复用：跨平台快捷键条件编译样板。
  - 需注意：本次只改 macOS，不扩散非 macOS 现有绑定。

### 2. 项目约定
- **命名约定**: Rust 类型使用 `PascalCase`，函数和 helper 使用 `snake_case`。
- **文件组织**: 应用入口级平台逻辑放 `main/src/main.rs`，应用内动作放 `main/src/onetcli_app.rs`。
- **导入顺序**: 先普通依赖，再 `cfg(target_os = "macos")` 平台依赖。
- **代码风格**: 倾向小粒度 helper、条件编译分支和纯逻辑单元测试。

### 3. 可复用组件清单
- `main/src/main.rs`: 现有 AppKit 恢复最小化窗口逻辑。
- `main/src/onetcli_app.rs`: 窗口切换计划与应用内绑定测试模式。
- `examples/system_monitor/src/main.rs`: 平台快捷键差异参考。
- `crates/ui/src/input/state.rs`: 快捷键条件编译样板。

### 4. 测试策略
- **测试框架**: Rust 内置单元测试。
- **测试模式**: 优先做纯逻辑 helper 测试，再做 `cargo check`/`cargo test`。
- **参考文件**: `main/src/onetcli_app.rs` 内嵌测试模块。
- **覆盖要求**:
  - macOS 下应用内快捷键不再返回 `cmd-m`
  - 系统级热键事件仅在目标 id 且 `Pressed` 时触发恢复

### 5. 依赖和集成点
- **外部依赖**: `cocoa`、`objc`、`raw-window-handle`、计划新增 `global-hotkey`
- **内部依赖**: `main.rs -> macOS 入口逻辑`，`onetcli_app.rs -> 应用内快捷键`
- **集成方式**: `app.run(...)` 主线程初始化系统热键，触发后直接走 AppKit 恢复链路
- **配置来源**: `main/Cargo.toml` 的 macOS target dependencies

### 6. 技术选型理由
- **事实**: `gpui` 的 `on_reopen` 在 macOS 上仅在没有打开窗口时触发；窗口级 `KeyBinding` 在最小化后收不到事件。
- **事实**: `/tauri-apps/global-hotkey` 官方文档和源码都要求 macOS 在主线程创建 manager，并支持系统级热键。
- **推断**: 本次应把恢复主链路收敛到“主线程全局热键 -> AppKit 恢复窗口”，避免继续依赖 `gpui` 窗口输入。

### 7. 关键风险点
- **热键冲突**: 系统级热键可能与用户已有全局快捷键冲突，注册失败时要允许降级而不是崩溃。
- **线程约束**: manager 必须在 macOS 主线程初始化并持有生命周期。
- **验证缺口**: 自动化可覆盖逻辑与编译，但最终仍需桌面环境冒烟验证“最小化后按系统热键恢复”。

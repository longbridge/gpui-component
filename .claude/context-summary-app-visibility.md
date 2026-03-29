## 项目上下文摘要（app-visibility）
生成时间：2026-03-28 23:20:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/main.rs`
  - 模式：应用入口集中处理平台相关生命周期和系统级行为。
  - 可复用：`app.run(...)` 注册点、`system_hotkey` 的纯函数测试组织方式。
  - 需注意：系统级热键不应和业务视图逻辑混在一起。

- **实现2**: `main/src/onetcli_app.rs`
  - 模式：窗口级动作先做纯逻辑计划，再调用平台相关恢复动作。
  - 可复用：`build_window_toggle_plan` / `build_window_reopen_plan` 的纯函数测试思路。
  - 需注意：GPUI 窗口级快捷键只在窗口活动时可靠，不能替代系统级恢复。

- **实现3**: `examples/system_monitor/src/main.rs`
  - 模式：入口按平台语义绑定不同快捷键。
  - 可复用：`macOS` 和非 `macOS` 的快捷键分支风格。
  - 需注意：平台差异应收敛到入口，而不是扩散到业务组件。

- **实现4**: `main/src/main.rs` 旧 `macos_activation_restore`
  - 模式：通过 AppKit 原生 API 做应用激活、窗口恢复和隐藏。
  - 可复用：`applicationDidBecomeActive` 观察器、`restore_first_minimized_window` 和 `activate_app`。
  - 需注意：这部分应该迁入语义层 crate，而不是继续留在入口文件。

### 2. 项目约定
- **命名约定**: Rust 类型使用 `PascalCase`，函数使用 `snake_case`。
- **文件组织**: 平台能力收敛到独立 crate，入口文件只保留接线。
- **导入顺序**: 普通依赖在前，条件编译依赖在后。
- **代码风格**: 优先纯逻辑测试，小粒度 helper，条件编译分支清晰隔离。

### 3. 可复用组件清单
- `main/src/main.rs`: 系统热键入口接线模式。
- `main/src/onetcli_app.rs`: GPUI 窗口内最小化/恢复策略与测试模式。
- `raw-window-handle 0.6.2`: `Win32WindowHandle`、`XlibWindowHandle`、`XcbWindowHandle` 的原生句柄描述。
- `global-hotkey 0.7.0`: 系统热键注册与事件回调。

### 4. 测试策略
- **测试框架**: Rust 内置单元测试。
- **测试模式**: 优先验证纯逻辑 helper，再跑 `cargo test` / `cargo check`。
- **参考文件**: `main/src/main.rs` 和 `main/src/onetcli_app.rs` 的内嵌测试模块。
- **覆盖要求**:
  - `app_visibility` 的显隐动作纯逻辑保持稳定。
  - `main` 中热键事件过滤只在匹配 id 且按下状态触发。
  - `macOS` 继续验证 `cmd+alt+m`，`Windows` 代码路径只做实现不在当前主机宣称已实测。

### 5. 依赖和集成点
- **外部依赖**:
  - `global-hotkey`：系统级热键注册。
  - `raw-window-handle`：主窗口原生句柄提取。
  - `windows`：Win32 `ShowWindow` / `SetForegroundWindow` / `IsWindowVisible` / `IsIconic`。
- **内部依赖**:
  - `main -> app_visibility`
  - `main -> onetcli_app`
- **集成方式**:
  - 窗口创建时注册主窗口句柄。
  - 系统热键事件回调转发到 `app_visibility`。

### 6. 技术选型理由
- **事实**: `global-hotkey` 文档说明 `macOS` 与 `Windows` 都可以用事件回调处理系统热键。
- **事实**: `raw-window-handle` 提供 `Win32` / `Xlib` / `Xcb` / `Wayland` 句柄分支，但 Linux 后端不统一。
- **推断**: 本轮应优先做 `Windows` 恢复，Linux 先保留句柄注册与扩展位，不伪装成已支持。
- **用户约束**: `Windows/Linux` 的隐藏不必走系统级路径，前台隐藏继续复用 GPUI 自身能力；系统级重点是“恢复”。

### 7. 关键风险点
- **Windows 验证缺口**: 当前开发机只安装了 `aarch64-apple-darwin` 目标，无法本地交叉验证 Windows 编译与运行。
- **Linux 差异**: `X11` 与 `Wayland` 恢复机制不同，不能在没有桌面实测的情况下宣称支持。
- **事件竞态**: 如果系统热键与应用内热键复用同一键位，系统回调必须在窗口可见时尽量不干预，以免抵消本地隐藏动作。

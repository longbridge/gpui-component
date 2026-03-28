## 项目上下文摘要（issue-5）
生成时间：2026-03-28 15:42:29 +0800

### 1. Issue 概况
- **GitHub Issue**: `feigeCode/onetcli#5`
- **标题**: `bug反馈`
- **提交时间**: `2026-03-28 14:24:10 +0800`
- **版本**: `0.1.9`
- **子问题**:
  1. 只有一个首页标签时无法通过顶部拖动窗口
  2. 关于页显示的版本号仍是 `0.1.3`
  3. `ctrl+space` 只能隐藏窗口，无法再次显示
  4. SSH 重连后右侧文件传输窗口空白

### 2. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/core/src/tab_container.rs:1639`
  - 模式：标签栏 `#tabs` 使用 `WindowControlArea::Drag` 与 `should_move -> start_window_move()`。
  - 可复用：`TabBarDragState` 与现有拖动事件链。
  - 需注意：当前仅滚动标签容器启用拖动，单 pinned 首页标签场景可能未完整覆盖。

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/title_bar.rs:252`
  - 模式：标题栏顶层和 `bar` 容器同时配合 `WindowControlArea::Drag`、鼠标按下/移动状态机。
  - 可复用：`TitleBarState.should_move` 的完整拖动写法。
  - 需注意：该实现比 `tab_container` 更稳，适合作为 issue 第 1 条参照。

- **实现3**: `/Users/hufei/RustroverProjects/onetcli/main/src/setting_tab.rs:1011`
  - 模式：About 页直接使用 `env!("CARGO_PKG_VERSION")` 展示版本号。
  - 可复用：继续沿用编译期版本注入，不新增第二版本源。
  - 需注意：真正的版本来源是 `main/Cargo.toml`，当前该文件仍为 `0.1.3`。

- **实现4**: `/Users/hufei/RustroverProjects/onetcli/main/Cargo.toml:1`
  - 模式：应用 crate 版本号是 UI About 页的事实来源。
  - 可复用：仓库已有 `script/bump-version.sh -> cargo set-version` 的标准 bump 流程。
  - 需注意：如果只改 UI 文案，不改 crate 版本，会再次失配。

- **实现5**: `/Users/hufei/RustroverProjects/onetcli/main/src/onetcli_app.rs:96`
  - 模式：`MinimizeWindow` 通过 `cx.active_window()` 拿活跃窗口，再依据 `window.is_window_active()` 选择 `minimize_window()` 或 `activate_window()`。
  - 可复用：现有动作绑定与 `on_action` 注册链路。
  - 需注意：窗口最小化后通常没有活跃窗口，上述逻辑无法作为全局恢复入口。

- **实现6**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/sidebar/file_manager_panel.rs:571`
  - 模式：右侧文件管理器使用独立 SFTP 连接，首次打开时 `connect()`，成功后 `refresh_dir()`。
  - 可复用：现有 `connect()`、`connect_if_idle()`、`set_initial_working_dir()`。
  - 需注意：该面板与独立 `sftp_view` 不是同一实现，issue 第 4 条应修这里。

- **实现7**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/sidebar/mod.rs:317`
  - 模式：文件管理器面板仅在首次打开 `FileManager` 时执行 `connect_if_idle()`。
  - 可复用：侧边栏统一入口与 `TerminalSidebarEvent`。
  - 需注意：终端重连后不会再次触发这里的自动连接。

- **实现8**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/view.rs:984`
  - 模式：SSH 终端重连按钮调用 `TerminalView::reconnect()`，继续下沉到 `Terminal::reconnect()`。
  - 可复用：终端侧已有稳定重连入口。
  - 需注意：当前没有把重连结果同步给 `FileManagerPanel`。

- **实现9**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal/src/terminal.rs:844`
  - 模式：`Terminal::reconnect()` 仅重建 SSH/串口终端后端，并发出 `TerminalModelEvent::Wakeup`。
  - 可复用：终端重连状态切换与 `current_working_dir()`。
  - 需注意：不包含任何文件管理器/SFTP 侧栏恢复逻辑。

### 3. 开源与框架参考
- **开源实现**: `zed-industries/zed/crates/platform_title_bar/src/platform_title_bar.rs`
  - 事实：上游标题栏使用统一 `WindowControlArea::Drag` + `should_move` 状态机，再触发 `window.start_window_move()`。
  - 推论：issue 第 1 条应复用这一稳定模式，而不是新增自定义拖动逻辑。

- **Context7 / GPUI**: `/websites/rs_gpui_gpui`
  - 事实：`WindowControlArea` 明确包含 `Drag/Close/Max/Min`，`Window` 提供窗口控制相关 API。
  - 推论：拖动区域和窗口控制问题都应优先沿用 GPUI 既有语义，不做旁路封装。

### 4. 项目约定
- **命名约定**: Rust 类型 `PascalCase`，函数/字段 `snake_case`。
- **文件组织**:
  - `main/src/` 负责应用级快捷键、设置页、窗口入口
  - `crates/core/src/` 负责标签容器与通用 UI 容器
  - `crates/ui/src/` 负责通用标题栏等基础组件
  - `crates/terminal_view/src/` 负责终端 UI、侧边栏与文件管理器
  - `crates/terminal/src/` 负责终端底层状态与 SSH 重连
- **代码风格**: GPUI 链式 UI 声明、事件通过 `cx.emit` / `cx.subscribe` 连接。

### 5. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/core/src/tab_container.rs`: `TabBarDragState`
- `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/title_bar.rs`: `TitleBarState`
- `/Users/hufei/RustroverProjects/onetcli/script/bump-version.sh`: 统一版本 bump 流程
- `/Users/hufei/RustroverProjects/onetcli/main/src/onetcli_app.rs`: 全局快捷键与窗口动作注册
- `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/sidebar/file_manager_panel.rs`: 文件管理器 SFTP 连接与目录刷新
- `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/sidebar/mod.rs`: 侧边栏面板激活与路径同步
- `/Users/hufei/RustroverProjects/onetcli/crates/terminal/src/terminal.rs`: SSH 重连与当前工作目录状态

### 6. 根因判断
- **issue 1 事实**：`tab_container` 的拖动逻辑只挂在 `#tabs` 滚动容器上，且已有历史摘要记录“单 pinned 标签/空白区命中不足”。
- **issue 1 推论**：单首页标签无法拖动，本质是标签栏拖动区域覆盖不足，应向标题栏模式对齐。

- **issue 2 事实**：About 页取 `env!("CARGO_PKG_VERSION")`，`main/Cargo.toml` 当前是 `0.1.3`。
- **issue 2 推论**：这不是渲染 bug，而是版本源未随发布 bump。

- **issue 3 事实**：`ctrl-space` 绑定到 `MinimizeWindow`，实现依赖 `cx.active_window()`。
- **issue 3 推论**：窗口最小化后没有活跃窗口句柄，第二次快捷键无法恢复。

- **issue 4 事实**：右侧文件传输窗是 `terminal_view/sidebar/file_manager_panel.rs`，只会在首次激活时 `connect_if_idle()`；`refresh_dir()` 失败时只清空列表，不置错连接、不自动重连；SSH 终端重连链路不通知文件管理器。
- **issue 4 推论**：服务端重启后侧栏继续持有坏的 SFTP client，终端重连成功也不会同步恢复，最终表现为空白。

### 7. 测试策略
- **issue 1**: `cargo check` + Windows/Linux 桌面冒烟，验证单首页标签下拖动窗口。
- **issue 2**: 检查 `main/Cargo.toml` 与设置页 About 版本展示一致。
- **issue 3**: 桌面环境手工验证 `ctrl+space` 两次触发的隐藏/恢复。
- **issue 4**: 真实 SSH 服务端重启场景冒烟，验证终端重连后文件管理器恢复或进入明确可重连状态。

### 8. 推荐修复顺序
1. `SSH 终端重连后右侧文件传输窗空白`
2. `单首页标签时顶部无法拖动窗口`
3. `ctrl+space 只能最小化不能恢复窗口`
4. `关于页版本号显示落后`

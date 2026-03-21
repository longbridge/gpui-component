## 项目上下文摘要（terminal-sidebar-sync-path）
生成时间：2026-03-20 18:36:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/setting_tab.rs`
  - 模式：终端布尔配置通过 `AppSettings` 持久化，`terminal_sync_path_with_terminal` 默认值为 `true`，并通过 `save()` 写回配置文件。
  - 可复用：`terminal_auto_copy`、`terminal_middle_click_paste`、`terminal_confirm_*` 的字段定义与保存方式。
  - 需注意：布尔配置要同时维护 `serde default` 和 `Default`，否则旧配置升级时值不稳定。

- **实现2**: `main/src/home/home_tabs.rs`
  - 模式：`setup_terminal_view` 在终端视图创建后统一读取 `AppSettings`，再通过 `apply_terminal_settings_to_all` 广播到全部终端视图。
  - 可复用：`FontSizeChanged`、`AutoCopyChanged`、`MiddleClickPasteChanged`、`SyncPathChanged` 的全局持久化与同步链路。
  - 需注意：新建 SSH 终端和从 SFTP 打开 SSH 终端两个入口都会读取全局 `sync_path` 再传给 `TerminalView::new_ssh_with_index`。

- **实现3**: `crates/terminal_view/src/view.rs`
  - 模式：`TerminalSidebarEvent` 先由 `TerminalView` 处理，再上抛 `TerminalViewEvent` 给 `HomePage` 做全局持久化。
  - 可复用：`apply_terminal_settings` 作为跨 tab 同步设置的统一入口。
  - 需注意：当前 `apply_terminal_settings` 只更新 sidebar 和视图层状态，没有修改 `Terminal` 内部 SSH 初始化命令。

- **实现4**: `crates/terminal/src/terminal.rs`
  - 模式：SSH 终端在 `new_ssh` 时通过 `build_ssh_init_commands` 一次性拼好 `init_commands`，连接成功后在 `handle_ssh_result` 中通过 `self.write(...)` 写入交互式 shell。
  - 可复用：基础初始化命令拼接方式、`reconnect` 复用 `ssh_config` 的重连模式。
  - 需注意：`reconnect` 不会重建 `init_commands`，因此已有终端在设置切换后仍可能沿用旧值。

### 2. 项目约定
- **命名约定**: Rust 字段与方法使用 `snake_case`，终端全局设置统一用 `terminal_*` 前缀。
- **文件组织**: `main/src/setting_tab.rs` 负责设置持久化；`main/src/home/home_tabs.rs` 负责跨 tab 同步；`crates/terminal_view/src/` 负责单终端 UI/事件；`crates/terminal/src/` 负责底层终端状态与连接。
- **代码风格**: 优先复用既有事件链路与 setter，不新增旁路配置系统。

### 3. 可复用组件清单
- `main/src/setting_tab.rs`：`AppSettings`
- `main/src/home/home_tabs.rs`：`setup_terminal_view`、`apply_terminal_settings_to_all`
- `crates/terminal_view/src/view.rs`：`apply_terminal_settings`
- `crates/terminal_view/src/sidebar/mod.rs`：`TerminalSidebarEvent::SyncPathChanged` 转发
- `crates/terminal/src/terminal.rs`：`build_ssh_init_commands`、`reconnect`

### 4. 测试策略
- **测试框架**: Rust 内置单元测试 + `cargo check`
- **参考实现**: `crates/terminal/src/terminal.rs` 已有 `build_cd_command` 单元测试，可沿用同文件测试风格。
- **本次策略**: 为 SSH 初始化命令构造补单元测试，验证启用/关闭同步路径时 `OSC7_PROMPT_COMMAND` 的注入行为，再做 `terminal` 与 `terminal_view` 的编译检查。

### 5. 依赖和集成点
- **内部依赖链路**: `AppSettings` -> `HomePage::setup_terminal_view` -> `TerminalView::apply_terminal_settings` -> `Terminal`
- **SSH 创建入口**: `open_ssh_terminal` 与 `open_sftp_view` 中的 `OpenSshTerminal` 分支
- **关键集成点**: `Terminal::new_ssh` 构造初始化命令，`Terminal::handle_ssh_result` 执行初始化命令，`Terminal::reconnect` 复用既有 SSH 配置重新连接

### 6. 技术选型理由
- **为什么用这个方案**: 现有设置同步链路已经覆盖“局部变更 -> 全局保存 -> 广播到全部终端”，只需补齐 `Terminal` 内部状态刷新，不必新增事件协议。
- **优势**: 改动范围集中，能同时修复旧 tab 后续重连和跨 tab 设置同步的一致性问题。
- **劣势和风险**: 已经建立完成的远端 shell 环境不会因为本地切换开关立即改写，需要新建连接或重连后才体现新配置。

### 7. 关键风险点
- **行为边界**: 本次只能修复“未来连接使用的初始化命令”，不能无痕改写当前远端 shell 已设置的 `PROMPT_COMMAND`。
- **兼容性风险**: 当前注入命令使用 bash 风格 `PROMPT_COMMAND`，仓库内未见 zsh/fish 分支逻辑。
- **验证缺口**: 本地可验证编译与单元测试，真实 SSH 会话仍需手工确认路径同步行为。

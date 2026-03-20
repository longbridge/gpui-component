生成时间：2026-03-20 18:34:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/setting_tab.rs:113`
  - 模式：终端布尔配置通过 `AppSettings` 持久化，使用 `#[serde(default = "default_true")]` 和 `Default` 保证升级兼容。
  - 可复用：`terminal_auto_copy`、`terminal_middle_click_paste`、`terminal_confirm_*` 字段定义方式。
  - 需注意：新增字段必须同时补 `Default`，否则旧配置文件加载后不会稳定落到预期值。

- **实现2**: `main/src/home/home_tabs.rs:33`
  - 模式：`setup_terminal_view` 在视图创建后读取 `AppSettings`，再通过 `TerminalViewEvent` 持久化并调用 `apply_terminal_settings_to_all` 广播到全部终端。
  - 可复用：`AutoCopyChanged`、`MiddleClickPasteChanged` 的事件同步链路。
  - 需注意：仅改 sidebar 本地状态不会进入全局设置，也不会影响新开的终端。

- **实现3**: `crates/terminal_view/src/sidebar/mod.rs:177`
  - 模式：`SettingsPanelEvent` 先进入 `TerminalSidebarEvent`，再由 `TerminalView` 决定是否上抛到更高层。
  - 可复用：`AutoCopyChanged`、`MiddleClickPasteChanged` 的转发方式和 setter 设计。
  - 需注意：当前 `SyncPathChanged` 只更新 `sync_path_enabled`，没有向外 emit，是这次不同步的直接原因。

- **实现4**: `crates/terminal/src/terminal.rs:276`
  - 模式：SSH 终端在启动时拼接初始化命令，并无条件注入 `export PROMPT_COMMAND=...` 以发送 OSC 7。
  - 可复用：初始化命令列表拼接方式。
  - 需注意：这段逻辑发生在 SSH 会话创建前，配置若不提前传入，就无法影响是否注入。

### 2. 项目约定
- **命名约定**: Rust 字段与方法使用 `snake_case`，终端全局设置统一用 `terminal_*` 前缀。
- **文件组织**: `main/src/setting_tab.rs` 负责应用设置；`main/src/home/home_tabs.rs` 负责全局同步；`crates/terminal_view/src/` 负责单终端 UI/事件；`crates/terminal/src/` 负责底层终端构造。
- **代码风格**: 新逻辑优先复用既有事件枚举和 setter，不新增独立配置抽象。

### 3. 可复用组件清单
- `main/src/setting_tab.rs`：`AppSettings` 与 `default_true`
- `main/src/home/home_tabs.rs`：`setup_terminal_view`、`apply_terminal_settings_to_all`
- `crates/terminal_view/src/view.rs`：`apply_terminal_settings`
- `crates/terminal_view/src/sidebar/mod.rs`：`TerminalSidebarEvent` 转发
- `crates/terminal_view/src/sidebar/settings_panel.rs`：switch 渲染与本地 setter

### 4. 测试策略
- **测试框架**: Rust 内置单元测试 + `cargo check`
- **参考文件**: `crates/terminal/src/terminal.rs:937` 已有 `build_cd_command` 单元测试
- **本次策略**: 在 `terminal.rs` 增加初始化命令构建测试，验证 `PROMPT_COMMAND` 只在启用时注入；再对相关 crate 做编译检查。

### 5. 依赖和集成点
- **外部依赖**: 无新增外部库。
- **内部依赖**: `AppSettings` -> `HomePage::setup_terminal_view` -> `TerminalView` -> `TerminalSidebar/SettingsPanel`；SSH 构造入口还会进入 `Terminal::new_ssh`。
- **集成方式**: 沿用现有终端设置同步模式，不新增旁路逻辑。

### 6. 技术选型理由
- **为什么用这个方案**: 现有 `auto_copy`/`middle_click_paste` 已经证明这条同步链能覆盖“单终端变更 -> 全局持久化 -> 广播到全部终端”。
- **优势**: 修改面集中、风险低，且能保证新开的 SSH 终端读取到统一配置。
- **劣势和风险**: 已经建立的 SSH 会话不会因为切换开关自动重写远端 `PROMPT_COMMAND`，该行为只能对新会话生效。

### 7. 关键风险点
- **边界条件**: 需要同时覆盖普通 SSH 打开和从 SFTP 打开 SSH 两个入口。
- **一致性风险**: 如果 `TerminalSidebarEvent` 或 `TerminalViewEvent` 漏掉 `SyncPathChanged`，界面状态仍会局部生效但不会持久化。
- **验证缺口**: 本地只能验证编译和单元测试，真实远端 SSH 会话行为仍需 UI 场景手测。

## 项目上下文摘要（window-not-found-fix）
生成时间：2026-03-26 11:31:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/main.rs:24-26`
  - 模式：应用入口通过 `Application::new().with_assets(...).run(...)` 启动，目前未显式配置 `QuitMode`。
  - 可复用：入口处统一配置应用级退出策略。
  - 需注意：macOS 下 `QuitMode::Default` 不会在最后一个窗口关闭时自动退出。

- **实现2**: `main/src/onetcli_app.rs:350-354`
  - 模式：当前主窗口实体在 `cx.on_release(|_, cx| cx.quit())` 中手动退出应用。
  - 可复用：说明当前行为目标是“主窗口关闭即退出应用”。
  - 需注意：该回调发生在主窗口释放阶段，可能与平台尾随事件形成生命周期竞态。

- **实现3**: `crates/story/examples/dock.rs:403-412`
  - 模式：示例代码也在窗口更新闭包里注册 `on_release -> quit`。
  - 可复用：证明仓库里存在同类写法。
  - 需注意：这是示例代码，不一定是最稳妥的生产用法。

- **实现4**: `/Users/hufei/.cargo/git/checkouts/zed-a70e2ad075855582/8b5328c/crates/gpui/src/app.rs:177-181,253-261`
  - 模式：`gpui::Application` 官方提供 `with_quit_mode(QuitMode::LastWindowClosed)`。
  - 可复用：使用框架内置的“最后一个窗口关闭自动退出”策略。
  - 需注意：该模式比在 `on_release` 里手动 `quit` 更贴近窗口表移除流程本身。

- **实现5**: `/Users/hufei/.cargo/git/checkouts/zed-a70e2ad075855582/8b5328c/crates/zed/src/main.rs:101-103`
  - 模式：上游 Zed 在入口处通过 `with_quit_mode(...)` 配置退出行为。
  - 可复用：入口统一声明退出模式的写法。
  - 需注意：这说明 `QuitMode` 是正式设计路径，而非临时技巧。

### 2. 项目约定
- **命名约定**: Rust 类型使用 `PascalCase`，函数和局部变量使用 `snake_case`。
- **文件组织**: 应用级启动逻辑放在 `main/src/main.rs`，主应用实体生命周期逻辑放在 `main/src/onetcli_app.rs`。
- **导入顺序**: 先本地模块，再外部 crate；同组导入保持项目既有顺序。
- **代码风格**: 优先复用框架内置机制，不额外引入自定义生命周期封装。

### 3. 可复用组件清单
- `main/src/main.rs`：应用入口，适合设置 `QuitMode`。
- `main/src/onetcli_app.rs`：主窗口实体，当前含有待移除的 release 退出监听。
- `gpui/src/app.rs`：`with_quit_mode` 与 `QuitMode::LastWindowClosed` 官方实现。
- `crates/zed/src/main.rs`：上游入口配置 `with_quit_mode(...)` 的参考模式。

### 4. 测试策略
- **测试框架**: Rust 内置编译与测试体系。
- **验证方式**: 先执行 `cargo check -p main` 做编译验证，再进行人工关闭主窗口冒烟验证。
- **参考文件**: `main/src/main.rs`、`main/src/onetcli_app.rs`、`gpui/src/app.rs`。
- **覆盖要求**: 确认编译通过，且关闭主窗口后应用正常退出，不再输出 `window not found`。

### 5. 依赖和集成点
- **外部依赖**: `gpui::Application::with_quit_mode`、`gpui::QuitMode::LastWindowClosed`。
- **内部依赖**: `OnetCliApp` 的 `on_app_quit` 存档逻辑仍需保留。
- **集成方式**: 在应用入口声明自动退出策略，移除主窗口 release 阶段的手动 `cx.quit()`。
- **配置来源**: 无新增配置。

### 6. 技术选型理由
- **为什么改用 `QuitMode::LastWindowClosed`**: 让应用退出和窗口表移除位于同一框架生命周期路径，减少“窗口已释放后再触发 quit”的竞态窗口。
- **为什么删除 `on_release -> quit`**: 它重复表达了退出意图，并把退出动作放在实体释放阶段，容易与平台尾随事件交错。
- **为什么不直接改 `gpui` 上游**: 当前问题可先在项目层规避，成本更低，也不需要引入依赖补丁。

### 7. 关键风险点
- **行为风险**: 需要确认 macOS 上关闭最后一个窗口后，现有 `on_app_quit` 收尾逻辑仍能正常执行。
- **验证风险**: 终端环境无法自动完成图形界面关闭动作，最终仍需人工冒烟验证一次。
- **资料风险**: Context7 对 GPUI 文档查询本次失败，因此官方 API 依据改为本地依赖源码与上游 `zed` 源码。 

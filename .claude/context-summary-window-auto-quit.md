## 项目上下文摘要（window-auto-quit）
生成时间：2026-03-17 11:35:29 +0800

### 1. 相似实现分析
- **实现1**: `main/src/main.rs:23`
  - 模式：应用入口在 `app.run` 中初始化全局状态、启动后台任务，再异步创建主窗口。
  - 可复用：主窗口打开链路、`cx.open_window(... Root::new(...))` 结构。
  - 需注意：当前只注册了 `on_app_quit`，没有在主窗口释放时显式调用 `cx.quit()`。

- **实现2**: `crates/story/examples/dock.rs:399`
  - 模式：窗口创建后通过 `cx.on_release(|_, cx| cx.quit())` 在窗口释放时退出应用。
  - 可复用：释放主实体时显式退出应用的做法。
  - 需注意：这是仓库内唯一直接覆盖“关闭窗口即退出”的现成模式。

- **实现3**: `crates/core/src/popup_window.rs:83`
  - 模式：弹窗窗口复用统一 `open_popup_window` 创建流程，但不会绑定全局退出逻辑。
  - 可复用：区分“主窗口生命周期”和“辅助窗口生命周期”的边界。
  - 需注意：退出逻辑不应挂在通用弹窗实现上，否则关闭任意弹窗都会退出整个应用。

- **实现4**: `crates/db/src/cache_manager.rs:562` 与 `crates/db/src/manager.rs:696`
  - 模式：入口初始化后会启动长期运行的清理循环任务。
  - 可复用：说明后台任务与应用生命周期绑定，只有应用真正 quit 才会停止。
  - 需注意：若主窗口关闭但未触发 `cx.quit()`，这些循环会让进程持续存活。

### 2. 项目约定
- **命名约定**: Rust 类型使用 `PascalCase`，函数、字段、局部变量使用 `snake_case`
- **文件组织**: 应用级初始化放在 `main/src/main.rs` 与 `main/src/onetcli_app.rs`
- **导入顺序**: 先本地模块，再外部 crate；同一 `use` 语句内按项目既有顺序组织
- **代码风格**: 生命周期订阅通过 `cx.on_*` 注册并在无需持有时直接 `.detach()`

### 3. 可复用组件清单
- `main/src/onetcli_app.rs`: 主应用实体 `OnetCliApp`，适合挂接主窗口释放时的退出逻辑
- `crates/story/examples/dock.rs`: `cx.on_release(...).detach()` 的现成写法
- `crates/core/src/popup_window.rs`: 辅助窗口创建模式，用于确认本次不应修改通用弹窗行为
- `main/src/update.rs`: `on_app_quit` 已有保存/退出链路，可与 `cx.quit()` 联动

### 4. 测试策略
- **测试框架**: Rust 内置单元测试（`#[cfg(test)]` / `#[test]`）
- **参考文件**: `main/src/update.rs:806`、`crates/terminal/src/terminal.rs:938`
- **覆盖要求**: 本次 UI 生命周期改动以编译验证为主；图形化“关闭窗口退出”需要人工交互验证

### 5. 依赖和集成点
- **外部依赖**: `gpui::Context::on_release`、`gpui::App::quit`
- **内部依赖**: `OnetCliApp::new`、`save_tab_state` 的 `on_app_quit` 回调
- **集成方式**: 在主应用实体释放时调用 `cx.quit()`，让现有 `on_app_quit` 回调继续负责收尾
- **配置来源**: 无新增配置

### 6. 技术选型理由
- **为什么在 `OnetCliApp` 上绑定 `on_release`**: `OnetCliApp` 只存在于主窗口，关闭主窗口时会释放该实体，且不会误伤弹窗窗口
- **为什么不修改后台任务**: 任务本身不是根因；缺的是“主窗口关闭后触发应用退出”的桥接动作
- **为什么不改通用弹窗**: 弹窗关闭不应退出整个应用，会带来明显行为回归

### 7. 关键风险点
- **生命周期风险**: 若 `on_release` 挂在错误实体上，可能导致关闭子窗口或普通控件时意外退出
- **验证风险**: 当前环境无法自动执行图形界面关闭窗口动作，只能以编译和代码路径分析验证
- **工具约束**: 当前会话未提供 `desktop-commander`、`context7`、`github.search_code`，本次改用本地源码检索、Cargo 依赖源码和 `rg`/`sed` 完成证据收集

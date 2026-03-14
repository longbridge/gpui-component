## 项目上下文摘要（shortcut-key-support）
生成时间：2026-03-14 12:45:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/ui/src/input/state.rs:200`
  - 模式：大量快捷键使用 `#[cfg(target_os = "macos")]` / `#[cfg(not(target_os = "macos"))]` 成对定义
  - 可复用：跨平台快捷键分支写法、`KeyBinding::new` 注册习惯
  - 需注意：这里是输入系统最完整的跨平台映射样板

- **实现2**: `crates/one_ui/src/edit_table/mod.rs:15`
  - 模式：复制/粘贴/全选的最小跨平台绑定模板
  - 可复用：`cx.bind_keys([...])` 组织方式
  - 需注意：适合通用编辑动作类快捷键

- **实现3**: `crates/terminal_view/src/view.rs:70`
  - 模式：终端上下文 `TERMINAL_CONTEXT` + `KeyBinding::new` + `on_action` 处理
  - 可复用：终端复制/粘贴/搜索等绑定与处理结构
  - 需注意：终端有 `increase_font_size/decrease_font_size/reset_font_size` 但未绑定快捷键

- **实现4**: `main/src/onetcli_app.rs:48`
  - 模式：全局 `cx.bind_keys(vec![...])` 注册应用级快捷键
  - 可复用：全局快捷键入口
  - 需注意：目前仅有 `shift-escape` / `ctrl-w`

### 2. 项目约定
- **命名约定**: Rust 类型 `PascalCase`，函数与字段 `snake_case`
- **文件组织**: 模块级快捷键集中在 `init(cx)` 或视图 `key_context` 中注册
- **导入顺序**: 先标准库、再外部依赖、最后本地模块
- **代码风格**: `#[cfg(target_os = "macos")]`/`#[cfg(not(target_os = "macos"))]` 成对分支

### 3. 可复用组件清单
- `crates/terminal_view/src/view.rs`: 终端快捷键与动作绑定模板
- `crates/ui/src/kbd.rs`: 快捷键显示格式化（跨平台）
- `crates/one_ui/src/edit_table/mod.rs`: 最小跨平台快捷键模板
- `crates/core/src/tab_container.rs`: `set_active_index` 用于切换标签

### 4. 测试策略
- **测试框架**: Rust 内置单元测试（`#[test]`）
- **参考文件**: `crates/ui/src/kbd.rs:244`
- **覆盖要求**: 快捷键字符串解析与展示；新增绑定优先做解析测试或行为测试

### 5. 依赖和集成点
- **外部依赖**: `gpui::KeyBinding` / `gpui::Keystroke` / `gpui::Modifiers`
- **内部依赖**: `TabContainer`（切换标签）、`TerminalView`（字体大小/复制粘贴）
- **集成方式**: `cx.bind_keys` 注册，`on_action` 绑定动作处理
- **配置来源**: `main/src/setting_tab.rs` 的 `AppSettings`（含 font_size）

### 6. 技术选型理由
- **为什么使用 gpui 绑定**: 项目现有输入系统统一使用 `KeyBinding` + `on_action`
- **为什么按平台分支**: 仓库既有大量跨平台快捷键分支，风格一致且风险低

### 7. 关键风险点
- **功能风险**: 部分快捷键需全局热键能力，gpui 是否支持需验证
- **一致性风险**: 不同模块可能重复定义相同快捷键，需要集中管理或明确上下文
- **测试风险**: 现有测试较少，新增绑定可能缺乏行为级覆盖

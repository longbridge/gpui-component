## 项目上下文摘要（shortcuts-cross-platform）
生成时间：2026-03-06 16:54:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/ui/src/input/state.rs:220`
  - 模式：大量快捷键通过 `#[cfg(target_os = "macos")]` / `#[cfg(not(target_os = "macos"))]` 成对定义。
  - 可复用：跨平台快捷键分支写法、编辑器类输入的完整键位映射。
  - 需注意：这里是项目里最完整的跨平台样板。

- **实现2**: `crates/one_ui/src/edit_table/mod.rs:27`
  - 模式：对复制/粘贴/全选使用显式 macOS / 非 macOS 分支。
  - 可复用：简单场景下的最小跨平台模板。
  - 需注意：适合通用文本操作类动作。

- **实现3**: `examples/system_monitor/src/main.rs:607`
  - 模式：退出快捷键对 macOS 用 `cmd-q`，对非 macOS 用 `alt-f4`。
  - 可复用：系统级动作按平台语义分支，而不是硬编码同一套按键。
  - 需注意：说明仓库本身已经有平台差异意识。

- **实现4**: `crates/terminal_view/src/view.rs:69`
  - 模式：直接硬编码 `cmd-c/cmd-v/cmd-a/cmd-f/cmd-g`，无非 macOS 分支。
  - 需注意：这是本轮发现的真实问题点之一。

- **实现5**: `crates/redis_view/src/redis_cli_view.rs:281`
  - 模式：同时存在 `ctrl-a/ctrl-e` 与 `cmd-left/cmd-right`，但复制粘贴和行首行尾选择仍有未分支的 `cmd-*`。
  - 需注意：这是第二个真实问题点，且影响范围比 terminal_view 更大。

### 2. 项目约定
- **命名约定**: Rust 类型 `PascalCase`，函数与字段 `snake_case`。
- **文件组织**: 组件级快捷键通常在各模块的 `init(cx)` 中集中注册。
- **代码风格**: 已存在大量 `#[cfg(target_os = "macos")]` 与 `#[cfg(not(target_os = "macos"))]` 的成对绑定模式。
- **显示层约定**: `crates/ui/src/kbd.rs` 负责将快捷键格式化为平台相关展示文本。

### 3. 可复用组件清单
- `crates/ui/src/input/state.rs`: 完整跨平台键位分支参考。
- `crates/one_ui/src/edit_table/mod.rs`: 通用编辑动作的最小跨平台模板。
- `crates/ui/src/kbd.rs`: 快捷键显示格式化，已区分 macOS 与 Windows/Linux。

### 4. 测试策略
- **测试框架**: Rust 内置单元测试。
- **现有参考**: `crates/ui/src/kbd.rs:248` 覆盖了快捷键显示格式化的部分平台差异。
- **缺口**: 当前缺少针对具体 `KeyBinding::new` 注册结果的跨平台单元测试。

### 5. 依赖和集成点
- **外部依赖**: `gpui::KeyBinding`、`gpui::Keystroke`、`gpui::Modifiers`。
- **内部依赖**: 各模块 `init(cx)` 通过 `cx.bind_keys(...)` 注册动作。
- **集成方式**: 依赖 GPUI 的按键解析与上下文机制。

### 6. 技术选型理由
- **为什么显式分支可行**: 仓库当前大量使用条件编译定义平台差异，维护成本可控且与现有风格一致。
- **为什么硬编码 `cmd-` 有风险**: 在 Windows/Linux 下，`cmd-*` 会偏向平台键（Win/Super）而非用户预期的 `Ctrl`，导致实际不可用或不符合惯例。

### 7. 关键风险点
- **功能风险**: 复制/粘贴/搜索/全选等常用动作在 Windows/Linux 上可能失效或映射到错误修饰键。
- **一致性风险**: 不同模块对同类动作使用不同平台策略，用户体验割裂。
- **可维护性风险**: 缺少统一快捷键辅助函数，未来继续引入 `cmd-*` 漏配的概率较高。

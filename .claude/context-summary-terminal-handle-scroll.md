## 项目上下文摘要（terminal-handle-scroll）
生成时间：2026-03-06 16:10:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/terminal_view/src/view.rs:1248`
  - 模式：终端视图统一处理滚轮事件，按模式分支到 ALT_SCREEN、VI、普通滚动。
  - 可复用：`scroll_lines_accumulated`、`write_to_pty`、`Terminal::scroll`。
  - 需注意：ALT_SCREEN 目前用 `ceil()` 直接离散化，易放大小幅滚轮输入。

- **实现2**: `crates/redis_view/src/redis_cli_view.rs:1251`
  - 模式：基于滚轮像素量换算偏移并 `clamp`。
  - 可复用：保留 delta 原始语义，不额外放大输入。
  - 需注意：更新偏移后立即同步滚动条指标并 `cx.notify()`。

- **实现3**: `crates/one_ui/src/edit_table/state.rs:236`
  - 模式：先获取 `pixel_delta`，再根据正负值和边界决定是否消费事件。
  - 可复用：项目内对滚轮正负方向的约定。
  - 需注意：正负方向判断应和边界/滚动语义保持一致。

### 2. 项目约定
- **命名约定**: Rust 类型 `PascalCase`，函数与字段 `snake_case`。
- **文件组织**: 终端视图逻辑集中在 `crates/terminal_view/src/view.rs`，键序列测试在 `crates/terminal_view/src/keys.rs`。
- **导入顺序**: 先外部 crate，再内部模块；与现有文件保持一致。
- **代码风格**: 状态变更后配合 `cx.notify()`，优先小范围修复而非改动架构。

### 3. 可复用组件清单
- `crates/terminal_view/src/view.rs`: `scroll_lines_accumulated` 现有滚轮累计状态。
- `crates/terminal_view/src/view.rs`: `write_to_pty` 终端输入写入封装。
- `crates/terminal/src/terminal.rs`: `Terminal::scroll`，封装 display scroll。
- `crates/terminal_view/src/keys.rs`: APP_CURSOR 上下箭头序列测试参考。

### 4. 测试策略
- **测试框架**: Rust 内置单元测试。
- **测试模式**: 小范围纯函数测试 + 保留现有键序列测试。
- **参考文件**: `crates/terminal_view/src/keys.rs:291`。
- **覆盖要求**: 正常流程、正负小数累计、跨整数阈值行为。

### 5. 依赖和集成点
- **外部依赖**: `gpui::ScrollWheelEvent`、`alacritty_terminal::term::TermMode`。
- **内部依赖**: `TerminalView::handle_scroll` → `write_to_pty` / `Terminal::scroll`。
- **集成方式**: 通过 `.on_scroll_wheel(cx.listener(Self::handle_scroll))` 绑定到视图。
- **配置来源**: 无新增配置，沿用现有滚轮行为。

### 6. 技术选型理由
- **为什么用累计量化**: 现有普通模式已采用累计器，复用它能统一行为并避免触控板事件被放大。
- **优势**: 改动小、可测试、与当前架构一致。
- **劣势和风险**: 需要谨慎处理负数转整，避免方向或阈值出错。

### 7. 关键风险点
- **边界条件**: 小于 1 行的连续滚动不能被提前触发。
- **性能瓶颈**: ALT_SCREEN 下不应因细小滚动而频繁写 PTY。
- **维护风险**: VI 分支注释若继续错误，会误导后续修改方向判断。

## 项目上下文摘要（terminal-command-scroll-bottom）
生成时间：2026-03-26 16:41:37 +0800

### 1. 相似实现分析
- **实现1**: `crates/terminal_view/src/view.rs:982`
  - 模式：`write_to_pty` 在用户输入前检查当前 `display_offset`，若不在底部则直接 `scroll_display(Bottom)`。
  - 可复用：现有“用户输入时回到底部”语义。
  - 需注意：当前只处理已生效偏移，没有处理延迟应用的滚动偏移。

- **实现2**: `crates/terminal_view/src/view.rs:383`
  - 模式：滚动条拖动不立即改终端，而是把目标位置写入 `future_display_offset`，在下一次渲染时应用。
  - 可复用：现有滚动条异步提交机制。
  - 需注意：如果别的路径先改变了终端滚动状态，陈旧的待提交偏移需要被取消。

- **实现3**: `crates/terminal_view/src/view.rs:2226`
  - 模式：render 阶段读取 `take_future_display_offset()`，再调用 `terminal.scroll(delta)` 真正落地。
  - 可复用：延迟偏移消费点已经存在，无需新增渲染路径。
  - 需注意：这里会覆盖 render 前发生的其它滚动决定。

### 2. 项目约定
- **命名约定**: Rust 常规 `snake_case` / `CamelCase`。
- **文件组织**: 终端视图逻辑集中在 `crates/terminal_view/src/view.rs`，底部带文件内单元测试。
- **代码风格**: 事件或输入逻辑优先收敛到私有辅助函数，尽量不扩散接口。

### 3. 可复用组件清单
- `crates/terminal_view/src/view.rs:367` `take_future_display_offset`
- `crates/terminal_view/src/view.rs:982` `write_to_pty`
- `crates/core/src/ai_chat/engine.rs:232` `scroll_to_bottom`

### 4. 测试策略
- **测试框架**: Rust 内置测试。
- **参考文件**: `crates/terminal_view/src/view.rs` 文件尾部现有纯函数测试。
- **覆盖要求**: 覆盖“清除陈旧待提交偏移”和“是否需要滚到底部”两个场景。

### 5. 依赖和集成点
- **外部依赖**: `alacritty_terminal` 提供 `scroll_display(Bottom)` 语义。
- **内部依赖**: `TerminalScrollbarHandle` 延迟记录滚动目标，render 阶段消费。
- **集成方式**: `write_to_pty` 是用户输入统一入口，适合在此清理陈旧滚动请求。

### 6. 技术选型理由
- **为什么用这个方案**: 问题发生在现有滚动状态协调，而不是终端渲染能力缺失；最小修复应留在现有输入入口。
- **优势**: 不改渲染链路、不改滚动条 API，只修正用户输入优先级。
- **劣势和风险**: 如果清理时机过宽，可能误取消用户刚发起的有效滚动请求，因此只在用户输入路径处理。

### 7. 关键风险点
- **边界条件**: 当前已在底部但存在 `future_display_offset` 时，也必须清理待提交偏移。
- **一致性风险**: 修复后不能影响滚轮、拖动滚动条、VI/ALT_SCREEN 等其它滚动路径。
- **验证缺口**: 目前没有完整 UI 冒烟测试，本次先用文件内回归测试锁定核心逻辑。

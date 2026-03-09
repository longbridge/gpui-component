## 项目上下文摘要（terminal-scroll）
生成时间：2026-03-09

### 1. 相似实现分析
- **实现1**: `crates/terminal_view/src/view.rs:1345`
  - 模式：将 `ScrollWheelEvent.delta.pixel_delta()` 转成行数，再分支处理普通终端、VI 模式和 `ALT_SCREEN`。
  - 可复用：`take_whole_scroll_lines`、`Terminal::scroll_display_delta` / `scroll_display` 语义。
  - 需注意：`ALT_SCREEN` 下手工把滚轮映射成上下箭头，存在额外符号转换。

- **实现2**: `crates/ui/src/input/state.rs:1551`
  - 模式：直接 `old_offset + delta`，不反转 `delta.y`。
  - 可复用：项目内 GPUI 滚轮的直接位移语义。
  - 需注意：只有偏移实际变化时才停止事件传播。

- **实现3**: `crates/ui/src/scroll/scrollable_mask.rs:127`
  - 模式：直接 `offset.y += delta.y`，不反转 `delta.y`。
  - 可复用：GPUI 通用滚动遮罩处理。
  - 需注意：触控板同时给出 x/y 时，会保留主轴方向。

- **实现4**: `crates/redis_view/src/redis_cli_view.rs:1269`
  - 模式：将像素转行后用 `scroll_offset - delta` 更新正向滚动值。
  - 可复用：标量偏移场景下通过容器坐标系换算方向。
  - 需注意：这里的减号来自内部 `scroll_offset` 定义，不代表平台事件反向。

### 2. 项目约定
- **命名约定**: Rust 常规 `snake_case`，类型使用 `CamelCase`。
- **文件组织**: 视图层在 `crates/*_view/src/`，通用滚动控件在 `crates/ui/src/scroll/`。
- **代码风格**: 事件处理函数统一先取 `event.delta.pixel_delta(...)`，再按内部坐标语义更新状态。

### 3. 可复用组件清单
- `crates/terminal_view/src/view.rs:79` `take_whole_scroll_lines`
- `crates/terminal/src/terminal.rs:187` `scroll_display_delta`
- `crates/ui/src/scroll/scrollable_mask.rs:127` 通用滚轮偏移处理

### 4. 测试策略
- **测试框架**: Rust 内置测试。
- **参考实现**: `crates/terminal_view/src/view.rs` 底部已有 `take_whole_scroll_lines` 相关测试。
- **覆盖缺口**: 当前未见 `ALT_SCREEN` 上下箭头方向测试。

### 5. 依赖和集成点
- **外部依赖**: `gpui` 提供 `ScrollWheelEvent`；`alacritty_terminal` 提供 `scroll_display`、VI 光标滚动语义。
- **内部依赖**: `TerminalView.handle_scroll` -> `Terminal` / `PTY` 写入。
- **集成方式**: 普通模式直接滚动 display；`ALT_SCREEN` 模式手工发送箭头序列。

### 6. 技术选型理由
- **为什么这样设计**: 终端普通回滚区与 `ALT_SCREEN`（如 `vim`/`less`）行为不同，后者通常需要转成应用内可识别的输入事件。
- **优势**: 能兼容终端应用自身的翻页/滚动逻辑。
- **风险**: 一旦手工映射的符号语义和平台事件约定不一致，就会出现方向反转。

### 7. 关键风险点
- **边界条件**: 小数滚轮增量累计后才触发行，容易在手工映射分支出现符号误判。
- **一致性风险**: `ALT_SCREEN` 分支如果和普通滚动、项目其他滚动实现不一致，会造成用户感知“与系统设置相反”。
- **验证缺口**: 缺少“正负 delta -> 上下箭头”的本地测试。

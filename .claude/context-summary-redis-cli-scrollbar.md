## 项目上下文摘要（redis-cli-scrollbar）
生成时间：2026-03-05 21:16:11 +0800

### 1. 相似实现分析
- 实现1：`crates/redis_view/src/redis_cli_view.rs`
  - 现状：已有 `scroll_offset`、`handle_scroll`、自绘 `RedisCliElement`，但没有可见滚动条。
  - 可复用：现有滚轮滚动逻辑、`terminal_bounds`、`line_height` 计算。
  - 需注意：不能破坏鼠标选择、IME 光标定位、输入编辑链路。

- 实现2：`crates/terminal_view/src/view.rs`
  - 模式：`Scrollbar::vertical + ScrollbarHandle(offset/set_offset/content_size)`。
  - 可复用：`metrics + handle + future/pending offset` 的同步机制。
  - 需注意：在 `render` 中消费 pending 偏移并更新实际内容滚动状态。

- 实现3：`crates/redis_view/src/redis_tree_view.rs`
  - 模式：容器级 `.vertical_scrollbar(&scroll_handle)`。
  - 可复用：在右侧叠加可见滚动条的交互预期。
  - 需注意：该模式依赖 `track_scroll`，不直接适配 redis_cli 的自绘内容。

- 实现4：`crates/one_ui/src/edit_table/state.rs`
  - 模式：绝对定位叠加滚动条层（`.absolute().right_0()`）。
  - 可复用：滚动条与内容层解耦渲染。

### 2. 项目约定
- 命名约定：Rust 函数/字段 `snake_case`，类型 `PascalCase`。
- 文件组织：视图状态、交互、渲染集中在 `redis_cli_view.rs`。
- 代码风格：GPUI 链式构建 UI；局部状态 + `cx.notify()` 驱动重绘。
- 导入方式：优先按模块分组导入，复用现有 gpui/gpui_component 结构。

### 3. 可复用组件清单
- `gpui_component::scroll::Scrollbar`
- `gpui_component::scroll::ScrollbarHandle`
- `gpui_component::scroll::ScrollbarShow`
- `RedisCliView::scroll_offset` 与 `handle_scroll`
- `RedisCliView::terminal_bounds` 与 `theme.font_size/line_height_scale`

### 4. 测试策略
- 当前 `redis_view` 几乎无 UI 层单测，主要通过编译与手动交互验证。
- 参考验证方式：`cargo check -p redis_view`。
- 本次验证重点：
  - 编译通过
  - 滚轮滚动与滚动条位置同步
  - 拖动滚动条可改变内容偏移
  - 输入/选择/IME 相关代码路径不回归（编译级）

### 5. 依赖与集成点
- 依赖：`gpui`、`gpui_component::scroll`。
- 集成点：
  - `render()`：叠加滚动条 UI
  - `handle_scroll()`：滚轮更新 `scroll_offset`
  - `canvas` 预绘制回调：更新视口尺寸与滚动度量

### 6. 外部资料
- Context7：`/longbridge/gpui-component`，确认了 `Scrollbar` 与 `ScrollbarHandle` 的通用语义。
- GitHub 搜索：`impl ScrollbarHandle for`，定位到 `longbridge/gpui-component` 与其他 GPUI 项目参考。

### 7. 风险点
- 风险1：滚动条 `set_offset` 无法直接访问 `Context`，需要 pending 机制在 `render` 回写。
- 风险2：视口高度变化导致 `scroll_offset` 越界，需要统一 clamp。
- 风险3：叠加层可能影响鼠标选择，需要保持滚动条仅占右侧狭窄区域。
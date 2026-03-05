## 项目上下文摘要（tab-container-windows-drag）
生成时间：2026-03-05 10:30:00 +0800

### 1. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/core/src/tab_container.rs:1520`
  - 模式：`tab-bar` 顶层用 `start_window_move`，`#tabs` 子容器声明 `window_control_area(WindowControlArea::Drag)`。
  - 可复用：`TabBarDragState` + `on_mouse_down/on_mouse_move` 手动拖动状态机。
  - 需注意：`#tabs` 同时使用 `overflow_x_scroll()`，可能吞掉鼠标事件。

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/title_bar.rs:252`
  - 模式：顶层 `title-bar` 与内部 `bar` 组合；`bar` 声明 `WindowControlArea::Drag`，同时保留手动拖动状态。
  - 可复用：`TitleBarState.should_move` 事件流。
  - 需注意：可交互子元素会主动 `stop_propagation`，防止误拖动。

- **实现3（开源）**: `zed-industries/zed/crates/platform_title_bar/src/platform_title_bar.rs`
  - 模式：标题栏主容器统一声明 `WindowControlArea::Drag`，并用 `should_move` 状态触发 `start_window_move()`。
  - 可复用：拖动区域声明在稳定容器，不依赖子层事件冒泡。
  - 需注意：Windows 控件区与拖动区分离（Min/Max/Close 独立 control area）。

### 2. 项目约定
- **命名约定**: Rust 变量/函数 `snake_case`，类型 `PascalCase`。
- **文件组织**: 标签容器逻辑集中在 `crates/core/src/tab_container.rs`。
- **导入顺序**: 先标准库，再第三方，再项目内模块。
- **代码风格**: GPUI 链式声明 + 条件 `.when(...)` 组合。

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/core/src/tab_container.rs`：`TabBarDragState`
- `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/title_bar.rs`：`TitleBarState`
- `/Users/hufei/RustroverProjects/onetcli/crates/core/src/tab_container.rs`：`render_control_button` 的 `window_control_area`

### 4. 测试策略
- **测试框架**: Cargo 编译检查 + 手动交互验证。
- **测试模式**: 冒烟验证（Windows 标题栏拖动）。
- **参考文件**: `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/title_bar.rs`
- **覆盖要求**: 非 tab 空白区域可拖动；tab 点击/拖拽排序/关闭按钮行为保持不变。

### 5. 依赖和集成点
- **外部依赖**: `gpui` 的 `WindowControlArea` 与窗口拖动 API。
- **内部依赖**: `TabContainer::render_tab_bar` 与 `render_window_controls`。
- **集成方式**: 通过 `OnetCliApp::new` 的 `.with_window_controls(true)` 启用。
- **配置来源**: `main/src/onetcli_app.rs`。

### 6. 技术选型理由
- **为什么用这个方案**: 最小改动补齐 `#tabs` 的拖动事件链，避免依赖父层冒泡。
- **优势**: 不改变布局结构，不影响现有 tab 重排与窗口控件逻辑。
- **劣势和风险**: 需要确认不会触发重复 `start_window_move`，通过 `should_move` 置回 `false` 控制。

### 7. 关键风险点
- **并发问题**: 无共享并发状态，风险低。
- **边界条件**: tab 区域空白、tab 元素本身、窗口控件区事件优先级。
- **性能瓶颈**: 仅增加少量事件回调，无明显性能影响。
- **安全考虑**: 不涉及权限或数据安全路径。
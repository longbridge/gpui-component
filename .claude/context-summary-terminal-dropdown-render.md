## 项目上下文摘要（terminal-dropdown-render）
生成时间：2026-03-03

### 1. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/terminal_element.rs`
  - 模式：缓存行（background_rects/text_runs）+ paint 分层绘制
  - 可复用：`RenderCache`、`TerminalElementImpl::paint`
  - 需注意：当前仅绘制非默认背景，可能导致擦除场景残影

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/redis_view/src/redis_cli_element.rs:459`
  - 模式：每帧先绘制整块背景，再绘制文本/选择/光标
  - 可复用：背景先覆盖后叠加文本的顺序
  - 需注意：该模式可避免旧字形残留

- **实现3**: `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/input/element.rs:1609`
  - 模式：绘制补全文本前，先刷背景覆盖“可能存在的旧文本”
  - 可复用：局部覆盖旧内容的防残影策略
  - 需注意：注释明确说明覆盖旧文本的意图

### 2. 项目约定
- **命名约定**: Rust snake_case / PascalCase
- **文件组织**: terminal_view 负责终端缓存与渲染，view 传入计算参数
- **导入顺序**: crate 内模块在前，外部依赖在后，标准库最后
- **代码风格**: 早返回、局部注释说明意图、最小侵入修改

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/terminal_element.rs`: 终端缓存与渲染主实现
- `/Users/hufei/RustroverProjects/onetcli/crates/redis_view/src/redis_cli_element.rs`: 背景覆盖渲染模式参考
- `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/input/element.rs`: 覆盖旧文本防残影策略参考

### 4. 测试策略
- **测试框架**: Rust `#[test]`（按 crate 内模块分布）
- **测试模式**: 单元测试为主，构建检查作为快速回归
- **参考文件**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/view.rs`（含 `mod tests`）
- **覆盖要求**: 先做 `cargo check -p terminal_view`，再手动验证下拉交互场景

### 5. 依赖和集成点
- **外部依赖**: `gpui`, `alacritty_terminal`
- **内部依赖**: `TerminalView -> TerminalElement::new -> TerminalElementImpl::paint`
- **集成方式**: RenderCache 缓存构建 + Element paint 绘制
- **配置来源**: `TerminalTheme`（含 `background` / `cursor`）

### 6. 技术选型理由
- **为什么用这个方案**: 最小改动即可修复擦除残影，不破坏增量缓存结构
- **优势**: 兼容现有渲染链路，稳定消除旧字形
- **劣势和风险**: 每帧多一次背景填充，需关注低端设备绘制开销

### 7. 关键风险点
- **并发问题**: 无额外并发路径
- **边界条件**: 内容区域裁剪与可见行范围计算需保持不变
- **性能瓶颈**: 全背景填充的额外绘制成本
- **安全考虑**: 本任务不涉及安全边界变更

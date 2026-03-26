## 项目上下文摘要（ui-main-conflict-merge）
生成时间：2026-03-26 13:38:12 +0800

### 1. 相似实现分析
- **实现1**: `crates/ui/src/input/input.rs`
  - 模式：输入类组件统一通过主题色和 `input_style` 决定背景、前景与禁用态外观。
  - 可复用：`input_style(disabled, cx)` 可直接下沉到 `select`、`date_picker`、`otp_input` 等输入变体。
  - 需注意：禁用态在当前仓库倾向使用 `opacity(0.5)`，而不是每个组件单独切到 `muted`。

- **实现2**: `crates/ui/src/input/state.rs`
  - 模式：输入态更新集中在 `replace_text_in_range`、IME 路径和 `render` 前的 `_pending_update` 分支。
  - 可复用：`Task`、`background_executor`、`cx.spawn_in` 已提供后台工作调度能力。
  - 需注意：高亮器更新必须兼容当前分支已有 `InputMode::CodeEditor` 结构，不能引入不存在的 wasm stub。

- **实现3**: `crates/ui/src/highlighter/highlighter.rs`
  - 模式：语法树、注入层和样式匹配都集中在 `SyntaxHighlighter` 内部维护。
  - 可复用：`update`、`apply_background_tree`、`compute_injection_layers` 形成同步解析 + 后台补完的闭环。
  - 需注意：当前分支没有上游同版本的 `mix_oklab` 和 wasm 高亮入口，需要做兼容适配。

### 2. 项目约定
- **命名约定**: 输入类组件继续沿用现有英文 API，如 `input_style`、`input_background`、`dispatch_background_parse`。
- **文件组织**: 语法高亮逻辑留在 `crates/ui/src/highlighter` 与 `crates/ui/src/input`；主题样式留在 `crates/ui/src/theme`。
- **导入顺序**: 保持标准库、第三方、工作区模块的现有排序。
- **代码风格**: 以最小侵入方式手工迁移 main 提交，不额外重构消费方。

### 3. 可复用组件清单
- `crates/ui/src/input/input.rs`: `input_style(disabled, cx)` 统一输入类背景与前景色。
- `crates/ui/src/theme/mod.rs`: `input_background()` 与 `editor_background()` 负责主题回退关系。
- `crates/ui/src/input/state.rs`: 后台解析任务派发与输入态更新入口。
- `crates/ui/src/highlighter/highlighter.rs`: 语法树更新、注入层构建和后台结果回填。

### 4. 测试策略
- **测试框架**: 本次以 `cargo check` 做集成编译验证。
- **测试模式**: 先手工补丁迁移，再用 `env CLANG_MODULE_CACHE_PATH=/tmp/clang-cache cargo check -p main` 验证主 crate 及依赖链可编译。
- **参考文件**: `crates/ui/src/input/input.rs`、`crates/ui/src/input/state.rs`、`crates/ui/src/highlighter/highlighter.rs`。
- **覆盖要求**: 至少确认输入样式链路、高亮器 API 调整和主 crate 编译三者同时成立。

### 5. 依赖和集成点
- **内部依赖**: `InputMode::update_highlighter` 依赖 `SyntaxHighlighter::update` 的新超时参数；`InputState` 负责后台解析调度。
- **消费方依赖**: `select`、`date_picker`、`otp_input` 等输入变体复用统一主题接口，不改外部调用方式。
- **配置来源**: 编译验证继续使用 `CLANG_MODULE_CACHE_PATH=/tmp/clang-cache` 规避本地 clang 模块缓存路径问题。

### 6. 技术选型理由
- **为什么用这个方案**: 直接 cherry-pick 会冲突，因此采用“保留现有 dev 结构 + 手工迁移核心优化”的最小风险方案。
- **优势**: 可以得到 `#2128` 的输入性能收益和 `#2135` 的统一输入外观，同时避免拉入当前分支缺失的 wasm 链路。
- **劣势和风险**: `input_background` 在当前分支只能用 `mix` 近似替代上游 `mix_oklab`，视觉效果可能与 main 略有差异。

### 7. 关键风险点
- **接口差异**: `SyntaxHighlighter` 在当前分支不可直接 `Clone`，只能传递 `Rc<RefCell<Option<_>>>` 容器。
- **平台差异**: wasm 高亮 stub 未迁入，因此后台解析仅在非 wasm 路径启用。
- **视觉差异**: 主题混色 API 不一致，深色模式输入背景与 main 可能存在轻微色差。

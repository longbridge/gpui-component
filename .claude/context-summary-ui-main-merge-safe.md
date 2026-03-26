## 项目上下文摘要（main-ui-safe-merge）
生成时间：2026-03-26 12:10:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/ui/src/root.rs`
  - 模式：窗口根容器统一管理对话框、抽屉和边框绘制。
  - 可复用：`Root` 的窗口阴影、焦点恢复和弹层状态管理。
  - 需注意：`root.rs` 被 `sheet` 和 `window_border` 同时依赖，适合只回迁局部修复。

- **实现2**: `crates/ui/src/sheet.rs`
  - 模式：抽屉组件在 overlay 和焦点恢复链路上与 `Root` 协作。
  - 可复用：现有 `open_sheet` 行为与焦点恢复逻辑。
  - 需注意：`sheet` 可单独回迁，但不能和 `dialog/table/time picker` 大改混合处理。

- **实现3**: `crates/ui/src/window_border.rs`
  - 模式：边框、阴影、窗口 resize 命中区域集中在单文件处理。
  - 可复用：窗口阴影尺寸与 tiling 判断逻辑。
  - 需注意：适合直接回迁局部平台修复。

### 2. 项目约定
- **命名约定**: UI 基础库继续沿用上游英文标识，提交信息保持既有 git 历史风格。
- **文件组织**: 基础组件在 `crates/ui/src/*`，消费方在 `main` 与各 `*_view`/`one_ui` crate。
- **导入顺序**: 优先标准库，再 workspace/第三方，最后本地模块。
- **代码风格**: 以最小改动复用上游提交，不做额外重构。

### 3. 可复用组件清单
- `crates/ui/src/tree.rs`: Tree 焦点能力入口。
- `crates/ui/src/root.rs`: 窗口阴影、sheet 焦点恢复。
- `crates/ui/src/window_border.rs`: 阴影与 resize 命中区域逻辑。
- `crates/ui/src/notification.rs`: 通知点击行为。
- `crates/ui/src/button/button.rs`: 按钮标签布局。

### 4. 测试策略
- **测试框架**: 以 `cargo check` 为主，当前任务没有独立自动化单测入口。
- **测试模式**: 先在临时 worktree 演练 `git cherry-pick`，再在当前分支做 `cargo check -p main`。
- **参考文件**: `crates/story/src/stories/*` 仅用于观察 UI 组件用法，不作为本次直接修改目标。
- **覆盖要求**: 至少覆盖“能否无冲突回迁”和“当前 main crate 是否可编译”。

### 5. 依赖和集成点
- **内部依赖**: `sheet` 修复依赖 `root.rs` 焦点管理；`window_border` 修复依赖窗口 tiling 状态。
- **消费方依赖**: `one_ui` 和 `db_view` 仍依赖 `datetime_picker/time_picker`，因此相关 main 提交不能直接回迁。
- **配置来源**: 本次没有新增配置，编译时使用 `CLANG_MODULE_CACHE_PATH=/tmp/clang-cache` 规避沙箱缓存路径限制。

### 6. 技术选型理由
- **为什么用这个方案**: 只回迁已在临时 worktree 证明可直接 cherry-pick 的提交，风险最低。
- **优势**: 不触碰 `table`、`dialog`、`time picker`、`WASM` 等高风险接口面。
- **劣势和风险**: 仍有大量 main UI 优化未回迁，需要后续分主题处理。

### 7. 关键风险点
- **接口破坏**: `main` 的 `Table -> DataTable`、移除 `datetime_picker/time_picker` 会打断现有业务 crate。
- **冲突热点**: `crates/ui/Cargo.toml`、`input/*`、`text/*`、`dialog/*`、`theme/*` 在 dev/main 双方都改过。
- **验证限制**: `gpui` 在默认沙箱下会写 `~/.cache/clang/ModuleCache`，需要改用 `/tmp` 缓存路径。

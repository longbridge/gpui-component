## 项目上下文摘要（import-export-no-dialog）
生成时间：2026-03-06 15:00:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/import_export/sql_dump_view.rs:678`
  - 模式：完成态按钮直接 `window.remove_window()` 关闭窗口
  - 可复用：无需弹窗，直接完成关闭
  - 需注意：保持 `is_finished` 条件渲染不变

- **实现2**: `crates/db_view/src/import_export/table_export_view.rs:1346`
  - 模式：导出完成后 `Button::new("close")` + `window.remove_window()`
  - 可复用：Import/Export 视图统一关闭行为
  - 需注意：保持按钮文案与主题风格一致

- **实现3**: `crates/db_view/src/import_export/sql_run_view.rs:683`
  - 模式：执行完成后关闭窗口，无 dialog 依赖
  - 可复用：目标约束“不能加 dialog”已在该文件满足
  - 需注意：只做合规确认，不引入额外改动

### 2. 项目约定
- **命名约定**: Rust 函数/变量 `snake_case`，类型 `PascalCase`
- **文件组织**: import_export 下每个视图自包含状态与渲染
- **导入顺序**: `std`、外部 crate、本地模块分组
- **代码风格**: GPUI 链式构建 + `cx.notify()` 显式刷新
### 3. 可复用组件清单
- `TableImportView::add_log(...)`：现有日志追加入口
- `VirtualListScrollHandle::scroll_to_bottom()`：日志区域滚动到底
- `window.remove_window()`：同模块统一关闭方式

### 4. 测试策略
- **测试框架**: Rust `cargo check` / `cargo test`
- **当前任务验证**: 先做 `cargo check -p db_view`
- **补充校验**: 代码搜索确认目标文件不含 `dialog`

### 5. 依赖和集成点
- **外部依赖**: `gpui`、`gpui_component`
- **内部依赖**: `db::GlobalDbState` 的导入流程不改
- **集成方式**: 仅修改 UI 触发与关闭逻辑，不改异步导入链路

### 6. 技术选型理由
- 采用“日志提示 + 直接执行”而非确认弹窗
- 原因：满足“不能加 dialog”约束，并与 import_export 现有模式一致

### 7. 关键风险点
- 去掉确认弹窗后，`truncate_before=true` 的误触发风险上升
- 缓解：在启动前追加明显日志提示并保持按钮步骤不变

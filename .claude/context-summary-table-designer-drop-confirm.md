## 项目上下文摘要（table-designer-drop-confirm）
生成时间：2026-03-27 17:02:24 +0800

### 1. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:592`
  - 模式：`build_diff_preview_sql -> update_previews -> handle_execute/save_and_close`
  - 可复用：`normalize_column_renames`、`sql_preview_input`、`active_tab`
  - 需注意：保存与关闭保存原本各自维护一套执行链路，容易行为漂移

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/schema_editor_view.rs:141`
  - 模式：通过本地 tab 状态在“表单/SQL 预览”之间切换
  - 可复用：页签切换只修改状态并 `cx.notify()`
  - 需注意：预览页是只读视图，不应引入第二套 SQL 来源

- **实现3**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_event.rs:1050`
  - 模式：破坏性操作统一使用 `window.open_dialog(...).confirm()` 展示二段式提示
  - 可复用：`DialogButtonProps`、`Common.irreversible`、`overlay(false)`
  - 需注意：确认后必须回到既有执行链路，而不是在弹窗里重写业务

### 2. 项目约定
- **命名约定**: Rust 使用 `snake_case`，结构体/枚举使用 `PascalCase`
- **文件组织**: 表设计器逻辑集中在 `crates/db_view/src/table_designer_tab.rs`，i18n 文案集中在 `crates/db_view/locales/db_view.yml`
- **代码风格**: 通过小型 helper 收口重复逻辑，避免在 UI 事件里展开大段异步执行代码
- **导入方式**: 继续沿用 `gpui_component` 现有组件，不新增 UI 抽象层

### 3. 可复用组件清单
- `crates/db_view/src/table_designer_tab.rs`: `build_diff_preview_sql`、`update_previews`、`TableDesignerEvent::Saved`
- `crates/db_view/src/common/schema_editor_view.rs`: SQL 预览页签切换模式
- `crates/db_view/src/db_tree_event.rs`: 破坏性确认弹窗模板
- `crates/db_view/locales/db_view.yml`: `Common.cancel`、`Common.irreversible`

### 4. 测试策略
- **测试框架**: Rust `#[test]`
- **参考文件**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:3116`
- **覆盖重点**: 危险 SQL 文本识别、No changes/普通 ALTER 不误判、原有 table_designer 模块回归

### 5. 依赖和集成点
- **外部依赖**: `gpui`、`gpui_component`
- **内部依赖**: `GlobalDbState.execute_script`、`TabContainer.force_close_tab_by_id`、`TableDesignerEvent::Saved`
- **集成方式**: 执行前统一生成 diff SQL；危险 SQL 时先切到 `DesignerTab::SqlPreview` 再弹确认框

### 6. 外部资料检索记录
- **github.search_code**: 查询 `"DROP COLUMN" confirm preview sql language:Rust`
- **结论**: 没有直接可复用的成熟 UI 模式结果，采用仓库内既有 destructive confirm 模板更稳妥，外部检索仅作为“显式 DROP 语句需谨慎确认”的佐证

### 7. 关键风险点
- `save_and_close` 若不接入同一保护链路，会绕过确认
- SQLite 等方言可能通过 `DROP TABLE` 重建表，仍应视为破坏性 SQL
- 当前缺少 UI 自动化测试，交互弹窗需依赖代码审查和后续人工冒烟补充

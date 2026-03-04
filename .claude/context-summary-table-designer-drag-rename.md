## 项目上下文摘要（table-designer-drag-rename）
生成时间：2026-03-04 19:17:57 +0800

### 1. 相似实现分析
- **实现1**: /Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:1980
  - 模式：表格行级交互 + 拖拽排序
  - 可复用：`DragColumn`、`move_column`
  - 需注意：当前 `on_drag` 绑定在整行，导致输入框交互可触发拖拽

- **实现2**: /Users/hufei/RustroverProjects/onetcli/crates/ui/src/table/state.rs:1298
  - 模式：可移动列头拖拽
  - 可复用：`on_drag` 回调中调用 `cx.stop_propagation()`
  - 需注意：通过条件 `.when(movable, ...)` 限定拖拽范围

- **实现3**: /Users/hufei/RustroverProjects/onetcli/crates/one_ui/src/edit_table/state.rs:2045
  - 模式：编辑表头拖拽与 drop 处理
  - 可复用：`drag_over` + `on_drop` 组合
  - 需注意：拖拽实体需要校验 `entity_id` 防串扰

### 2. 项目约定
- **命名约定**: Rust `snake_case`（函数/变量），类型 `PascalCase`
- **文件组织**: 表设计器主要集中在 `crates/db_view/src/table_designer_tab.rs`
- **导入顺序**: 先外部 crate，再 `std`，再本地 `crate`
- **代码风格**: 事件驱动（`subscribe/observe`）+ `cx.emit(...)` 通知刷新

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs`: `DragColumn`, `ColumnsEditor`, `move_column`
- `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/table/state.rs`: `on_drag` + `cx.stop_propagation` 模式
- `/Users/hufei/RustroverProjects/onetcli/crates/db/src/*/plugin.rs`: 各数据库 `build_alter_table_sql` 差异生成入口

### 4. 测试策略
- **测试框架**: Rust 内置 `#[test]`
- **测试模式**: 以插件单元测试为主（`build_alter_table_sql`）
- **参考文件**:
  - `/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/plugin.rs:2040`
  - `/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/plugin.rs:1887`
  - `/Users/hufei/RustroverProjects/onetcli/crates/db/src/sqlite/plugin.rs:1291`
- **覆盖要求**: 正常路径 + 删除/重命名冲突路径 + 无变更路径

### 5. 依赖和集成点
- **外部依赖**: `gpui`, `gpui_component`
- **内部依赖**: `GlobalDbState.db_manager.get_plugin(...)` + `plugin.build_alter_table_sql(...)`
- **集成方式**: `TableDesigner.collect_design -> update_sql_preview/save/execute`
- **配置来源**: `TableDesignerConfig`（`database_type/table_name/schema_name`）

### 6. 技术选型理由
- **为什么用该方案**: 不改数据库插件大面积逻辑，优先在设计器层补充“列来源映射”并拼接 rename SQL
- **优势**: 改动面小、风险低、对现有插件兼容性高
- **劣势和风险**: 不同数据库 rename 语法差异，需要分支处理与测试兜底

### 7. 关键风险点
- **并发问题**: SQL 在异步保存路径生成，需确保捕获的映射与设计快照一致
- **边界条件**: 目标名冲突（如删除 `a` 后 `b->a`）
- **性能瓶颈**: 仅线性遍历列/索引，风险低
- **安全考虑**: 名称转义需复用插件 `quote_identifier` 或适配数据库语法

### 8. 外部资料检索记录
- **context7**: `/websites/rs_gpui_gpui`，确认 `stop_propagation` 与 `on_drag`/`on_drop` 事件模型
- **github.search_code**: `on_drag cx.stop_propagation language:Rust`，对照 zed/gpui-component 相关实践

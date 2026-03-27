## 项目上下文摘要（db_tree 复制表）
生成时间：2026-03-27 16:08:04 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/db_tree_event.rs`
  - 模式：`RenameTable` 使用对话框输入新名称，再走异步事件处理。
  - 可复用：输入框对话框布局、通知提示、树刷新模式。
  - 需注意：异步执行后要刷新数据库树，不要只更新当前节点文本。

- **实现2**: `crates/db_view/src/table_designer_tab.rs`
  - 模式：读取 `ColumnInfo` / `IndexInfo` 后组装 `TableDesign`，再交给数据库插件生成 SQL。
  - 可复用：列类型解析、索引转换、`build_create_table_sql` 调用链。
  - 需注意：不要改动 `TableDesigner` 的“新建/修改”保存语义。

- **实现3**: `crates/db_view/src/mysql/mysql_view_plugin.rs`、`postgresql_view_plugin.rs`、`mssql_view_plugin.rs`、`oracle_view_plugin.rs`、`sqlite_view_plugin.rs`、`clickhouse_view_plugin.rs`
  - 模式：表右键菜单由各数据库 UI 插件分别构建。
  - 可复用：统一插入新的 `ContextMenuItem::item(...)`，挂接 `DbTreeViewEvent`。
  - 需注意：不能只改 `db_tree_view.rs`，否则右键菜单不会出现新入口。

### 2. 项目约定
- **命名约定**: Rust 使用 `snake_case` / `PascalCase`；事件枚举使用 `VerbNoun` 风格。
- **文件组织**: 视图事件在 `db_tree_event.rs`，菜单入口在各数据库 `*_view_plugin.rs`，本地化文案在 `locales/db_view.yml`。
- **导入顺序**: 先 crate 内模块，再 `db` 依赖，再 `gpui` / `gpui_component` / 第三方。
- **代码风格**: 复用现有 helper 和异步执行模式，避免新造数据库复制接口。

### 3. 可复用组件清单
- `crates/db_view/src/db_tree_event.rs`：`handle_rename_table`、通知和树刷新模式。
- `crates/db_view/src/table_designer_tab.rs`：`ColumnInfo` / `IndexInfo` 到 `TableDesign` 的转换逻辑。
- `crates/db/src/manager.rs`：`list_columns`、`list_indexes`、`execute_script`。
- `crates/db/src/plugin.rs`：`parse_column_type`、`build_create_table_sql`。

### 4. 测试策略
- **测试框架**: `cargo test -p db_view --lib`
- **测试模式**: 以库测试和编译检查为主，覆盖改动后的 `db_view` 代码路径不回归。
- **参考文件**: `crates/db_view/src/table_designer_tab.rs` 现有单测。
- **覆盖要求**: 至少完成 `cargo check -p db_view` 和 `cargo test -p db_view --lib`。

### 5. 依赖和集成点
- **外部依赖**: `gpui`、`gpui_component`。
- **内部依赖**: `GlobalDbState`、`DatabaseViewPluginRegistry`、`DatabaseObjectsPanel`。
- **集成方式**: 菜单事件发射到 `DatabaseEventHandler`，再通过 `GlobalDbState` 执行数据库操作。
- **配置来源**: `DbNode` 的 `database_type`、`database` / `schema` 元数据。

### 6. 技术选型理由
- **为什么用这个方案**: 用户澄清语义是“创建备份表”，因此应优先走数据库原生 SQL，而不是事件层重建表结构。
- **优势**: 备份语义下沉到数据库插件，事件层只负责收集目标表名和执行 SQL；更符合各数据库方言能力。
- **劣势和风险**: 不同数据库原生备份语法保留约束/索引的能力不同，语义存在方言差异。

### 7. 关键风险点
- **并发问题**: 异步执行后必须在 UI 线程关闭对话框并刷新树。
- **边界条件**: 新表名为空或与原表同名时不能提交。
- **性能瓶颈**: 备份表会执行结构复制和数据复制，源表较大时耗时取决于数据库端执行。
- **安全考虑**: 本次任务不新增额外安全逻辑，保持既有执行链路。

### 8. 支持范围（按当前已注册数据库插件）
- **MySQL**: `CREATE TABLE ... LIKE ...; INSERT INTO ... SELECT * ...;`
- **PostgreSQL**: `CREATE TABLE ... (LIKE ... INCLUDING ALL); INSERT INTO ... SELECT * ...;`
- **MSSQL**: `SELECT * INTO ... FROM ...;`
- **Oracle**: `CREATE TABLE ... AS SELECT * FROM ...;`
- **SQLite**: `CREATE TABLE ... AS SELECT * FROM ...;`
- **ClickHouse**: `CREATE TABLE ... AS ...; INSERT INTO ... SELECT * ...;`

## 项目上下文摘要（mssql-dump-sql-menu）
生成时间：2026-03-07 22:14:00 +0800

### 1. 相似实现分析
- **实现1**: `crates/db_view/src/mssql/mssql_view_plugin.rs`
  - 模式：各数据库插件通过 `build_context_menu` 手工声明树节点菜单。
  - 现状：MSSQL 只有 `RunSqlFile`、`ImportData`、`ExportData`，没有 `DumpSqlFile`。
  - 需注意：保持菜单顺序与其他数据库一致。

- **实现2**: `crates/db_view/src/mysql/mysql_view_plugin.rs`
  - 模式：在 `Database`、`Table` 节点用 `ContextMenuItem::submenu` 暴露 `DumpSqlFile`。
  - 可复用：三种模式 `StructureOnly/DataOnly/StructureAndData` 的菜单组织。

- **实现3**: `crates/db_view/src/postgresql/postgresql_view_plugin.rs`
  - 模式：与 MySQL 相同，直接复用 `DbTreeViewEvent::DumpSqlFile`。
  - 可复用：菜单摆放位置与分隔符顺序。

### 2. 依赖与调用链
- `crates/db_view/src/db_tree_view.rs`：定义 `DbTreeViewEvent::DumpSqlFile` 与 `SqlDumpMode`
- `crates/db_view/src/db_tree_event.rs`：实现 `handle_dump_sql_file`
- `crates/db_view/src/import_export/sql_dump_view.rs`：构造 `ExportConfig { format: DataFormat::Sql }`
- `crates/db/src/plugin.rs`：将 SQL 导出路由到 `SqlFormatHandler`

### 3. 测试策略
- 优先执行 `cargo fmt --all`
- 执行 `cargo check -p db_view`
- 本次为菜单接线改动，不新增单元测试

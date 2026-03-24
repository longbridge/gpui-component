## 项目上下文摘要（db-tree-csv-import-target）
生成时间：2026-03-24 18:20:00 +0800

### 1. 相似实现分析
- 实现1：`crates/db/src/plugin.rs:1307`
  - 模式：统一通过 `DatabasePlugin::format_table_reference(database, schema, table)` 生成查询目标表。
  - 可复用：表数据查询、分页查询都依赖这一抽象。
  - 需注意：不同数据库插件会覆盖默认实现，不能手写库名/模式名拼接。

- 实现2：`crates/db/src/plugin.rs:1952`
  - 模式：导出 SQL 时先用完整表引用查询，再用导出专用表引用生成输出。
  - 可复用：导入侧同样应该先定位到完整目标表，而不是依赖连接默认库。
  - 需注意：导出和导入对“表定位”的语义必须一致。

- 实现3：`crates/db/src/import_export/formats/csv.rs:307`
  - 模式：CSV 导出使用 `format_table_reference(&config.database, None, table)`。
  - 可复用：导入路径应复用相同的目标表定位策略。
  - 需注意：当前导入与导出行为不对称，是本次缺陷的直接信号。

- 实现4：`crates/db/src/import_export/formats/json.rs:202`、`crates/db/src/import_export/formats/txt.rs:220`
  - 模式：JSON/TXT 导出也统一使用完整表引用。
  - 可复用：说明 `import_export/formats/*` 内已有稳定约定。
  - 需注意：导入格式处理器存在成组偏差，应整体修正。

### 2. 调用链与集成点
- UI 入口：`crates/db_view/src/db_tree_event.rs:811`
  - `handle_import_data` 从树节点提取 `connection_id/database/schema/table` 并传给 `TableImportView::new`。
- 视图配置：`crates/db_view/src/import_export/table_import_view.rs:204`
  - `TableImportView::new` 保存 `database/schema/table`。
- 导入执行：`crates/db_view/src/import_export/table_import_view.rs:464`
  - `start_import` 将 `database/schema/table` 写入 `ImportConfig`。
- 后端分发：`crates/db/src/manager.rs:2087`
  - `import_data_with_progress_sync` 只负责创建连接会话并调用插件导入，不重写 SQL。

### 3. 项目约定
- 命名约定：Rust 函数/变量使用 `snake_case`，类型使用 `PascalCase`。
- 文件组织：导入导出按格式拆在 `crates/db/src/import_export/formats/*`。
- 复用规则：表定位必须走 `DatabasePlugin` 抽象，避免在格式处理器里手写数据库特定 SQL。
- 测试风格：模块内 `#[cfg(test)] mod tests`，使用 Rust 内置 `#[test]`。

### 4. 可复用组件清单
- `crates/db/src/plugin.rs`：`DatabasePlugin::format_table_reference`
- `crates/db/src/import_export/mod.rs`：`ImportConfig { database, schema, table }`
- `crates/db/src/mysql/plugin.rs`：`MySqlPlugin::new()`，可用于断言库级表引用
- `crates/db/src/mssql/plugin.rs`：`MsSqlPlugin::new()`，可用于断言库+模式级表引用

### 5. 测试策略
- 测试框架：Rust 内置单元测试
- 参考模式：`crates/db/src/mssql/plugin.rs:1962` 的模块内测试
- 本次验证：
  - 为导入目标表引用 helper 添加单元测试
  - 覆盖 MySQL 的 `database.table`
  - 覆盖 MSSQL 的 `database.schema.table`
  - 执行最小范围 `cargo test -p db import_export::formats::tests`

### 6. 技术选型理由
- 为什么用统一 helper：
  - 事实：导入的 `database/schema/table` 已经在 `ImportConfig` 中存在。
  - 事实：查询与导出路径都依赖 `format_table_reference`。
  - 推论：最小且一致的修复方式是在导入格式处理器内部复用同一抽象。
- 优势：
  - 不改 UI 和 manager 接口
  - 可一次性修复 CSV/TXT/JSON/SQL 同类问题
- 风险：
  - 会改变过去错误写入默认库的行为，但这是缺陷修复的预期结果

### 7. 关键风险点
- 边界条件：`schema` 为 `None` 时不能破坏 MySQL/MSSQL 现有格式化规则。
- 扩散风险：不能误改导出逻辑或不支持导入的 XML 分支。
- 验证风险：若测试仅覆盖 helper，需确保所有导入格式处理器都切换到 helper。

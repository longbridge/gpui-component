## 项目上下文摘要（全数据库 SQL 生成测试集）
生成时间：2026-03-27 00:30:00

### 1. 相似实现分析
- **实现1**: crates/db/src/mysql/plugin.rs:2140-2320
  - 模式：ALTER TABLE 测试覆盖新增/删除/修改/排序
  - 可复用：断言 SQL 片段（ADD/DROP/MODIFY、FIRST/AFTER）
  - 需注意：MySQL 支持列顺序调整

- **实现2**: crates/db/src/postgresql/plugin.rs:1910-2040
  - 模式：ALTER TABLE 测试覆盖新增/删除/修改类型
  - 可复用：ALTER COLUMN TYPE/SET NOT NULL/SET DEFAULT 断言
  - 需注意：PostgreSQL 不处理列顺序

- **实现3**: crates/db/src/mssql/plugin.rs:2240-2330
  - 模式：ALTER TABLE 测试覆盖新增/删除
  - 可复用：ALTER COLUMN/ADD INDEX 断言
  - 需注意：标识符使用 []

- **实现4**: crates/db/src/oracle/plugin.rs:1960-2070
  - 模式：ALTER TABLE 测试覆盖新增/删除
  - 可复用：MODIFY DEFAULT/NULL 断言
  - 需注意：标识符使用 ""

- **实现5**: crates/db/src/sqlite/plugin.rs:1290-1400
  - 模式：ALTER TABLE 测试覆盖新增/删除，重建表
  - 可复用：_dg_tmp/rename to 断言
  - 需注意：结构变更走重建路径

- **实现6**: crates/db/src/clickhouse/plugin.rs:1280-1375
  - 模式：ALTER TABLE 测试覆盖新增/删除
  - 可复用：ADD INDEX/MODIFY COLUMN 断言
  - 需注意：索引通过 ALTER TABLE 单独添加

### 2. 项目约定
- **命名约定**: Rust snake_case 函数/变量，结构体 CamelCase
- **文件组织**: 各数据库插件位于 crates/db/src/<db>/plugin.rs
- **导入顺序**: 标准 Rust use 分组
- **代码风格**: 断言关键 SQL 片段，避免过度严格匹配

### 3. 可复用组件清单
- 各插件的 build_alter_table_sql 与既有测试模式

### 4. 测试策略
- **测试框架**: Rust 内置 #[test]
- **测试模式**: 单元测试，断言 SQL 片段
- **覆盖要求**: 新增/删除/修改/索引/默认值/顺序差异

### 5. 依赖和集成点
- **外部依赖**: 无
- **内部依赖**: TableDesign/ColumnDefinition/IndexDefinition
- **集成方式**: 仅在插件测试模块新增用例

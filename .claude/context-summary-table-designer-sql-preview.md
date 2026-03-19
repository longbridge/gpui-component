## 项目上下文摘要（table-designer-sql-preview）
生成时间：2026-03-19 18:35:56 +0800

### 1. 相似实现分析
- **实现1**: /Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:367
  - 模式：`collect_design -> update_sql_preview -> plugin.build_alter_table_sql_with_renames`
  - 可复用：`normalize_column_renames`、`collect_design`
  - 需注意：SQL 预览完全依赖 `original_design` 与当前设计的语义一致性

- **实现2**: /Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:1667
  - 模式：列编辑器把界面状态回收为 `ColumnDefinition`
  - 可复用：`ColumnsEditor::get_columns`、`ColumnsEditor::load_columns`
  - 需注意：当前界面态会保留 `charset/collation`，但未显式暴露 `is_unsigned` 控件

- **实现3**: /Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/plugin.rs:392
  - 模式：从 MySQL 信息架构读取列元数据，再由 `column_changed` 决定是否生成 `MODIFY COLUMN`
  - 可复用：`list_columns`、`parse_column_type`、`build_alter_table_sql`
  - 需注意：`column_changed` 会比较 `is_unsigned/default_value/comment/charset/collation`

### 2. 项目约定
- **命名约定**: Rust 函数和变量使用 `snake_case`，结构体和枚举使用 `PascalCase`
- **文件组织**: 表设计器逻辑集中在 `crates/db_view/src/table_designer_tab.rs`，数据库方言 SQL 逻辑位于 `crates/db/src/*/plugin.rs`
- **导入顺序**: 先第三方 crate，再 `std`，再本地 `crate`
- **代码风格**: 事件驱动更新 + 小范围辅助函数，优先复用现有插件能力

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs`: `collect_design`、`ColumnsEditor::load_columns`、`ColumnsEditor::get_columns`
- `/Users/hufei/RustroverProjects/onetcli/crates/db/src/plugin.rs`: `parse_column_type` 默认归一化逻辑
- `/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/plugin.rs`: `list_columns`、`build_alter_table_sql`、MySQL 测试模块

### 4. 测试策略
- **测试框架**: Rust 内置 `#[test]`
- **测试模式**: MySQL 插件 DDL 生成单元测试
- **参考文件**:
  - `/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/plugin.rs:1778`
  - `/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/plugin.rs:1431`
  - `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:723`
- **覆盖要求**: 无变更路径 + 真实字段变更路径 + 列级字符集/排序规则保真

### 5. 依赖和集成点
- **外部依赖**: `gpui`, `gpui_component`
- **内部依赖**: `GlobalDbState.db_manager.get_plugin(...)`、`DatabasePlugin::parse_column_type`、`build_alter_table_sql_with_renames`
- **集成方式**: `load_table_structure` 构建 `original_design`，`update_sql_preview/save` 基于 diff 生成 SQL
- **配置来源**: `TableDesignerConfig` 中的 `database_type/table_name/schema_name`

### 6. 技术选型理由
- **为什么用这个方案**: 问题源于设计器层归一化不一致，直接在 `TableDesigner` 收口修复比修改所有数据库插件的通用 diff 更小、更可控
- **优势**: 改动面小，复用现有 `parse_column_type`，可以一次性修复 `charset/collation/is_unsigned` 多类误判
- **劣势和风险**: 需要保证界面态与原始态转换保持一致，否则仍可能在其他列属性上产生伪变更

### 7. 关键风险点
- **并发问题**: 无，主要为同步状态构建
- **边界条件**: `ENUM/SET`、`UNSIGNED`、列级 `charset/collation`、SQLite 自增语义
- **性能瓶颈**: 仅为列数组线性转换，风险低
- **安全考虑**: 不涉及新增安全逻辑，本次仅修正本地状态归一化

### 8. 外部资料检索记录
- **github.search_code**: 查询 `mysql column collation charset compare schema diff language:Rust`，得到通用数据库抽象项目示例，结论是列级字符集/排序规则属于 schema diff 的一部分，必须在归一化阶段保真

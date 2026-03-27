## 项目上下文摘要（table_designer 字段排序不生成语句）
生成时间：2026-03-27 00:00:00

### 1. 相似实现分析
- **实现1**: crates/db/src/mysql/plugin.rs:1560-1710
  - 模式：基于原始/新设计对比生成 ALTER TABLE 语句，新增列使用 FIRST/AFTER 控制位置
  - 可复用：build_alter_table_sql 的列差异检测与 ADD/MODIFY 语句构造
  - 需注意：当前仅在新增列时处理位置，未处理“仅排序变化”的场景

- **实现2**: crates/db/src/mssql/plugin.rs:1830-1970
  - 模式：基于列/索引增删改生成 ALTER TABLE 语句
  - 可复用：column_changed 判断列属性变化、增删改策略
  - 需注意：不支持列顺序调整

- **实现3**: crates/db/src/postgresql/plugin.rs:1500-1670
  - 模式：基于列/索引增删改生成 ALTER TABLE 语句
  - 可复用：索引差异处理方式
  - 需注意：不支持列顺序调整

- **实现4**: crates/db_view/src/table_designer_tab.rs:340-520、1600-1705、2160-2270
  - 模式：拖拽移动列触发 ColumnsEditorEvent::Changed → update_sql_preview
  - 可复用：move_column 维护 columns 顺序并触发变更事件
  - 需注意：SQL 预览依赖插件 build_alter_table_sql_with_renames 的 diff 结果

### 2. 项目约定
- **命名约定**: Rust snake_case 函数/变量，结构体 CamelCase
- **文件组织**: db 侧插件位于 crates/db/src/<db>/plugin.rs；UI 侧位于 crates/db_view/src/table_designer_tab.rs
- **导入顺序**: 标准 Rust use 分组
- **代码风格**: 以显式逻辑为主，尽量少副作用

### 3. 可复用组件清单
- `crates/db/src/plugin.rs`: build_alter_table_sql_with_renames / merge_alter_sql
- `crates/db/src/mysql/plugin.rs`: build_alter_table_sql（含 FIRST/AFTER 位置）
- `crates/db_view/src/table_designer_tab.rs`: ColumnsEditor::move_column 与 update_sql_preview

### 4. 测试策略
- **测试框架**: Rust 内置 #[test]
- **测试模式**: 单元测试，直接断言 SQL 文本
- **参考文件**: crates/db/src/mysql/plugin.rs（已有 ALTER TABLE 相关测试）
- **覆盖要求**: 正常流程 + 排序变化场景 + 无变化场景
### 5. 依赖和集成点
- **外部依赖**: 无新增库需求
- **内部依赖**: TableDesign/ColumnDefinition（crates/db/src/types.rs）
- **集成方式**: TableDesigner 通过插件 build_alter_table_sql_with_renames 生成 SQL
- **配置来源**: DatabaseType + 插件注册（db_view）

### 6. 技术选型理由
- **为什么用这个方案**: 表设计器依赖插件生成 SQL，修复应集中在插件差异逻辑
- **优势**: 影响面可控、最小改动即可覆盖排序变更
- **劣势和风险**: 不同数据库对列排序支持程度不同，需要限定实现范围

### 7. 关键风险点
- **并发问题**: 无
- **边界条件**: 仅排序变化且无其他变更时应生成 SQL
- **性能瓶颈**: diff 仅在内存中遍历列集合，影响可忽略
- **安全考虑**: 不新增安全逻辑（遵循项目约束）

### 8. 外部参考
- **GitHub 搜索**: GreptimeTeam/greptimedb（alter_parser.rs）中存在 ALTER 语法相关实现，用于确认“MODIFY COLUMN ... AFTER”语义场景

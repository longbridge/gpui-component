## 项目上下文摘要（table_designer DDL 页签）
生成时间：2026-03-27 15:02:06 +0800

### 1. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:33`
  - 模式：`DesignerTab + TabBar + render_active_tab` 组织多页签视图。
  - 可复用：`render_tabs`、`render_sql_preview`、`collect_design`、`normalize_column_renames`。
  - 需注意：现有 `sql_preview_input` 语义是“差异 SQL”，`has_unsaved_changes` 直接依赖它，不能改成完整 DDL。

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/story/src/stories/tabs_story.rs:122`
  - 模式：`TabBar::new(...).selected_index(...).on_click(...).child(Tab::new())`。
  - 可复用：页签索引映射和点击切换写法。
  - 需注意：新增页签时必须同步更新索引映射和默认回退分支。

- **实现3**: `/Users/hufei/RustroverProjects/onetcli/main/src/encourage.rs:186`
  - 模式：`Clipboard::new(...).value(...)` 直接作为复制交互组件。
  - 可复用：复制按钮本身，不需要额外封装点击逻辑。
  - 需注意：复制内容应直接使用当前文本状态，避免拼接额外提示文案。

- **实现4**: `/Users/hufei/RustroverProjects/onetcli/crates/db/src/plugin.rs:2141`
  - 模式：数据库插件 trait 暴露 `build_create_table_sql(&TableDesign)`。
  - 可复用：完整表级 DDL 的统一生成入口。
  - 需注意：这是完整 DDL 生成能力，不应污染现有差异 SQL 预览路径。

- **实现5**: `/Users/hufei/RustroverProjects/onetcli/crates/db/src/mssql/plugin.rs:1763`
  - 模式：具体数据库插件返回 `CREATE TABLE` 和索引语句；测试区验证关键片段。
  - 可复用：插件层已有完整 DDL 正确性保障。
  - 需注意：UI 层只负责触发和展示，不新增插件接口。

### 2. 项目约定
- **命名约定**: Rust 函数和变量使用 `snake_case`，结构体和枚举使用 `PascalCase`。
- **文件组织**: 表设计器视图集中在 `crates/db_view/src/table_designer_tab.rs`，数据库方言 SQL 逻辑位于 `crates/db/src/*/plugin.rs`。
- **导入顺序**: 先第三方 crate，再 `std`，再本地 `crate`。
- **代码风格**: 事件驱动刷新 + 小函数收口；优先局部补丁，不扩散公共接口。

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs`: `collect_design`、`collect_column_renames`、`render_sql_preview`。
- `/Users/hufei/RustroverProjects/onetcli/crates/db/src/plugin.rs`: `build_create_table_sql` trait 接口。
- `/Users/hufei/RustroverProjects/onetcli/main/src/encourage.rs`: `Clipboard::new(...).value(...)` 复制模式。
- `/Users/hufei/RustroverProjects/onetcli/crates/story/src/stories/tabs_story.rs`: `TabBar` 标准用法。

### 4. 测试策略
- **测试框架**: Rust 内置 `cargo check` / `cargo test`。
- **测试模式**: 本次优先做 `db_view` 编译验证；完整 DDL 生成正确性依赖插件层现有单测。
- **参考文件**:
  - `/Users/hufei/RustroverProjects/onetcli/crates/db/src/mssql/plugin.rs:2188`
  - `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:449`
- **覆盖要求**: DDL 页签可见、完整 SQL 跟随设计器状态刷新、SQL 预览仍保持差异 SQL 语义。

### 5. 依赖和集成点
- **外部依赖**: `gpui`、`gpui_component`。
- **内部依赖**: `GlobalDbState.db_manager.get_plugin(...)`、`DatabasePlugin::build_create_table_sql(...)`、`build_alter_table_sql_with_renames(...)`。
- **集成方式**: `TableDesigner` 在表单变更订阅中刷新预览；`load_table_structure` 加载原始结构后刷新视图。
- **配置来源**: `TableDesignerConfig` 的 `database_type/table_name/schema_name`。

### 6. 技术选型理由
- **为什么用这个方案**: 直接在 `TableDesigner` 增加独立 DDL 预览状态，最小改动且不会破坏现有差异 SQL 保存逻辑。
- **优势**: 复用现有插件能力、UI 组件和页签模式；风险集中在单文件内。
- **劣势和风险**: 需要确保所有原有刷新入口都统一更新两个预览，避免加载后只有一个文本更新。

### 7. 关键风险点
- **并发问题**: `load_table_structure` 的异步回填必须在同一次 UI 更新中设置 `original_design` 后刷新预览。
- **边界条件**: 空表名或无列时不应展示伪造 DDL。
- **性能瓶颈**: 每次编辑同步生成两段 SQL，复杂度与列/索引数量线性相关，当前可接受。
- **安全考虑**: 本次不新增安全逻辑，仅调整本地 UI 展示。

### 8. 外部资料检索记录
- **context7**: 查询 `/longbridge/gpui-component`，确认 `InputState::code_editor + multi_line + disabled` 适合作为只读代码展示。
- **github.search_code**: 查询 `\"build_create_table_sql\" language:Rust`，结果表明开源项目中也普遍把完整建表 SQL 作为独立生成能力；另一个更宽泛的 Clipboard 查询因远端 fetch 失败，已记录并回退到仓库内现成实现。 

## 项目上下文摘要（table-designer-preview-lag）
生成时间：2026-03-28 16:49:00 +0800

### 1. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:379`
  - 模式：`subscribe_in/observe_in -> update_previews -> collect_design -> build_*_preview_sql`
  - 可复用：`collect_design`、`collect_column_renames`、`build_diff_preview_sql`、`build_ddl_preview_sql`
  - 需注意：当前实现由父实体在事件回调里主动回读多个子实体状态，存在时序风险

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/database_editor_view.rs:56`
  - 模式：表单先发出 `DatabaseFormEvent::FormChanged(request)`，父视图直接基于事件载荷更新 SQL 预览
  - 可复用：集中式预览刷新思路，而不是在父视图里跨层重新遍历所有控件
  - 需注意：这里没有“回读嵌套子实体聚合状态”的步骤，链路更稳定

- **实现3**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/schema_editor_view.rs:50`
  - 模式：`SchemaFormEvent::FormChanged(request)` 驱动 `update_sql_preview`
  - 可复用：事件驱动、单入口刷新预览
  - 需注意：同样避免了在嵌套事件栈中聚合读取多个控件状态

- **实现4**: `/Users/hufei/RustroverProjects/onetcli/main/src/onetcli_app.rs:117`
  - 模式：使用 `cx.defer(...)` 把 UI 操作延后到当前效果周期之后执行
  - 可复用：延后调度写法与“等待当前更新栈回收后再操作”的思路
  - 需注意：项目内已有延后执行模式，可作为本次修复的风格依据

### 2. 项目约定
- **命名约定**: Rust 函数和变量使用 `snake_case`，结构体和枚举使用 `PascalCase`
- **文件组织**: 表设计器与其测试集中在 `crates/db_view/src/table_designer_tab.rs`
- **导入顺序**: 先第三方 crate，再本地 `crate`
- **代码风格**: 事件驱动刷新 + 局部辅助函数，优先最小改动，不扩散公共接口

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs`: `collect_design`、`collect_column_renames`、`update_previews`
- `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/database_editor_view.rs`: `update_sql_preview`
- `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/schema_editor_view.rs`: `update_sql_preview`
- `/Users/hufei/RustroverProjects/onetcli/crates/ui/src/input/state.rs`: `InputState::set_value`、`replace_text_in_range`

### 4. 测试策略
- **测试框架**: Rust 内置 `#[test]`
- **测试模式**: 优先纯逻辑单测，避免引入完整 GPUI 窗口级集成测试
- **参考文件**:
  - `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/table_designer_tab.rs:3401`
  - `/Users/hufei/RustroverProjects/onetcli/main/src/onetcli_app.rs:477`
- **覆盖要求**: 调度去重、执行后可再次调度、原有 SQL 预览相关测试不回归

### 5. 依赖和集成点
- **外部依赖**: `gpui`, `gpui_component`
- **内部依赖**: `ColumnsEditorEvent`、`IndexesEditorEvent`、`InputEvent`、`GlobalDbState`、数据库插件 `build_*_sql`
- **集成方式**: 订阅输入/子编辑器事件后刷新预览；加载表结构异步回填后也会触发预览
- **配置来源**: `TableDesignerConfig`

### 6. 技术选型理由
- **为什么用这个方案**: 问题在 UI 事件时序，不在 SQL 生成逻辑；把刷新延后到当前 effect cycle 末能最小成本修正回读时机
- **优势**: 只改表设计器单文件，不碰数据库插件；还能减少连续输入时的重复 SQL 构造
- **劣势和风险**: 需要确保初始加载和异步回填路径仍会触发一次刷新，避免遗漏

### 7. 关键风险点
- **并发问题**: 主要是同一事件周期内重复调度，需避免重复 defer
- **边界条件**: 初始化空表、异步 `load_table_structure` 回填、连续快速输入、多次切换页签
- **性能瓶颈**: `collect_design` 和 SQL 生成是线性操作；延后合并后反而更省
- **安全考虑**: 本次不涉及新增安全逻辑，仅修复本地预览刷新时机

### 8. 外部资料检索记录
- **context7**: 查询 `/websites/rs_gpui_gpui`，确认 `Context::defer_in(&Window, ...)` 用于“当前 effect cycle 结束后执行”，适合等待当前栈上的实体返回应用
- **github.search_code**: 查询 `\"InputEvent::Change\" Rust preview update state lag`，结果较弱，未发现比仓库内模式更直接的参考，因此以仓库内既有实现和 GPUI 文档为主

## 项目上下文摘要（handle-explain-sql）
生成时间：2026-03-09 21:00:01 +0800

### 1. 相似实现分析
- 实现1：`crates/db_view/src/sql_editor_view.rs:681`
  - 模式：点击 handler 内部优先取选中文本，否则回退全文，再委托给下层执行
  - 可复用：`handle_run_query`、`execute_sql_text`
  - 注意点：保持数据库/Schema 选择逻辑不变，避免改动 UI 交互契约

- 实现2：`crates/db_view/src/sql_editor_view.rs:700`
  - 模式：将可纯化的文本处理逻辑独立执行，handler 只负责拿输入和回写
  - 可复用：`handle_format_query` 的“先判空再处理”流程
  - 注意点：适合把 EXPLAIN SQL 构造抽成纯函数并加测试

- 实现3：`crates/db_view/src/sql_result_tab.rs:198`
  - 模式：UI 层统一通过 `SqlResultTabContainer::handle_run_query` 进入异步执行链
  - 可复用：现有结果展示和执行状态管理
  - 注意点：不应改动该接口，只修正传入的 SQL 内容

### 2. 项目约定
- 命名约定：Rust 函数使用 `snake_case`，类型使用 `PascalCase`
- 文件组织：`sql_editor_view.rs` 负责 UI 行为，执行逻辑下沉到 `sql_result_tab.rs`
- 代码风格：早返回、局部纯函数、`match` 按数据库类型分支

### 3. 可复用组件清单
- `crates/db_view/src/sql_editor_view.rs`: `handle_run_query`、`handle_format_query`
- `crates/db_view/src/sql_result_tab.rs`: `SqlResultTabContainer::handle_run_query`
- `crates/db/src/oracle/connection.rs`: `execute_statement_sync`

### 4. 测试策略
- 测试框架：Rust `#[test]` + `cargo test`
- 参考文件：`crates/db_view/src/sql_inline_completion.rs:716`
- 本次策略：为 EXPLAIN SQL 构造函数补纯函数单测，避免依赖 gpui 上下文

### 5. 依赖和集成点
- 外部依赖：`one_core::storage::DatabaseType`
- 内部依赖：`SqlResultTabContainer::handle_run_query`
- 集成方式：`handle_explain_sql` 先分句，再用 `sqlparser` 判断 `SELECT`，最后复用现有执行链

### 6. 技术选型理由
- 抽取 `build_explain_sql` 可直接复用现有 UI 与查询执行链，改动范围最小
- Oracle 当前仅执行 `EXPLAIN PLAN FOR ...`，执行层会把它当成非查询语句处理，无法返回计划详情
- 用 `DBMS_XPLAN.DISPLAY()` 补第二条查询，可以把 Oracle 执行计划变成可展示结果集
- 多语句场景不能直接在整段文本前拼 `EXPLAIN`，应复用 `StreamingSqlParser` 按数据库方言安全分句后逐条包装
- Explain 动作不应误执行 DML/DDL，因此需要用 `sqlparser` AST 判断每条语句是否为 `SELECT`

### 7. 关键风险点
- 边界条件：输入 SQL 为空时仍需保持原有提示文案
- 方言差异：Oracle 和 MSSQL 都是多语句，必须保持拼接顺序正确
- 多语句风险：手工按分号切分会误伤字符串和注释，必须复用现有 parser
- 测试缺口：当前文件没有现成测试，需要新增最小可验证纯函数测试

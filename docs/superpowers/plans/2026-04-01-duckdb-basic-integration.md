# DuckDB Basic Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为现有数据库系统补上 DuckDB 的基础接入，让它能完成注册、连接、本地文件建库、对象树浏览和 SQL 编辑主流程。

**Architecture:** 新增独立 `duckdb` 数据库插件与视图插件，但尽量复用 SQLite 的文件型数据库模型与 UI 行为；通过扩展 `DatabaseType`、`DbManager`、`DatabaseViewPluginRegistry` 和少量类型分支，把 DuckDB 接进现有数据库主链路。

**Tech Stack:** Rust, gpui, gpui-component, duckdb-rs, sqlparser

---

### Task 1: 接入 DuckDB 依赖与类型枚举

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/db/Cargo.toml`
- Modify: `crates/core/src/storage/models.rs`

- [ ] **Step 1: 添加 DuckDB workspace 依赖**

在 workspace 依赖中加入 `duckdb = { version = "1.5.0", features = ["bundled"] }`，并在 `crates/db/Cargo.toml` 中启用 `duckdb.workspace = true`。

- [ ] **Step 2: 扩展 `DatabaseType`**

把 `DuckDB` 加入 `DatabaseType` 枚举、`all()`、`as_str()`、`from_str()`、图标映射和文件型连接显示逻辑。

- [ ] **Step 3: 运行类型层验证**

Run: `cargo test -p one-core storage::models -- --nocapture`
Expected: `DatabaseType` 相关测试通过，至少能序列化/反序列化和枚举出 `DuckDB`

### Task 2: 先写失败测试，锁定最小接入行为

**Files:**
- Modify: `crates/db/src/manager.rs`
- Modify: `crates/db_view/src/sql_editor_view.rs`

- [ ] **Step 1: 为 `DbManager` 注册补测试**

新增测试，断言 `DbManager::default().get_plugin(&DatabaseType::DuckDB)` 返回成功，并且插件名为 `DuckDB`。

- [ ] **Step 2: 为 SQL Explain 补测试**

新增测试，断言 `SqlEditorTab::build_explain_sql(DatabaseType::DuckDB, "select * from users")` 产出 DuckDB 可接受的 explain SQL。

- [ ] **Step 3: 运行测试并确认红灯**

Run: `cargo test -p db manager -- --nocapture`
Expected: 因 `DuckDB` 未注册而失败

Run: `cargo test -p db_view build_explain_sql -- --nocapture`
Expected: 因缺少 `DatabaseType::DuckDB` 分支而失败

### Task 3: 实现 DuckDB 后端插件

**Files:**
- Create: `crates/db/src/duckdb/mod.rs`
- Create: `crates/db/src/duckdb/connection.rs`
- Create: `crates/db/src/duckdb/plugin.rs`
- Modify: `crates/db/src/lib.rs`
- Modify: `crates/db/src/manager.rs`

- [ ] **Step 1: 创建 DuckDB 连接实现**

基于 `duckdb::Connection` 实现 `DuckDbConnection`，最小支持：
`connect()` 打开本地文件数据库；
`disconnect()` 关闭连接；
`query()` / `execute()` 复用 SQLite 风格的“prepare + column_count 判断”执行模型；
值提取先覆盖 `NULL / INTEGER / REAL / TEXT / BLOB`。

- [ ] **Step 2: 创建 DuckDB 插件实现**

基于 SQLite 最小能力提供：
`name()`、`quote_identifier()`、`create_connection()`、`list_databases()`（返回 `main`）、`list_tables()`、`list_columns()`、`list_views()`、`get_table_ddl()`。

- [ ] **Step 3: 把 DuckDB 注册进 `DbManager`**

新增字段、构造逻辑和 `get_plugin()` 分发分支。

- [ ] **Step 4: 运行后端定向测试**

Run: `cargo test -p db manager -- --nocapture`
Expected: `DuckDB` 插件注册测试通过

### Task 4: 实现 DuckDB 视图插件与必要 UI 微调

**Files:**
- Create: `crates/db_view/src/duckdb/mod.rs`
- Create: `crates/db_view/src/duckdb/duckdb_view_plugin.rs`
- Modify: `crates/db_view/src/lib.rs`
- Modify: `crates/db_view/src/database_view_plugin.rs`
- Modify: `crates/db_view/src/sql_editor_view.rs`
- Modify: `main/src/home_tab.rs`
- Modify: `crates/db_view/src/db_tree_view.rs`

- [ ] **Step 1: 创建 DuckDB 视图插件**

优先复用 SQLite 的连接表单与工具栏/右键菜单行为；如果现有 UI 层已经提供 DuckDB 表单，则直接接入该表单，否则先临时复用 SQLite 配置。

- [ ] **Step 2: 注册到视图插件注册表**

把 `DuckDB` 插件加入 `db_view::lib` 模块导出与 `DatabaseViewPluginRegistry::new()`。

- [ ] **Step 3: 补最小类型分支**

在 `sql_editor_view`、`home_tab`、`db_tree_view` 等已有 `match DatabaseType` 逻辑中，为 `DuckDB` 补上与文件型数据库一致的最小行为。

- [ ] **Step 4: 运行前端/视图定向测试**

Run: `cargo test -p db_view build_explain_sql -- --nocapture`
Expected: `DuckDB` explain 测试通过

### Task 5: 编译验收

**Files:**
- Modify: `docs/superpowers/plans/2026-04-01-duckdb-basic-integration.md`

- [ ] **Step 1: 运行定向编译**

Run: `cargo check -p db -p db_view -p main`
Expected: DuckDB 基础接入相关 crate 编译通过

- [ ] **Step 2: 记录偏差**

如果编译或测试暴露出 SQLite 假设与 DuckDB 不兼容，只修正阻塞基础接入的问题；其余能力缺口作为后续完善项记录，不在本次扩 scope。

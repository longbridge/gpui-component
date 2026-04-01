# DuckDB Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补齐 DuckDB 在对象树、DDL 导出和表设计器主路径上的基础能力，避免继续依赖 SQLite 的 metadata/DDL 假设。

**Architecture:** 继续保留现有 UI 结构与 SQLite 的通用导入导出能力，但把 `duckdb` 后端插件里的 metadata、DDL 导出和 ALTER/CREATE TABLE 生成替换成 DuckDB 原生实现；同时收敛 UI 能力面，只暴露当前真正可用的 DuckDB 设计器能力。

**Tech Stack:** Rust, duckdb-rs, gpui, sqlparser

---

### Task 1: 用测试锁定 DuckDB metadata/DDL 缺口

**Files:**
- Modify: `crates/db/src/duckdb/plugin.rs`

- [x] 为 `list_tables/list_columns/list_views/export_table_create_sql` 增加 DuckDB 定向测试
- [x] 为 `build_create_table_sql/build_alter_table_sql` 增加 DuckDB 定向测试

### Task 2: 实现 DuckDB 原生 metadata 与 DDL

**Files:**
- Modify: `crates/db/src/duckdb/plugin.rs`

- [x] 用 `duckdb_tables()/duckdb_columns()/duckdb_views()/duckdb_indexes()` 替换 SQLite metadata 代理
- [x] 用 DuckDB DDL + index SQL 实现 `export_table_create_sql`
- [x] 为 DuckDB 实现原生 `build_create_table_sql/build_alter_table_sql`

### Task 3: 调整 UI 能力面

**Files:**
- Modify: `crates/db_view/src/duckdb/duckdb_view_plugin.rs`
- Modify: `crates/db_view/src/table_designer_tab.rs`

- [x] 关闭 DuckDB 的“SQLite 式自增”能力暴露
- [x] 让表设计器不再把 DuckDB 列误判为 SQLite 自增列

### Task 4: 验证

**Files:**
- Modify: `docs/superpowers/plans/2026-04-01-duckdb-completion.md`

- [ ] 运行 DuckDB 定向测试
- [ ] 运行 `cargo check -p db -p db_view -p main`
- [ ] 根据验证结果修正实现

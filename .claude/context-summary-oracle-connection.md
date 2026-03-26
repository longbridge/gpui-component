## 项目上下文摘要（oracle-connection）
生成时间：2026-03-26 09:12:31 +0800

### 1. 相似实现分析
- **实现1**: `crates/db/src/oracle/connection.rs`
  - 模式：连接、执行和流式执行都通过 `spawn_blocking + Arc<Mutex<Option<Connection>>>` 包装同步 Oracle API。
  - 可复用：`build_query_result`、`build_exec_result`、`execute_statement_sync`、现有日志与错误返回结构。
  - 需注意：原 `extract_value` 只尝试 `String/i64/f64`，运行时覆盖不足，特别是日期时间和二进制类型。
- **实现2**: `crates/db/src/postgresql/connection.rs:244-349`
  - 模式：先看列类型，再做 `try_get` 分支，日期时间按 `%Y-%m-%d %H:%M:%S` / `%Y-%m-%d` / `%H:%M:%S` 格式输出。
  - 可复用：类型驱动的取值策略、时间格式约定、二进制转字符串展示方式。
  - 需注意：Oracle 同样需要先看数据库类型，不能只靠宽泛的字符串兜底。
- **实现3**: `crates/db/src/mssql/connection.rs:36-84`
  - 模式：通过有序 `try_get` 支持 `&str`、整数、浮点、布尔和 chrono 日期时间。
  - 可复用：失败后继续降级尝试的模式，以及与 UI 层约定的 `Option<String>` 输出。
  - 需注意：MSSQL 已显式支持 `NaiveDateTime/NaiveDate/NaiveTime`，Oracle 也应跟进 chrono 类型。
- **实现4**: `crates/db/src/sqlite/connection.rs:30-60`
  - 模式：针对日期时间和二进制做格式化，二进制按十六进制字符串展示。
  - 可复用：`0x...` 的二进制展示策略，与查询表格的纯文本输出兼容。
  - 需注意：Oracle 的 `RAW/BLOB/BFILE` 同样应走稳定的十六进制文本路径。

### 2. 项目约定
- **命名约定**: Rust 函数与 helper 使用 `snake_case`，结构体使用 `PascalCase`，数据库连接实现按 `*DbConnection` 命名。
- **文件组织**: 每种数据库独立目录与 `connection.rs`；连接 trait 在 `crates/db/src/connection.rs`；工作留痕统一放在项目本地 `.claude/`。
- **导入顺序**: 标准库 -> 外部依赖 -> 当前 crate；同一组导入保持紧凑。
- **代码风格**: 连接层返回统一的 `SqlResult` / `DbError`，日志使用 `[Oracle]` 等前缀；注释与文档使用简体中文。

### 3. 可复用组件清单
- `OracleDbConnection::execute_statement_sync`：现有 Oracle 执行主干，无需改动。
- `QueryColumnMeta::new`：查询列元数据统一构造入口。
- `format_message` / `truncate_str`：执行结果提示与 SQL 日志截断。
- `DbConnection` trait：上层对连接层的接口契约，不应破坏。

### 4. 官方文档与外部参考
- `Cargo.toml` 已启用 `oracle = {version = "0.6.3", features = ["chrono"]}`。
- `docs.rs/oracle 0.6.3` 的 `sql_type/chrono.rs` 说明 rust-oracle 在开启 `chrono` 后支持 `NaiveDateTime`、`DateTime<FixedOffset>`、`DateTime<Utc>`、`DateTime<Local>` 等 Oracle 日期时间类型映射。
- `docs.rs/oracle 0.6.3` 的 `sql_type/mod.rs` 与 `OracleType` 文档表明 Oracle 列类型覆盖 `Date`、`Timestamp`、`TimestampTZ`、`TimestampLTZ`、`Raw`、`LongRaw`、`CLOB`、`BLOB`、`BFILE`、`Boolean`、`IntervalDS`、`IntervalYM` 等。
- `github.search_code` 检索到公开 Rust Oracle 代码示例较少，但确认社区实现也围绕 chrono/Oracle 类型映射展开，未发现更适合直接复用的仓库级实现。

### 5. 测试策略
- **测试框架**: `crates/db` 主要使用原生 `#[test]` / `#[tokio::test]`，但当前 Oracle 连接文件没有现成集成测试。
- **验证方式**: 本次最低可重复验证为 `cargo check -p db`，用于确认 `OracleType` 分支、chrono 类型和连接层接口的编译正确性。
- **覆盖要求**: 运行期应覆盖日期时间、时区时间、二进制和大对象读取；当前仓库缺少真实 Oracle 测试环境，只能在报告中记录风险。

### 6. 依赖和集成点
- **外部依赖**: `oracle` crate 提供同步连接、语句和 `Row::get`；`chrono` 提供日期时间格式化；`tokio` 用于 `spawn_blocking` 与超时控制。
- **内部依赖**: `DbConnectionConfig` 提供连接参数，`DatabasePlugin` 提供 SQL 解析器，`SqlResult`/`QueryResult` 是上层 UI 依赖的数据结构。
- **集成方式**: Oracle 结果值最终统一映射为 `Vec<Vec<Option<String>>>`，供表格展示、分页和导出逻辑复用。
- **配置来源**: 连接信息来自 `DbConnectionConfig`，`service_name` / `sid` / `extra_params` 在现有连接层中解析。

### 7. 技术选型理由
- **为什么用按 `OracleType` 分支的方案**: Oracle 类型远比 `String/i64/f64` 三种尝试复杂，特别是 `DATE/TIMESTAMP/TZ/RAW/BLOB/CLOB`；类型驱动方案与仓库其他数据库实现一致，且能直接利用 `chrono` 特性。
- **优势**: 输出稳定、格式统一、便于后续扩展更多 Oracle 类型，不影响现有连接和执行主干。
- **劣势和风险**: Oracle 部分类型存在多种可选 Rust 映射；如果真实库返回的会话/NLS 行为特殊，仍可能需要补充运行时调整。

### 8. 关键风险点
- **并发问题**: 仍沿用现有 `spawn_blocking + blocking_lock` 模式，未引入新的并发路径。
- **边界条件**: 未知 Oracle 类型或特定会话格式下读取失败时，需要保留字符串兜底，避免整行丢失。
- **性能瓶颈**: `BLOB/BFILE` 转十六进制会放大输出体积，大字段结果集仍有内存压力。
- **验证缺口**: 当前没有真实 Oracle 实例的自动化集成验证，只能确认编译通过与代码路径合理。

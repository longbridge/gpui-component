## 项目上下文摘要（db-connection-form-ssl）
生成时间：2026-03-25 13:35:05 +0800

### 1. 相似实现分析
- **实现1**: [db_connection_form.rs](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L230)
  - 模式：表单通过 `TabGroup` + `FormField` 配置式声明不同数据库的连接字段。
  - 可复用：`FormFieldType::Select/Text/Password`、`extra_params` 自动收集。
  - 需注意：空标签页会显示“没有设置项”，因此空白 `ssl` 分组会误导用户。

- **实现2**: [models.rs](/Users/hufei/RustroverProjects/onetcli/crates/core/src/storage/models.rs#L439)
  - 模式：`DbConnectionConfig.extra_params` 承载所有基础字段之外的扩展连接参数。
  - 可复用：`build_connection` 自动写入，`load_connection` 自动回填。
  - 需注意：优先复用该机制，不新增存储结构。

- **实现3**: [mssql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mssql/connection.rs#L236)
  - 模式：驱动通过 `get_param/get_param_as/get_param_bool` 读取扩展参数并应用到建连配置。
  - 可复用：`encrypt`、`trust_cert` 已证明该扩展方式可行。
  - 需注意：UI 字段名应直接对应驱动层参数键。

- **实现4**: [postgresql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L303)
  - 现状：当前硬编码 `NoTls`，只支持明文连接。
  - 结论：要支持 SSL，必须引入外部 TLS connector，而不是只改表单。

- **实现5**: [mysql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs#L281)
  - 现状：当前只构造 `OptsBuilder` 的基础参数，没有 SSL 选项。
  - 结论：需启用 `mysql_async` TLS feature，并构造 `SslOpts`。

### 2. 第三方依赖事实
- `mysql_async 0.36.1` 支持 `SslOpts`、`verify_ca/verify_identity/require_ssl` 语义，本地源码位于 `~/.cargo/registry/.../mysql_async-0.36.1/src/opts/mod.rs`。
- `tokio-postgres 0.7.16` 的 `SslMode` 仅提供 `Disable/Prefer/Require`，TLS 需外部 `MakeTlsConnect`，本地源码位于 `~/.cargo/registry/.../tokio-postgres-0.7.16/src/config.rs`。
- `native-tls 0.2.18` 支持 `add_root_certificate`、`danger_accept_invalid_certs`、`danger_accept_invalid_hostnames`。
- `clickhouse-rs` 文档确认 `https` 需要启用 TLS feature；当前 workspace 依赖原先未开启。

### 3. 项目约定
- **命名约定**: 连接扩展参数使用 `snake_case`。
- **文件组织**: UI 字段定义放 `db_view`，连接行为放 `db`，共享模型放 `core`。
- **代码风格**: 以最小侵入方式扩展现有配置式表单和驱动建连逻辑。

### 4. 可复用组件清单
- `DbConnectionConfig.extra_params`
- `StoredConnection::to_db_connection/from_db_connection`
- `DbConnectionForm::build_connection/load_connection`
- `DbConnectionConfig::get_param/get_param_as/get_param_bool`

### 5. 测试策略
- `cargo check -p db_view`
- `cargo test -p db ssl_ --lib`
- `cargo test -p db_view ssl_tab --lib`

### 6. 技术选型理由
- MySQL：直接复用 `mysql_async::SslOpts`，最小成本接入 CA/主机名校验。
- PostgreSQL：引入 `postgres-native-tls`，保持 `tokio-postgres` 主体不变，仅补 TLS connector。
- MSSQL：不改驱动行为，只把已有 SSL 相关字段归位到 `ssl` 标签页。
- ClickHouse：继续使用 `http/https` 协议选择，但启用 TLS feature 让 `https` 真正可用。
- Oracle：当前驱动未见现成 SSL 扩展点，本次不强行实现，改为去掉误导性的空 SSL 标签页。

### 7. 关键风险点
- 新增 TLS feature 会扩大编译图，需要确认本地依赖可解析。
- PostgreSQL 两种 connector 流类型不同，代码结构必须避免直接在同一 `match` 中返回不同泛型连接对象。
- ClickHouse 之前暴露了 `https` 但依赖未开 TLS feature，本次需要同步修正。

## 项目上下文摘要（db-ssl-rustls-migration）
生成时间：2026-03-25 14:30:01 +0800

### 1. 相似实现分析
- **实现1**: [http_client_tls.rs](/Users/hufei/RustroverProjects/onetcli/crates/reqwest_client/src/http_client_tls.rs#L1)
  - 模式：仓库内已有 `rustls 0.23` 初始化路径，通过安装默认 provider 并构造共享 `ClientConfig`。
  - 可复用：`aws_lc_rs::default_provider().install_default().ok()` 的初始化习惯。
  - 需注意：该实现使用平台验证器，更偏向通用 HTTP 客户端；数据库侧还需要承接自定义 CA 与放宽校验参数。

- **实现2**: [mysql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs#L36)
  - 模式：驱动层通过 `DbConnectionConfig::get_param*` 读取 `extra_params`，在建连前集中构造 TLS 配置。
  - 可复用：`require_ssl`、`verify_ca`、`verify_identity`、`ssl_root_cert_path` 等参数契约不需要改。
  - 需注意：MySQL 迁移到 rustls 主要是 feature 切换，尽量不要改现有 `SslOpts` 逻辑。

- **实现3**: [postgresql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L134)
  - 现状：前一版已用 `native-tls + postgres-native-tls` 接入 PostgreSQL TLS。
  - 可复用：`ssl_mode` 解析、`ssl_root_cert_path` / `ssl_accept_invalid_*` 参数键、`Disable/非 Disable` 分支结构。
  - 需注意：迁移到 rustls 后，必须用新的 connector crate 取代原 `MakeTlsConnector`，不能只切 Cargo feature。

- **实现4**: [mssql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mssql/connection.rs#L231)
  - 模式：驱动通过 `encrypt` / `trust_cert` 这类扩展参数控制底层 TLS 行为。
  - 可复用：继续沿用“参数解析留在连接实现内，表单和存储契约不变”的架构方式。
  - 需注意：MSSQL 主要是依赖 feature 迁移，不需要重新设计连接逻辑。

### 2. 依赖与外部事实
- 根 [Cargo.toml](/Users/hufei/RustroverProjects/onetcli/Cargo.toml#L86) 已存在 `rustls = "0.23.35"`，`reqwest` 也走 `rustls-tls-native-roots`，说明仓库整体接受 rustls 栈。
- GitHub 仓库 `jbg/tokio-postgres-rustls` 的 `Cargo.toml` 明确显示 `tokio-postgres-rustls 0.13.0` 兼容 `rustls 0.23` 与 `tokio-postgres 0.7`，并公开 `MakeRustlsConnect`。
- 本地依赖源码确认：
  - `mysql_async 0.36.1` 支持 `rustls-tls` feature。
  - `clickhouse 0.14.2` 支持 `rustls-tls-native-roots` feature。
  - `tiberius 0.12.3` 默认 feature 含 `native-tls`，切换 rustls 需要 `default-features = false` 并显式启用 `rustls` / `tds73`。
- 本地 `rustls-platform-verifier` / `rustls-native-certs` / `rustls-pemfile` 都已在锁文件中出现，说明项目依赖图已容纳这套证书加载能力。

### 3. 项目约定
- **命名约定**：连接扩展参数保持 `snake_case`，不改已有 key。
- **文件组织**：UI 表单在 `db_view`，驱动建连在 `db`，共享模型仍然通过 `DbConnectionConfig.extra_params`。
- **代码风格**：优先局部 helper + 建连前集中构造配置，不跨模块抽象新 TLS 框架。

### 4. 可复用组件清单
- `DbConnectionConfig::get_param/get_param_as/get_param_bool`
- `DbConnectionConfig.extra_params`
- `MysqlDbConnection::build_ssl_opts`
- `ReqwestClient` 的 rustls provider 初始化模式
- `PostgresDbConnection::ssl_mode`

### 5. 测试策略
- `cargo check -p db_view`
- `cargo test -p db ssl_ --lib`
- `cargo test -p db_view ssl_tab --lib`

### 6. 技术选型理由
- PostgreSQL：使用 `tokio-postgres-rustls::MakeRustlsConnect`，这是与当前 `tokio-postgres 0.7` 和 `rustls 0.23` 直接对齐的现成方案。
- 证书加载：使用 `rustls-native-certs` 加载系统根证书，再叠加 `ssl_root_cert_path` 指向的 PEM/DER 证书，尽量贴近原 `native-tls` 的默认行为。
- 校验放宽：保留现有 `ssl_accept_invalid_certs` 与 `ssl_accept_invalid_hostnames` 语义，通过最小包装 `ServerCertVerifier` 实现，而不是重写整套 TLS 连接逻辑。
- MySQL / ClickHouse / MSSQL：优先切 Cargo feature，减少逻辑层改动面。

### 7. 关键风险点
- 依赖图中存在多个 `rustls` 版本，写代码时必须避免把已经是 `Arc<_>` 的 verifier 再包一层，防止 trait object 类型不匹配。
- `cargo test -p db ssl_ --lib` 会按名称过滤测试；新加的 PostgreSQL TLS 测试必须以 `ssl_` 开头，否则不会进入现有验证流程。
- 自定义证书文件同时支持 PEM/DER，空文件与非法证书要返回明确错误，不能静默吞掉。

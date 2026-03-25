## 项目上下文摘要（mysql-ssh-tls-lab）
生成时间：2026-03-25 14:45:00 +0800

### 1. 相似实现分析
- **实现1**: [ssh_tunnel.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/ssh_tunnel.rs#L1)
  - 模式：数据库连接前统一解析 SSH 参数，并在本地创建端口转发。
  - 可复用：`ssh_tunnel_enabled`、`ssh_host`、`ssh_port`、`ssh_username`、`ssh_auth_type`、`ssh_private_key_path`、`ssh_target_host`、`ssh_target_port`。
  - 需注意：开启 SSH 后，真实数据库主机名由本地转发地址取代，因此 TLS 主机名验证必须考虑这一点。

- **实现2**: [mysql/connection.rs](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs#L36)
  - 模式：MySQL TLS 通过 `build_ssl_opts` 从 `extra_params` 统一构造。
  - 可复用：`require_ssl`、`verify_ca`、`verify_identity`、`ssl_root_cert_path`、`tls_hostname_override` 的参数契约。
  - 需注意：如果走 SSH 隧道且校验证书主机名，证书 SAN 必须匹配本地转发地址，或者显式提供 `tls_hostname_override`。

- **实现3**: [db_connection_form.rs](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L325)
  - 模式：UI 通过 SSL 和 SSH 标签页把字段写入 `DbConnectionConfig.extra_params`。
  - 可复用：测试环境应直接映射到应用表单字段，而不是单独定义另一套配置命名。
  - 需注意：手工测试时要分别覆盖“直连 + TLS”和“SSH + TLS”两种表单填写方式。

- **实现4**: [context-summary-db-ssh-tunnel.md](/Users/hufei/RustroverProjects/onetcli/.claude/context-summary-db-ssh-tunnel.md#L1)
  - 模式：此前 SSH 隧道功能已确认采用本地监听 + `direct-tcpip` 转发，兼容数据库驱动。
  - 可复用：本次测试环境可以用 SSH 跳板容器转发到 Docker 网络中的 MySQL，而无需改动驱动逻辑。

### 2. 项目约定
- **命名约定**：连接参数继续使用现有 `snake_case` 键名。
- **文件组织**：测试辅助文件写入项目本地 `.claude/` 目录，避免污染产品代码目录。
- **代码风格**：优先可重复执行的脚本和 compose 文件，避免只给一次性的手工命令。

### 3. 可复用组件清单
- `DbConnectionConfig.extra_params`
- `resolve_connection_target`
- `MysqlDbConnection::build_ssl_opts`
- `DbConnectionForm` 的 SSH/SSL 标签页字段约定

### 4. 测试策略
- 使用 Docker Compose 启动 `mysql` 与 `bastion` 两个服务。
- 使用本地脚本生成 CA、服务端证书和 SSH 私钥。
- 自动执行三段验证：
  - MySQL 直连 TLS
  - SSH 登录
  - SSH 本地转发 + MySQL TLS

### 5. 依赖和集成点
- 外部依赖：Docker、Docker Compose、OpenSSL、OpenSSH。
- 内部依赖：`db/src/ssh_tunnel.rs`、`db/src/mysql/connection.rs`、`db_view/src/common/db_connection_form.rs`。
- 集成方式：应用通过表单写入参数，测试环境通过 Docker 服务提供真实 SSH 与 TLS 终点。

### 6. 技术选型理由
- MySQL 采用官方 `mysql:8.0` 镜像，降低环境差异。
- SSH 跳板使用自定义最小 `openssh-server` 容器，完全受控且能明确开启 `AllowTcpForwarding`。
- 服务端证书 SAN 同时覆盖 `mysql`、`localhost`、`127.0.0.1`，让“直连”和“SSH 本地转发”两种 TLS 校验都能覆盖。

### 7. 关键风险点
- Docker daemon 访问和镜像拉取需要沙箱外权限。
- MySQL 协议不是纯 TLS，不能用 `openssl s_client` 代替 mysql 客户端验证。
- SSH 隧道场景下，若证书主机名不匹配本地转发地址，会在 `verify_identity=true` 时失败。

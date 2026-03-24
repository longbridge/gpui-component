## 项目上下文摘要（ssh-agent-auth）
生成时间：2026-03-24 09:35:05 +0800

### 1. 相似实现分析
- 实现1：`/Users/hufei/RustroverProjects/onetcli/crates/ssh/src/ssh.rs`
  - 模式：`SshConnectConfig -> SshAuth -> authenticate` 的枚举驱动认证链路
  - 可复用：`SshAuth`、`authenticate`、`RusshClient::connect`
  - 需注意：当前仅支持 `Password` 与 `PrivateKey`
- 实现2：`/Users/hufei/RustroverProjects/onetcli/crates/sftp/src/russh_impl.rs`
  - 模式：SFTP 连接前复制一份 SSH 认证逻辑
  - 可复用：与 `ssh.rs` 相同的 `SshAuth` 输入模型
  - 需注意：存在重复实现，新增能力时容易漏改
- 实现3：`/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/ssh_form_window.rs`
  - 模式：UI 枚举 `AuthMethodSelection` 映射到存储层 `SshAuthMethod`，再映射到运行时 `SshAuth`
  - 可复用：`build_ssh_params`、`build_ssh_connect_config`、编辑态回填逻辑
  - 需注意：当前表单只有“密码 / 私钥”两个单选项
- 实现4：`/Users/hufei/RustroverProjects/onetcli/crates/db/src/ssh_tunnel.rs`
  - 模式：从 `DbConnectionConfig.extra_params` 解析 `ssh_auth_type`
  - 可复用：`build_auth`
  - 需注意：当前仅识别 `private_key`，其余全部退回密码

### 2. 项目约定
- 命名约定：Rust 枚举变体使用 `PascalCase`，配置字符串使用小写下划线风格，如 `private_key`
- 文件组织：`core/storage` 管持久化模型，`terminal_view` 管表单，`ssh` 管协议连接，`sftp`/`db` 复用 SSH 能力
- 导入顺序：标准库 -> 第三方 -> workspace crate
- 代码风格：认证能力通过枚举驱动，而不是散落的布尔开关

### 3. 可复用组件清单
- `/Users/hufei/RustroverProjects/onetcli/crates/core/src/storage/models.rs`：`SshParams`、`SshAuthMethod`
- `/Users/hufei/RustroverProjects/onetcli/crates/ssh/src/ssh.rs`：`SshAuth`、`authenticate`、`RusshClient::connect`
- `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/ssh_form_window.rs`：认证方式表单和参数构造
- `/Users/hufei/RustroverProjects/onetcli/crates/db/src/ssh_tunnel.rs`：数据库 SSH 隧道认证解析

### 4. 测试策略
- 测试框架：Rust 内联单元测试 + `cargo check` / `cargo test`
- 参考文件：`/Users/hufei/RustroverProjects/onetcli/crates/core/src/storage/models.rs` 中现有序列化测试
- 当前缺口：`*ssh*.rs` 内没有 agent 相关测试，也没有现成 SSH 认证测试
- 建议覆盖：
  - `SshAuthMethod` 的序列化/反序列化
  - `db::ssh_tunnel::build_auth` 对 `agent` 的解析
  - SSH agent 不可用时的错误分支

### 5. 依赖和集成点
- 外部依赖：`russh`、`russh::keys::agent::client::AgentClient`
- 内部依赖：`terminal_view -> core/storage + ssh`，`terminal -> ssh`，`sftp -> ssh`，`db -> ssh`
- 集成方式：`StoredConnection.params` 反序列化为 `SshParams`，再映射到运行时连接配置
- 配置来源：SSH 终端连接存储在 `StoredConnection.params`，数据库隧道存储在 `DbConnectionConfig.extra_params`

### 6. 技术选型理由
- 事实：仓库当前没有 `SSH_AUTH_SOCK` 相关实现，issue #3 明确要求补齐 SSH agent 认证
- 事实：开源项目 `lablup/bssh` 与 `AnalyseDeCircuit/oxideterm` 都采用“新增 Agent 枚举分支 + 通过 `AgentClient::connect_env()` 枚举 identities 并调用 `authenticate_publickey_with()`”的方案
- 推论：最稳妥的方案是沿现有枚举链路新增 `Agent`，而不是在 UI 或某条连接路径做特殊分支

### 7. 关键风险点
- 逻辑重复：`ssh.rs` 与 `russh_impl.rs` 都有认证分支，若不抽公共 helper 会继续分叉
- 平台差异：当前证据主要覆盖 Unix/macOS 的 `SSH_AUTH_SOCK`，Windows named pipe 需单独评估
- 验证限制：真实 agent 成功认证依赖本机环境，自动化测试更适合覆盖“缺失 agent”与“枚举映射”场景

## 项目上下文摘要（db_connection_form-ssh_tunnel）
生成时间：2026-03-03

### 1. 相似实现分析
- 实现1: /Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs
  - 模式：表单字段统一映射到 `DbConnectionConfig.extra_params`
  - 可复用：`FormField`、`TabGroup`、`build_connection`
  - 需注意：默认 required 校验会影响 tab 扩展字段
- 实现2: /Users/hufei/RustroverProjects/onetcli/crates/ssh/src/ssh.rs
  - 模式：`RusshClient` 封装认证、代理、跳板能力
  - 可复用：`SshConnectConfig`、`SshAuth`、`channel_open_direct_tcpip`
  - 需注意：会话生命周期必须覆盖转发生命周期
- 实现3: /Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs
  - 模式：每种 DB 驱动从 `DbConnectionConfig` 组装实际连接
  - 可复用：`connect/disconnect` 生命周期管理
  - 需注意：如果做隧道，必须在连接器里持有隧道句柄

### 2. 项目约定
- 命名约定：snake_case 字段名映射到 extra_params key
- 文件组织：`db_view` 管表单，`db` 管连接行为，`ssh` 管传输协议
- 导入顺序：标准库 -> 第三方 -> crate 内部
- 代码风格：错误统一映射到 `DbError`

### 3. 可复用组件清单
- /Users/hufei/RustroverProjects/onetcli/crates/ssh/src/ssh.rs: `RusshClient` / `SshConnectConfig`
- /Users/hufei/RustroverProjects/onetcli/crates/core/src/storage/models.rs: `DbConnectionConfig.extra_params`
- /Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs: 动态表单/校验框架

### 4. 测试策略
- 测试框架：cargo check + 现有编译期约束
- 参考文件：各 DB connection.rs 的 connect/disconnect 实现
- 覆盖要求：SSH 关闭时行为不变；开启时 host/port 走本地隧道；错误信息可回传

### 5. 依赖和集成点
- 外部依赖：`russh`、`tokio`、`tokio::net`
- 内部依赖：`db -> ssh`、`db_view -> core/storage`
- 集成方式：`db_view` 写入 extra_params，`db` 在 connect 阶段解析并建立隧道

### 6. 技术选型理由
- 使用现有 `russh` 封装，避免引入新的 SSH 客户端实现
- 采用本地监听 + direct-tcpip 转发，兼容现有数据库驱动（无需改驱动协议栈）

### 7. 关键风险点
- 隧道生命周期泄漏：通过连接对象持有 tunnel 并在 disconnect/drop 释放
- 并发打开通道冲突：通过 `Arc<Mutex<RusshClient>>` 串行 open_channel
- 配置不完整：表单和 db 层双重校验

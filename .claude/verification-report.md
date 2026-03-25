# 验证报告

- 时间：2026-03-24
- 任务：修复 `crates/core/src/llm/connector.rs` 在升级 `llm-connector` 后的编译失败
- 审查结论：通过
- 综合评分：91/100

## 技术维度评分
- 代码质量：94/100
  - 改动集中在 `connector.rs`，保持了既有 `ProviderType` 分支结构。
  - 旧 API 调用已全部替换为 `llm-connector 1.1.14` 的显式 `base_url` 形式。
- 测试覆盖：82/100
  - 新增了 `provider_base_url` 的两个单元测试。
  - 受 `gpui` Metal shader 构建脚本和沙箱限制影响，`cargo test` 未能完成全流程执行。
- 规范遵循：96/100
  - 仅改动必要文件，命名、导入顺序、错误处理风格与项目现状一致。

## 战略维度评分
- 需求匹配：95/100
  - 直接修复了升级后的编译错误，并保留 `api_base` 可选时的既有体验。
- 架构一致：93/100
  - 未修改 `LlmProvider`、`ProviderManager`、`ProviderConfig` 接口，模块边界稳定。
- 风险评估：86/100
  - 默认 URL 来源已用本地 crate 源码和 README 示例校验。
  - 剩余风险主要来自运行环境对 `cargo test` 的限制，而非实现本身。

## 验证结果
- 已执行：`cargo check -p one-core`
  - 结果：通过
- 已执行：`cargo test -p one-core provider_base_url --lib`
  - 结果：失败
  - 原因：`gpui` 构建脚本写 `~/.cache/clang/ModuleCache` 被沙箱拒绝，报错为 Metal shader compilation failed
- 已执行：`CLANG_MODULE_CACHE_PATH=/tmp/clang-cache cargo test -p one-core provider_base_url --lib`
  - 结果：失败
  - 原因：同上，构建脚本未遵循该环境变量

## 审查清单
- 需求字段完整性：已确认目标、范围、交付物、审查要点
- 原始意图覆盖：无遗漏，聚焦编译失败修复
- 交付物映射：代码、上下文摘要、操作日志、验证报告均已生成
- 依赖与风险评估：已完成
- 审查留痕：已完成

## 建议
- 当前改动可以合并。
- 若需要补全自动化验证，建议在允许写用户缓存目录的环境下重跑 `cargo test -p one-core provider_base_url --lib`，或为 `gpui` 构建脚本单独配置可写模块缓存路径。

---

## 审查报告（ssh-agent-auth 实现）
生成时间：2026-03-24 09:35:05 +0800

### 需求完整性检查
- 目标明确：为 SSH 连接补齐 `ssh-agent` 认证支持
- 范围明确：覆盖存储模型、SSH/SFTP 认证、终端 SSH 表单、数据库 SSH 隧道与本地化文案
- 交付物明确：代码实现、上下文摘要、操作日志、验证报告、本地验证结果
- 风险与依赖明确：终端/UI 相关验证依赖 `gpui` 构建链，已通过提权本地校验完成

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：90/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：96/100
- 风险评估：93/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 认证能力链路已闭环：[`models.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/storage/models.rs#L244) 新增 `SshAuthMethod::Agent`，[`ssh.rs`](/Users/hufei/RustroverProjects/onetcli/crates/ssh/src/ssh.rs#L55) 新增 `SshAuth::Agent` 并实现 agent 认证流程。
- 重复逻辑已收敛：[`russh_impl.rs`](/Users/hufei/RustroverProjects/onetcli/crates/sftp/src/russh_impl.rs#L47) 不再维护单独的密码/私钥认证逻辑，而是复用 `ssh::authenticate_session`。
- UI 与配置映射已补齐：[`ssh_form_window.rs`](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/ssh_form_window.rs#L164) 新增 `Agent` 单选项，相关 `SshAuthMethod -> SshAuth` 映射点也已在终端、SFTP 视图和文件管理器侧补齐。
- 数据库隧道已支持：[`ssh_tunnel.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/ssh_tunnel.rs#L117) 现在能解析 `ssh_auth_type=agent`，[`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L271) 也增加了对应选项。
- 本地验证有效：`cargo check -p ssh -p sftp`、`cargo test -p ssh -p sftp`、提权后的相关 crate `cargo check`、`db` 与 `one-core` 的新增测试都已通过。

---

## 审查报告（ollama-thinking-fallback 实现）
生成时间：2026-03-24 13:33:30 +0800

### 需求完整性检查
- 目标明确：修复 Ollama 下 `qwen3:14b` 等模型正文为空但 `thinking` 有值时，聊天界面显示空回复的问题。
- 范围明确：只修改 `one-core` 项目侧流式消费逻辑，不改第三方 `llm-connector`。
- 交付物明确：共享 helper、双路径修复、最小单测、本地验证、上下文与操作留痕。
- 风险与依赖明确：潜在风险是 reasoning 与正文混合展示；当前修复以可用性优先解决空回复。

### 技术维度评分
- 代码质量：94/100
- 测试覆盖：90/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：95/100
- 风险评估：88/100

### 综合评分
- 93/100
- 建议：通过

### 结论
- 修复点集中且边界清晰：[`mod.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/llm/mod.rs#L19) 新增 `extract_stream_text`，正文优先、正文为空时回退 reasoning/thinking。
- 两条消费链路已统一：[`stream.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/ai_chat/stream.rs#L15) 与 [`general_chat.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/agent/builtin/general_chat.rs#L8) 都改为复用共享 helper，不再各自只读 `get_content()`。
- 问题现象已与本机运行时对齐：提权访问本机 Ollama 证明 `qwen3:14b` 的确会返回空 `content` 和非空 `thinking`，本次修复直接覆盖这一场景。
- 本地验证有效：`cargo check -p one-core` 与 `cargo test -p one-core extract_stream_text --lib` 均已通过。

---

## 审查报告（aliyun-qwen35-url 实现）
生成时间：2026-03-25 10:32:13 +0800

### 需求完整性检查
- 目标明确：修复阿里云官方 `qwen3.5-plus` 在 onecli 中因 URL 路径错误导致的 parse error。
- 范围明确：只调整 `one-core` 的 Aliyun client 路由，不修改第三方 `llm-connector`。
- 交付物明确：`connector.rs` 最小补丁、单元测试、本地验证、上下文与日志留痕。
- 风险与依赖明确：仅对 `qwen3.5-*` 或显式 `compatible-mode` 地址切换为 OpenAI 兼容路径，降低对现有普通模型的影响。

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：89/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：95/100
- 风险评估：90/100

### 综合评分
- 94/100
- 建议：通过

### 结论
- 根因已修正：[`connector.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/llm/connector.rs#L15) 新增阿里云 compatible-mode 默认地址，并在 [`connector.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/llm/connector.rs#L59) 为 `qwen3.5-*` 与显式 `compatible-mode` 地址改走 `openai_compatible`。
- 兼容边界清晰：[`connector.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/llm/connector.rs#L145) 的 `aliyun_prefers_compatible_mode` 只匹配明确场景，其余阿里云模型仍保留原生 `aliyun/aliyun_private` 路径。
- 本地验证有效：`cargo test -p one-core aliyun_prefers_compatible_mode --lib` 与 `cargo check -p one-core` 均已通过。

---

## 审查报告（aliyun-provider-cache 实现）
生成时间：2026-03-25 10:38:33 +0800

### 需求完整性检查
- 目标明确：修复阿里云 qwen3.5-plus 在运行时仍复用旧 provider 导致 URL 错误继续存在的问题。
- 范围明确：仅增强 `ai_chat` provider 创建配置与 `ProviderManager` 缓存命中条件。
- 交付物明确：缓存签名补丁、模型覆盖补丁、本地验证、上下文与操作留痕。
- 风险与依赖明确：补丁不会修改第三方库，只影响配置变化时的 provider 重建。

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：88/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：98/100
- 架构一致：96/100
- 风险评估：91/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 运行时根因已闭环：[`stream.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/ai_chat/stream.rs#L205) 在创建 provider 前把当前 `selected_model` 写回临时 `provider_config.model`。
- 缓存误复用已修正：[`manager.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/llm/manager.rs#L13) 新增 `ProviderCacheEntry`，[`manager.rs`](/Users/hufei/RustroverProjects/onetcli/crates/core/src/llm/manager.rs#L31) 开始按配置签名而非仅按 id 命中缓存。
- 本地验证有效：`cargo test -p one-core provider_cache_signature_changes_with_model --lib` 与 `cargo check -p one-core` 均已通过。

---

## 审查报告（db-tree-csv-import-target 实现）
生成时间：2026-03-24 18:40:00 +0800

### 需求完整性检查
- 目标明确：修复从数据库树表节点导入 CSV/TXT/JSON/SQL 时忽略 database/schema，导致写入默认库同名表的问题。
- 范围明确：限定在 `crates/db/src/import_export/formats/*`，不改 UI 和 manager 接口。
- 交付物明确：共享 helper、导入修复、单元测试、本地验证、上下文与操作留痕。
- 风险与依赖明确：风险主要是行为从“错误写入默认库”修正为“写入选中库”，属于预期缺陷修复。

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：90/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：96/100
- 风险评估：92/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 导入目标表定位已统一：[`mod.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/import_export/formats/mod.rs#L15) 新增 `format_import_table_reference`，直接复用 `DatabasePlugin::format_table_reference`。
- 受影响导入格式已修正：[`csv.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/import_export/formats/csv.rs#L135)、[`json.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/import_export/formats/json.rs#L32)、[`txt.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/import_export/formats/txt.rs#L51)、[`sql.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/import_export/formats/sql.rs#L52) 不再使用裸表名执行 `TRUNCATE/INSERT`。
- 回归验证有效：`cargo test -p db format_import_table_reference --lib` 与 `cargo check -p db` 均已通过。

---

## 审查报告（db-tree-filter-persist 实现）
生成时间：2026-03-24 18:46:00 +0800

### 需求完整性检查
- 目标明确：修复取消勾选数据库后重新进入数据库页仍显示旧筛选结果的问题。
- 范围明确：限定在 `db_tree_view` 的筛选保存和连接事件同步逻辑，不改存储 schema。
- 交付物明确：筛选同步 helper、事件广播修复、纯逻辑测试、本地验证、上下文与操作留痕。
- 风险与依赖明确：主页连接列表依赖连接更新事件，本次修复直接复用该链路。

### 技术维度评分
- 代码质量：94/100
- 测试覆盖：89/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：96/100
- 风险评估：91/100

### 综合评分
- 94/100
- 建议：通过

### 结论
- 根因已修复：[`db_tree_view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L1000) 在筛选写库成功后会发 `ConnectionUpdated`，不再只更新仓库而忽略内存连接列表。
- 打开的树视图也会同步筛选状态：[`db_tree_view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L725) 现在会从传入的 `StoredConnection` 刷新 `selected_databases`。
- 纯逻辑测试已覆盖“保留筛选”和“恢复全选”：[`db_tree_view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L2669)。
- 本地验证有效：`cargo test -p db_view sync_selected_databases_from_connection --lib` 与 `cargo check -p db_view` 均已通过。

---

## 审查报告（db-tree-refresh-tokio-runtime 实现）
生成时间：2026-03-25 10:45:01 +0800

### 需求完整性检查
- 目标明确：修复数据库树刷新时因 `tokio::fs` 在非 Tokio runtime 中执行而触发 panic 的问题。
- 范围明确：限定在 `db_tree_view` 的刷新任务调度，不改缓存模块公开接口。
- 交付物明确：运行时切换修复、本地编译验证、现有测试回归、上下文与操作留痕。
- 风险与依赖明确：依赖项目已有 `Tokio` 包装，风险主要是后台任务调度边界调整。

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：86/100
- 规范遵循：97/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：98/100
- 风险评估：92/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 根因已闭环：[`cache.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/cache.rs#L277) 的缓存失效逻辑使用 `tokio::fs`，而 [`db_tree_view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L1128) 之前在 GPUI task 中直接 await，运行时上下文不匹配。
- 修复符合项目既有模式：[`db_tree_view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L1128) 现在通过 `Tokio::spawn` 把缓存和元数据失效切到共享 Tokio runtime，再回 UI 线程重建树。
- 失败可见性更好：[`db_tree_view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/db_tree_view.rs#L1147) 新增 Tokio 任务失败日志，避免异常被静默吞掉。
- 本地验证有效：`cargo check -p db_view` 与 `cargo test -p db_view sync_selected_databases_from_connection --lib` 均已通过。

---

## 审查报告（db-connection-form-ssl 实现）
生成时间：2026-03-25 13:35:05 +0800

### 需求完整性检查
- 目标明确：为 `db_connection_form` 实现可用的 SSL 配置，并让保存的参数真正影响建连逻辑。
- 范围明确：UI 字段、i18n 文案、MySQL/PostgreSQL 驱动建连、ClickHouse TLS feature、MSSQL 分组调整。
- 交付物明确：SSL 标签页字段、驱动层参数接入、依赖特性启用、纯逻辑测试、本地验证、上下文与操作留痕。
- 风险与依赖明确：PostgreSQL 需要外部 TLS connector，MySQL/ClickHouse 需要启用 TLS feature。

### 技术维度评分
- 代码质量：94/100
- 测试覆盖：88/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：96/100
- 架构一致：97/100
- 风险评估：91/100

### 综合评分
- 94/100
- 建议：通过

### 结论
- 表单层已补齐 SSL 配置：[`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L333) 开始为 MySQL/PostgreSQL/MSSQL/ClickHouse 提供非空 SSL 分组；Oracle 的空白 SSL 标签页已移除。
- MySQL SSL 已接入：[`mysql/connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs#L31) 新增 `build_ssl_opts`，根据 `require_ssl/verify_ca/verify_identity/ssl_root_cert_path/tls_hostname_override` 构造 `mysql_async::SslOpts`。
- PostgreSQL TLS 已接入：[`postgresql/connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L39) 新增 `ssl_mode` 与 TLS connector 构建逻辑，不再固定 `NoTls`。
- 依赖特性已对齐：[`Cargo.toml`](/Users/hufei/RustroverProjects/onetcli/Cargo.toml#L94) 为 `mysql_async`、`clickhouse` 开启 TLS feature，并新增 `postgres-native-tls` / `native-tls`。
- 本地验证有效：`cargo check -p db_view`、`cargo test -p db ssl_ --lib`、`cargo test -p db_view ssl_tab --lib` 全部通过。

---

## 审查报告（db-ssl-rustls-migration 实现）
生成时间：2026-03-25 14:30:01 +0800

### 需求完整性检查
- 目标明确：将数据库 SSL 实现从 `native-tls` 迁移到 `rustls`，同时保持 `db_connection_form` 的字段和 `extra_params` 契约不变。
- 范围明确：工作区依赖、`db` crate 依赖、PostgreSQL connector、MySQL/ClickHouse/MSSQL 的 TLS feature。
- 交付物明确：依赖迁移、PostgreSQL rustls connector、补充测试、本地验证、上下文与操作留痕。
- 风险与依赖明确：PostgreSQL 是唯一需要替换 connector 的驱动，其余驱动主要依赖 feature 切换。

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：91/100
- 规范遵循：97/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：96/100
- 风险评估：93/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 依赖已切换到 rustls：[`Cargo.toml`](/Users/hufei/RustroverProjects/onetcli/Cargo.toml#L97) 现改用 `mysql_async` 的 `rustls-tls`、`clickhouse` 的 `rustls-tls-native-roots`、`tokio-postgres-rustls`，并移除了工作区对 `native-tls/postgres-native-tls` 的直接依赖。
- PostgreSQL connector 已替换：[`connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L209) 通过 `MakeRustlsConnect` 建连，不再依赖 `native_tls::TlsConnector`。
- 现有参数语义被保留：[`connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L31) 的包装 verifier 仅针对 `ssl_accept_invalid_certs` / `ssl_accept_invalid_hostnames` 放宽对应证书错误，不影响其它 TLS 校验路径。
- 自定义 CA 仍可用：[`connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L146) 同时支持从 `ssl_root_cert_path` 读取 PEM/DER 证书并叠加到系统根证书。
- 本地验证有效：`cargo check -p db_view`、`cargo test -p db ssl_ --lib`、`cargo test -p db_view ssl_tab --lib` 全部通过。

---

## 审查补充（mysql-ssh-tls-lab 镜像复用调整）
生成时间：2026-03-25 15:09:17 +0800

### 技术维度评分
- 代码质量：93/100
- 测试覆盖：78/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：96/100
- 架构一致：97/100
- 风险评估：84/100

### 综合评分
- 89/100
- 建议：需讨论

### 结论
- 需求已落实：[`docker-compose.yml`](/Users/hufei/RustroverProjects/onetcli/.claude/mysql-ssh-tls-lab/docker-compose.yml#L3) 已默认复用 `mysql:8.4.5`，[`verify.sh`](/Users/hufei/RustroverProjects/onetcli/.claude/mysql-ssh-tls-lab/verify.sh#L10) 与之保持一致。
- 文档已对齐：[`README.md`](/Users/hufei/RustroverProjects/onetcli/.claude/mysql-ssh-tls-lab/README.md#L13) 说明了默认镜像与 `MYSQL_IMAGE` 覆盖方式。
- 当前唯一未闭环项不是 MySQL 镜像，而是 bastion 首次构建依赖的 `ubuntu:24.04` 拉取/构建仍在进行，因此整套 SSH+TLS 自动验证尚未最终通过。

---

## 审查补充（本地 Docker MySQL TLS + 远程 sshd 联调准备）
生成时间：2026-03-25 16:27:27 +0800

### 技术维度评分
- 代码质量：92/100
- 测试覆盖：90/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：97/100
- 风险评估：90/100

### 综合评分
- 94/100
- 建议：通过

### 结论
- 本地 TLS MySQL 已可用：[`docker-compose.yml`](/Users/hufei/RustroverProjects/onetcli/.claude/mysql-ssh-tls-lab/docker-compose.yml#L2) 的 `mysql` 服务已成功启动，健康检查通过。
- 容器内 TLS 校验通过：使用 `VERIFY_IDENTITY` 和 CA 文件查询 [`01-init.sql`](/Users/hufei/RustroverProjects/onetcli/.claude/mysql-ssh-tls-lab/mysql/init/01-init.sql#L1) 初始化的 `smoke_test` 表，结果为 `2`。
- 宿主机路径也通过：使用本机 `mysql` 客户端连接 `127.0.0.1:33306` 并携带 [`ca.pem`](/Users/hufei/RustroverProjects/onetcli/.claude/mysql-ssh-tls-lab/mysql/certs/ca.pem#L1) 做 `VERIFY_IDENTITY` 校验成功，说明后续经远程 sshd 反向转发到本地 Docker MySQL 的链路具备基础条件。
- `host.docker.internal` 校验失败不影响本次方案：证书 SAN 针对的是 `127.0.0.1`/`localhost`/`mysql`，而 onetcli 通过 SSH 隧道建立本地转发后实际连接主机同样是 `127.0.0.1`。 

---

## 审查补充（MySQL rustls provider panic 修复）
生成时间：2026-03-25 18:40:56 +0800

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：86/100
- 规范遵循：97/100

### 战略维度评分
- 需求匹配：98/100
- 架构一致：97/100
- 风险评估：92/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 根因已闭环：[`mysql/connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs#L37) 之前在启用 TLS 时直接进入 `mysql_async` 的 rustls connector，但进程级默认 `CryptoProvider` 未安装，导致运行时 panic。
- 修复方式与仓库既有模式一致：新增 [`rustls_provider.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/rustls_provider.rs#L1) 统一封装 `aws_lc_rs::default_provider().install_default().ok()`，并用 `Once` 保证只初始化一次。
- 驱动复用已收口：[`mysql/connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/mysql/connection.rs#L37) 和 [`postgresql/connection.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db/src/postgresql/connection.rs#L210) 现在都复用同一个 helper，避免 TLS 初始化逻辑继续分叉。
- 本地编译验证有效：`cargo check -p db` 已通过。

---

## 审查报告（bracketed-paste-fallback 实现）
生成时间：2026-03-25 16:12:44 +0800

### 需求完整性检查
- 目标明确：修复远端未开启 bracketed paste 时，多行粘贴尤其 heredoc 被 shell 错误续行解析的问题。
- 范围明确：限定在 `TerminalView` 的粘贴入口、相关文案和本地单元测试，不改 `Terminal`/`PTY` 透明传输层。
- 交付物明确：高风险粘贴拦截、上下文摘要、操作日志、本地测试与审查报告。
- 风险与依赖明确：shell 结构识别是启发式；依赖 `alacritty_terminal::TermMode` 提供 `ALT_SCREEN` 与 `BRACKETED_PASTE` 状态。

### 技术维度评分
- 代码质量：95/100
- 测试覆盖：90/100
- 规范遵循：97/100

### 战略维度评分
- 需求匹配：96/100
- 架构一致：98/100
- 风险评估：92/100

### 综合评分
- 95/100
- 建议：通过

### 结论
- 高风险结构已在视图层统一拦截：[`view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/view.rs#L113) 新增 `UnbracketedPasteHazard` 和相关纯函数，覆盖 heredoc、未闭合引号与反斜杠续行。
- 粘贴决策已从“只确认”升级为“必要时阻断”：[`view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/view.rs#L1231) 在无 `BRACKETED_PASTE` 时先检查高风险块，再决定是否允许进入原有多行确认流程。
- 协议语义保持正确：[`view.rs`](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/view.rs#L1252) 的 `paste_text_unchecked` 仍只在远端程序已开启 `BRACKETED_PASTE` 时发送 `\x1b[200~...\x1b[201~`，没有伪造远端能力。
- 用户提示已补齐：[`main.yml`](/Users/hufei/RustroverProjects/onetcli/main/locales/main.yml#L1169) 新增 `TerminalView` / `TerminalSidebar` 相关文案，避免新对话框显示原始 key。
- 本地验证有效：`env CLANG_MODULE_CACHE_PATH=/tmp/clang-cache cargo test -p terminal_view --lib` 在沙箱外通过，13 个测试全部通过。

### 剩余风险
- 当前 shell 结构检测是启发式规则，不覆盖所有复杂复合语法；但已覆盖用户报告的 heredoc 主故障路径和两类常见续行结构。
- `main/locales/main.yml` 中补入了当前代码已在使用但仓库缺失的 `TerminalView` / `TerminalSidebar` 文案键，若后续有专门的本地化整理任务，可再统一清理同类缺口。

---

## 审查报告（home-encourage-tab 实现）
生成时间：2026-03-25 19:39:55 +0800

### 需求完整性检查
- 目标明确：将首页“支持作者”从弹框改为在 `tab_container` 中打开页签，并改善赞赏码展示空间。
- 范围明确：限定在首页按钮入口、赞赏视图自身和 `HomePage` 页签打开辅助方法。
- 交付物明确：代码修改、上下文摘要、操作日志、验证报告。
- 风险与依赖明确：整仓编译当前被既有 `db_connection_form.rs` 错误阻塞，因此只能做局部无新增错误验证。

### 技术维度评分
- 代码质量：93/100
- 测试覆盖：78/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：96/100
- 架构一致：97/100
- 风险评估：86/100

### 综合评分
- 90/100
- 建议：通过

### 结论
- 赞赏视图已转为页签内容：[`encourage.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/encourage.rs) 中的 `EncouragePanel` 现在实现了 `EventEmitter<TabContentEvent>` 和 `TabContent`，可以直接挂入 `TabContainer`。
- 首页入口已切换为单实例页签：[`home_tabs.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/home/home_tabs.rs) 新增 `add_encourage_tab`，复用 `activate_or_add_tab_lazy`，重复点击只会激活已有页签。
- 原弹框路径已移除：[`home_tab.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/home_tab.rs) 的“支持作者”按钮已改为调用 `add_encourage_tab`，不再走 `window.open_dialog`。
- 展示尺寸已放大：[`encourage.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/encourage.rs) 将二维码尺寸从 `180` 调整到 `220`，更适合主内容区域。

### 剩余风险
- 当前无法给出“整仓编译通过”结论，因为 `crates/db_view/src/common/db_connection_form.rs` 已存在与本次无关的编译错误。
- 如果后续项目启用统一的 `TabContentRegistry` 恢复注册，建议再补 `EncouragePanel` 的恢复逻辑；本次未做这部分扩展。

---

## 审查报告（db-connection-form-ssh-ssl-fixed 实现）
生成时间：2026-03-25 21:57:00 +0800

### 需求完整性检查
- 目标明确：将数据库连接表单中的 `ssl` 与 `ssh` 页签改成固定代码渲染，并用复选框控制整块启用。
- 范围明确：改动限定在 `crates/db_view/src/common/db_connection_form.rs` 的渲染、辅助逻辑和单元测试，不触碰存储结构。
- 交付物明确：代码修改、上下文摘要、操作日志、本地测试和审查报告均已落地。
- 风险与依赖明确：依赖既有 `extra_params` 键名和 `ssh_form_window.rs` 交互模式；ClickHouse `ssl` 页签本次保持通用渲染，属于刻意收敛范围。

### 技术维度评分
- 代码质量：93/100
- 测试覆盖：89/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：96/100
- 架构一致：95/100
- 风险评估：90/100

### 综合评分
- 94/100
- 建议：通过

### 结论
- `ssh` 页签已改为固定代码渲染：[`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L2115) 新增 `render_ssh_tab_content`，使用复选框控制整块启用，并用单选控制密码、私钥、agent 三种认证输入联动。
- `ssl` 页签已改为固定代码渲染：[`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L2222) 新增 `render_ssl_tab_content`，对 MySQL/PostgreSQL/MSSQL 分别按既有字段语义做启用控制。
- 通用状态链路保持不变：[`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L1819) 继续保留标准页签渲染和字段状态容器，专用页签仍通过原 `set_field_value/get_field_value/build_connection/load_connection` 工作。
- SSH agent 校验已对齐后端语义：[`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L924) 与 [`db_connection_form.rs`](/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs#L1390) 通过纯函数统一必填判断，agent 模式不再错误要求密码。
- 本地验证有效：`CLANG_MODULE_CACHE_PATH=/tmp/clang-cache cargo test -p db_view --lib db_connection_form` 与 `cargo check -p db_view` 均已通过。

### 剩余风险
- 目前覆盖的是纯函数和字段定义层测试，未做 UI 交互快照或人工点击回归，布局细节仍建议你本地实际点一下表单确认观感。
- ClickHouse 的 `ssl` 页签仍沿用原通用渲染，因为本次需求和参考模式主要针对 MySQL/PostgreSQL/MSSQL 的 SSL 语义与 SSH 联动场景。

---

## 审查补充（home-encourage-tab 二次视觉调整）
生成时间：2026-03-25 23:33:00 +0800

### 技术维度评分
- 代码质量：94/100
- 测试覆盖：80/100
- 规范遵循：96/100

### 战略维度评分
- 需求匹配：97/100
- 架构一致：97/100
- 风险评估：88/100

### 综合评分
- 92/100
- 建议：通过

### 结论
- 支持页签布局已重构为居中分区卡片：[`encourage.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/encourage.rs) 现在按“顶部说明卡片 + 中部支付卡片区 + 底部辅助支持卡片”三段展示，不再像截图那样散在左上角。
- 支付卡片层级已增强：[`encourage.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/encourage.rs) 为每个赞赏方式增加外层容器、统一间距和标题行图标，二维码区域更集中。
- 图标风格已统一：页签图标改为星标，[`home_tab.rs`](/Users/hufei/RustroverProjects/onetcli/main/src/home_tab.rs) 中首页入口图标也同步从心形改成了星标，辅助支持区补了 `CircleCheck`、`GitHub`、`ExternalLink` 图标。
- 局部编译筛查有效：`cargo check -p main --keep-going --message-format short 2>&1 | rg 'main/src/(encourage|home_tab)\\.rs|error\\['` 无输出，说明这次视觉调整未给目标文件引入新报错。

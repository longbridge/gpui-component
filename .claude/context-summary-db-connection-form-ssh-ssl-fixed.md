## 项目上下文摘要（db-connection-form-ssh-ssl-fixed）
生成时间：2026-03-25 21:40:00 +0800

### 1. 相似实现分析
- **实现1**: `/Users/hufei/RustroverProjects/onetcli/crates/db_view/src/common/db_connection_form.rs`
  - 模式：通过 `TabGroup + FormField` 初始化所有字段状态，再由 `build_connection/load_connection/set_field_value/get_field_value` 统一映射到 `DbConnectionConfig.extra_params`。
  - 可复用：字段状态容器、Select/Input 初始化、回填与保存链路。
  - 需注意：当前 `render()` 对所有页签都走通用渲染，`ssh`/`ssl` 无法做联动显示。

- **实现2**: `/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/ssh_form_window.rs`
  - 模式：`Checkbox` 控制整块配置是否显示，`Radio` 配合 `.when(...)` 控制子字段联动。
  - 可复用：跳板机、代理、认证方式的固定代码渲染结构。
  - 需注意：交互层直接改本地状态并 `cx.notify()`，不依赖额外状态机。

- **实现3**: `/Users/hufei/RustroverProjects/onetcli/crates/db/src/ssh_tunnel.rs`
  - 模式：通过 `ssh_tunnel_enabled`、`ssh_auth_type`、`ssh_password`、`ssh_private_key_path` 等固定键从 `extra_params` 解析隧道配置。
  - 可复用：字段键名和 `agent/private_key/password` 语义。
  - 需注意：UI 必须继续写回这些原始键名，否则测试连接和保存连接会失效。

- **实现4**: `/Users/hufei/RustroverProjects/onetcli/crates/core/src/storage/models.rs`
  - 模式：`DbConnectionConfig.extra_params` 是数据库连接扩展参数的唯一承载结构。
  - 可复用：`get_param/get_param_as/get_param_bool`。
  - 需注意：本次不能新增新的存储结构或破坏现有序列化。

### 2. 项目约定
- **命名约定**: Rust helper 使用 `snake_case`；连接扩展参数键名保持既有 `snake_case`。
- **文件组织**: `db_view` 负责 UI 与表单状态，`db` 负责连接行为，`core/storage` 负责持久化模型。
- **代码风格**: 优先最小侵入式改动，在现有组件与状态机制上加专用渲染分支。

### 3. 可复用组件清单
- `DbConnectionForm::set_field_value`
- `DbConnectionForm::get_field_value`
- `DbConnectionForm::build_connection`
- `DbConnectionForm::load_connection`
- `DbConnectionConfig::get_param_bool`
- `Checkbox::new(...).checked(...).on_click(...)`
- `Radio::new(...).checked(...).on_click(...)`
- `.when(condition, |this| ...)`

### 4. 测试策略
- **测试框架**: Rust 内联单元测试。
- **参考位置**: `db_connection_form.rs` 文件尾部已有页签字段测试；`db/src/ssh_tunnel.rs` 已有 agent 解析测试。
- **本次覆盖**:
  - SSH agent 模式不再要求密码。
  - SSH/SSL 专用辅助逻辑的启用判定。
  - 现有 SSL 字段定义仍保留，避免破坏 `extra_params` 键映射。

### 5. 依赖和集成点
- **外部依赖**: `gpui`、`gpui_component`。
- **内部依赖**: `db_view -> core/storage`，`db -> ssh`。
- **集成方式**: 页签专用渲染只操作现有字段状态；保存、加载、测试连接继续依赖统一字段映射。

### 6. 技术选型理由
- **为什么用专用渲染分支**: 当前 `FormField` 配置式渲染无法表达“复选框启用整块 + 单选联动子表单”的 UI 需求。
- **为什么保留字段定义**: 字段定义已经承担初始化、回填、保存和测试连接的状态容器职责，保留它可以避免改持久化层。
- **为什么参考 `ssh_form_window`**: 仓库内已经有完全一致的交互模式，直接复用比重新抽象更稳妥。

### 7. 关键风险点
- `validate_ssh_tunnel` 旧逻辑会把 `agent` 误判为密码分支，必须同步修正。
- PostgreSQL 的 `ssl_mode` 有 `disable/prefer/require` 三态，复选框只能做外层启用控制，不能丢掉内部模式选择。
- 专用渲染如果直接绕过现有字段状态，会破坏 `load_connection/build_connection`，必须继续复用既有字段容器。

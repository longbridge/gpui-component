## 项目上下文摘要（windows-owner-id-build）
生成时间：2026-03-20 15:29:09 +0800

### 1. 相似实现分析
- 实现1：`crates/core/src/storage/models.rs`
  - 模式：`StoredConnection` 结构体已包含 `owner_id: Option<String>`
  - 可复用：确认所有字面量初始化都必须显式补该字段
  - 注意点：编译器报错正是因为某处手写初始化漏掉了它
- 实现2：`crates/core/src/storage/models.rs` 中 `new_database/new_ssh/new_redis/new_mongodb/new_serial`
  - 模式：所有构造函数统一写入 `owner_id: None`
  - 可复用：说明默认语义就是“没有所有者时填 None”
  - 注意点：这也是最小修复的正确值
- 实现3：`crates/core/src/storage/repository.rs`
  - 模式：从数据库行反序列化 `StoredConnection` 时显式映射 `owner_id: row.owner_id`
  - 可复用：证明字段已经在持久化层全面接入
  - 注意点：漏字段的位置只剩手写字面量初始化
- 实现4：`crates/core/src/cloud_sync/conflict.rs`
  - 模式：测试里手写 `StoredConnection { ... }`
  - 可复用：当前唯一编译失败点
  - 注意点：这里应补 `owner_id: None`

### 2. 项目约定
- 命名约定：Rust 字段使用 `snake_case`
- 文件组织：模型结构定义在 `storage/models.rs`，冲突测试在 `cloud_sync/conflict.rs`
- 代码风格：对结构体新增字段，手写字面量初始化要全部补齐；默认值明确时优先填显式 `None`

### 3. 可复用组件清单
- `StoredConnection` 结构体定义
- `StoredConnection::new_*` 系列构造函数
- `impl From<ConnectionRow> for StoredConnection`

### 4. 测试策略
- 修改漏字段初始化后，执行 `cargo check -p one-core --tests`
- 若可行，再执行更精确的 `cargo test -p one-core cloud_sync::conflict --no-run`
- 当前目标是修复 Windows CI 编译错误，因此编译级验证优先

### 5. 依赖和集成点
- 失败文件：`crates/core/src/cloud_sync/conflict.rs`
- 字段来源：`crates/core/src/storage/models.rs`
- 持久化映射：`crates/core/src/storage/repository.rs`

### 6. 技术选型理由
- 选择直接补 `owner_id: None`，因为报错点是测试里的手写字面量初始化，且现有构造函数默认值就是 `None`
- 不重构为 `StoredConnection::new_*`，因为这里是最小化测试修复，没必要扩大改动面

### 7. 关键风险点
- 当前修复针对截图中的已知报错；若 Windows job 后续还有其他手写初始化漏字段，CI 会继续暴露
- `cargo check --tests` 能覆盖编译层面，但不替代完整测试运行

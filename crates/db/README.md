# Database Abstraction Layer

这个 crate 提供了一个统一的数据库抽象层，支持多种数据库类型。

## 架构

- `src/` - 顶层接口和公共类型
  - `plugin.rs` - DatabasePlugin trait 定义
  - `manager.rs` - 数据库管理器
  - `connection.rs` - 连接接口和连接池
  - `executor.rs` - SQL 执行器
  - `runtime.rs` - Tokio 运行时
  - `types.rs` - 公共类型定义

- `src/mysql/` - MySQL 实现
- `src/postgresql/` - PostgreSQL 实现  
- `src/sqlite/` - SQLite 实现

## 使用示例

```rust
use db::{DbManager, DatabaseType, DbConnectionConfig};

let manager = DbManager::new();
let plugin = manager.get_plugin(&DatabaseType::MySQL)?;
let connection = plugin.create_connection(config).await?;
```

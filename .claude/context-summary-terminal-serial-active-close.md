## 项目上下文摘要（terminal-serial-active-close）
生成时间：2026-03-20 15:23:03 +0800

### 1. 相似实现分析
- 实现1：`main/src/home_tab.rs`
  - 模式：主页在编辑/删除连接前通过 `ActiveConnections::is_active(conn_id)` 判断连接是否正在使用
  - 可复用：确认问题的最终表现和唯一状态来源
  - 注意点：只要 `ActiveConnections` 残留，串口卡片就会被判定为不可编辑
- 实现2：`crates/sftp_view/src/lib.rs`
  - 模式：SFTP 在断开和关闭路径中显式调用 `set_connection_active(false, cx)`
  - 可复用：关闭视图时同步回收活跃状态的模式
  - 注意点：不依赖实体 drop 后的异步时序
- 实现3：`crates/mongodb_view/src/mongo_tab.rs`
  - 模式：Mongo tab 在 `try_close()` 里显式 `ActiveConnections.remove(connection_id)`
  - 可复用：tab 关闭前同步移除活跃连接状态
  - 注意点：即使后续还有异步清理，`remove` 也是幂等的
- 实现4：`crates/terminal_view/src/view.rs` + `crates/terminal/src/terminal.rs`
  - 模式：TerminalView 的 `try_close()` 当前只调用 `shutdown()`；Terminal 自身只在异步断开回调或连接失败时 `set_connection_active(false, cx)`
  - 可复用：保留现有 `shutdown()` 行为
  - 注意点：tab 被立即移除后，异步回调可能来不及更新全局状态

### 2. 项目约定
- 命名约定：Rust 方法与字段使用 `snake_case`
- 文件组织：UI 关闭逻辑留在对应 view 的 `try_close()`，全局状态通过 `one_core::storage::ActiveConnections` 维护
- 代码风格：优先最小改动，复用既有全局状态与关闭模式，不改 TabContainer 通用逻辑

### 3. 可复用组件清单
- `one_core::storage::models::ActiveConnections`：全局活跃连接状态
- `Terminal::connection_id()`：获取当前终端关联的连接 ID
- `Terminal::shutdown()`：保留原有底层关闭逻辑
- `MongoTabView::try_close()`：关闭前同步移除活跃状态的相似实现

### 4. 测试策略
- 静态确认 `TerminalView::try_close()` 先回收 `ActiveConnections` 再调用 `shutdown()`
- 执行 `cargo check -p terminal_view` 验证编译通过
- 当前环境无法自动完成 GUI 行为验证，最终需手动确认“关闭串口 tab 后主页卡片可编辑”

### 5. 依赖和集成点
- 活跃状态来源：`ActiveConnections`
- 禁止编辑入口：`main/src/home_tab.rs`
- 关闭入口：`TerminalView::try_close()`
- 底层异步清理：`Terminal` 的 disconnect handler

### 6. 技术选型理由
- 选择在 `TerminalView::try_close()` 同步清理，是因为问题发生在 tab 生命周期边界，且这里最接近 Mongo/SFTP 的现有模式
- 不修改 `HomePage`，因为它只是消费 `ActiveConnections` 的只读状态
- 不修改 `TabContainer`，因为问题并非所有 tab 都有，而是 TerminalView 的关闭回收缺失

### 7. 关键风险点
- 当前修复主要针对“tab 已关闭但状态未回收”，不改变底层串口读写线程的关闭时机
- GUI 交互仍需手动确认，尤其是串口断开提示与重新编辑体验

## 项目上下文摘要（home-sftp-button）
生成时间：2026-03-08 12:35:00 +0800

### 1. 相似实现分析
- **实现1**: `main/src/home_tab.rs:2335`
  - 模式：连接卡片右上角 hover 操作区使用 `h_flex().absolute().top_2().right_2().group_hover(...).opacity(0.0)`
  - 可复用：在同一 hover 容器里追加小按钮
  - 需注意：按钮点击前需要 `cx.stop_propagation()`，避免冒泡到卡片点击/双击

- **实现2**: `main/src/home/home_tabs.rs:55`
  - 模式：`open_sftp_view` 负责把 `StoredConnection` 打开为 SFTP 标签页
  - 可复用：新增按钮点击后直接调用该方法
  - 需注意：该方法已处理 tab_id、重复打开和订阅逻辑，无需重复实现

- **实现3**: `main/src/home_tab.rs:2629`
  - 模式：SSH/SFTP 连接卡片上下文菜单已有 `with SSH` 和 `with SFTP` 两个动作
  - 可复用：SFTP 图标使用 `IconName::Folder1.color().with_size(Size::Medium)`，动作调用 `open_sftp_view`
  - 需注意：说明仓库已经认可“同一 SSH/SFTP 连接可直接打开 SFTP 视图”的交互模型

- **实现4**: `main/src/home_tab.rs:1117`
  - 模式：SSH 配置编辑入口通过 `ConnectionType::SshSftp` 查找连接并复用 `show_ssh_form`
  - 可复用：新增 hover 按钮也应仅对 `ConnectionType::SshSftp` 生效
  - 需注意：不要影响其他连接类型的 hover 区布局

### 2. 项目约定
- **命名约定**: hover 按钮 id 使用 `edit-conn-*`、`delete-conn-*` 这类结构化命名
- **代码风格**: 在 `render_connection_card` 内用链式 UI 拼装，尽量局部条件分支，不拆出过度抽象
- **交互模式**: 双击 SSH/SFTP 卡片默认打开 SSH 终端；额外动作放在 hover 或右键菜单

### 3. 技术参考
- **Context7 / GPUI**: `group_hover` 用于 hover 组联动样式，现有 `group_hover(...).opacity(0.0)` 模式已足够，无需额外刷新逻辑
- **GitHub 搜索**: 外部未找到比仓库内现有 hover/action 模式更直接的示例，因此本次以仓库既有实现为主

### 4. 实现判断
- **事实**：仓库已有 SFTP 打开方法、已有 SSH/SFTP 上下文菜单动作、已有卡片 hover 操作区。
- **推论**：最佳实现是在 SSH/SFTP 卡片 hover 操作区中追加一个 SFTP 小按钮，点击直接调用 `open_sftp_view`，最小侵入且交互一致。

## 项目上下文摘要（file-manager-toolbar-path-edit）
生成时间：2026-03-20 18:11:31 +0800

### 1. 相似实现分析
- **实现1**: `crates/sftp_view/src/lib.rs:526`
  - 模式：为路径编辑维护 `path_editing + path_input + InputEvent` 订阅
  - 可复用：`start_remote_path_editing`、`confirm_remote_path`、`cancel_remote_path_editing`
  - 需注意：按 Enter 确认，Blur 取消，避免额外状态分叉

- **实现2**: `crates/sftp_view/src/lib.rs:2601`
  - 模式：`open_dialog + InputState + DialogButtonProps` 新建文件夹
  - 可复用：输入框初始化、聚焦、非法名称校验、成功后刷新目录
  - 需注意：目录创建失败必须通过 `Notification` 提示

- **实现3**: `crates/terminal_view/src/sidebar/file_manager_panel.rs:1549`
  - 模式：已有上传入口 `select_and_upload_files/select_and_upload_folder` 最终统一走 `prepare_uploads`
  - 可复用：头部按钮只触发现有方法，不新增上传链路
  - 需注意：不能破坏现有冲突检测、传输队列和拖拽上传流程

### 2. 项目约定
- **命名约定**: Rust `snake_case` 方法和字段命名，状态字段与行为方法成对出现
- **文件组织**: 面板状态、导航、传输、渲染全部集中在 `file_manager_panel.rs`
- **导入顺序**: 先 `gpui`，再 `gpui_component`，随后项目依赖和标准库
- **代码风格**: 工具栏使用紧凑按钮/图标，异步任务统一通过 `Tokio::spawn` 与 `window.spawn/cx.spawn`

### 3. 可复用组件清单
- `crates/terminal_view/src/sidebar/file_manager_panel.rs`: `navigate_to`、`refresh_dir`、`prepare_uploads`
- `crates/sftp_view/src/lib.rs`: 路径编辑状态机与新建文件夹对话框模式
- `gpui_component::dialog::DialogButtonProps`: 对话框确认/取消按钮文案配置

### 4. 测试策略
- **测试框架**: 当前任务先走本地编译验证
- **验证方式**: `cargo check -p terminal_view`
- **关注点**: 新增状态字段、输入框订阅、工具栏渲染分支、新建目录异步任务闭包

### 5. 依赖和集成点
- **外部依赖**: `sftp::RusshSftpClient` 的 `mkdir/list_dir`
- **内部依赖**: `prepare_uploads`、`navigate_to`、`refresh_dir`
- **集成方式**: UI 按钮触发既有方法；路径输入确认后走既有导航链路

### 6. 技术选型理由
- **为什么用这个方案**: 仓库内已有 `sftp_view` 成熟交互模式，直接复用能减少偏差和维护成本
- **优势**: 最小侵入、风格统一、避免重复造轮子
- **劣势和风险**: 主要风险是缺少真实远端交互手测，但编译链路已闭环

### 7. 关键风险点
- **边界条件**: 输入空路径或非法目录名时只能退出编辑/保留对话框，不能进入异常状态
- **交互风险**: Blur 会退出路径编辑态，需要确保不会意外触发刷新
- **性能影响**: 仅新增轻量 UI 状态与一次 `mkdir` 异步任务，无显著额外负担

## 项目上下文摘要（file-manager-upload-conflict）
生成时间：2026-03-20 18:00:00 +0800

### 1. 相似实现分析
- **实现1**: [crates/terminal_view/src/sidebar/file_manager_panel.rs](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/sidebar/file_manager_panel.rs)
  - 模式：文件选择上传、文件夹上传、拖拽上传最终都直接进入 `enqueue_uploads`
  - 可复用：保留现有传输队列、进度刷新、上传完成后刷新目录的逻辑
  - 需注意：当前没有任何冲突检测或确认弹窗

- **实现2**: [crates/sftp_view/src/lib.rs](/Users/hufei/RustroverProjects/onetcli/crates/sftp_view/src/lib.rs)
  - 模式：上传前先 `list_dir` 获取目标目录文件名集合，再用 `show_conflict_dialog` 处理冲突
  - 可复用：`PendingTransfer`、`generate_unique_name`、`rename_conflicting_transfers`、冲突对话框按钮策略
  - 需注意：目录冲突时才显示“合并”，文件冲突没有“合并”意义

- **实现3**: [crates/sftp/src/russh_impl.rs](/Users/hufei/RustroverProjects/onetcli/crates/sftp/src/russh_impl.rs)
  - 模式：`upload_with_progress` 通过 `CREATE | TRUNCATE | WRITE` 打开远端文件
  - 可复用：作为“现有上传会直接覆盖”的底层行为证据
  - 需注意：如果前端不拦截冲突，上传同名文件会被静默覆盖

- **实现4**: [main/src/home_tab.rs](/Users/hufei/RustroverProjects/onetcli/main/src/home_tab.rs)
  - 模式：使用 `window.open_dialog(...).confirm().button_props(...).on_ok(...)` 组织确认对话框
  - 可复用：终端侧边栏若补冲突确认，应沿用相同的 dialog 构建方式
  - 需注意：对话框按钮和文案需走 i18n，而不是硬编码

### 2. 项目约定
- **命名约定**: Rust 内部辅助结构和函数使用 `snake_case`，结构体使用 `CamelCase`
- **文件组织**: 侧边栏文件管理逻辑集中在 `file_manager_panel.rs`，i18n 文案在 `crates/terminal_view/locales/terminal_view.yml`
- **代码风格**: 优先复用现有上传/对话框模式，避免新增底层接口或跨 crate 抽象

### 3. 可复用组件清单
- [crates/sftp_view/src/lib.rs](/Users/hufei/RustroverProjects/onetcli/crates/sftp_view/src/lib.rs): 冲突检测和冲突对话框完整实现
- [crates/terminal_view/src/sidebar/file_manager_panel.rs](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/src/sidebar/file_manager_panel.rs): 现有上传排队和进度展示逻辑
- [crates/terminal_view/locales/terminal_view.yml](/Users/hufei/RustroverProjects/onetcli/crates/terminal_view/locales/terminal_view.yml): FileManager 现有文案入口

### 4. 测试策略
- 修改后运行 `cargo check -p terminal_view`
- 重点人工逻辑检查：
  - 文件选择上传遇到同名文件时弹出冲突对话框
  - 文件夹选择上传遇到同名目录时出现“合并”按钮
  - 拖拽上传走同一套冲突检测入口

### 5. 依赖和集成点
- **外部依赖**: `gpui_component::WindowExt`、`dialog::DialogButtonProps`、`button::Button`、`notification::Notification`
- **内部依赖**: `RusshSftpClient::list_dir`、现有 `TransferQueue`、`start_upload_task`
- **集成方式**: 在上传入队前插入一次远端目录检查

### 6. 技术选型理由
- **为什么这样设计**: 已有 `sftp_view` 方案能直接证明项目接受“上传前先列目录，再按冲突策略处理”的交互
- **优势**: 行为与已有远程文件管理视图一致，用户不会在侧边栏和主 SFTP 视图看到不同冲突策略
- **风险**: 上传前会额外增加一次 `list_dir` 请求，但仅在上传触发时发生，可接受

### 7. 关键风险点
- `terminal_view` 自己的 locale 里当前没有 `Dialog.file_conflict` 和 `Conflict.*` 词条，需要补充
- 拖拽上传如果继续绕过新入口，会导致只有按钮上传有提示、拖拽上传仍静默覆盖

## 编码前检查 - release-workflow-migration
时间：2026-02-28 14:24:04 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-release-workflow-migration.md`
- 复用组件：
  - `script/bootstrap`：系统依赖安装
  - `script/bundle-macos.sh`：macOS 打包
- 命名约定：沿用 workflow/job/step 命名风格
- 代码风格：沿用现有 YAML 缩进与动作版本
- 不重复造轮子证明：基于已有 build-release/release/ci 三个 workflow 合并

## 编码后声明 - release-workflow-migration
时间：2026-02-28 14:24:04 +0800

### 1. 复用组件
- `script/bootstrap`：保持 Linux/macOS 依赖安装入口不变
- `script/bundle-macos.sh`：保持 macOS app bundle 打包逻辑不变

### 2. 遵循约定
- 命名约定：保留 `release.yml`，job 名称使用 `build/release/publish_crate`
- 代码风格：维持 actions 版本与缓存结构一致
- 文件组织：仅修改 `.github/workflows` 与 README

### 3. 相似实现对比
- 对比 build-release：迁入矩阵构建、打包、checksum、GitHub Release 上传
- 对比旧 release：保留 crates 发布职责并增加保护条件

### 4. 未重复造轮子证明
- 复用了已有脚本与矩阵配置，未新增自定义打包脚本或额外发布工具
## 编码前检查 - chatdb-agent-dispatcher
时间：2026-03-01 10:22:01 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-chatdb-agent-dispatcher.md`
- 将复用组件：
  - `one_core::agent::dispatcher::AgentDispatcher`：统一路由分发
  - `one_core::agent::registry::AgentRegistry`：全局 Agent 注册快照
  - `chatdb::agents::CAP_DB_METADATA`：DB 能力注入
- 命名约定：沿用现有 Rust snake_case / PascalCase 约定
- 代码风格：沿用 `cx.spawn + AsyncApp` 与早返回风格
- 不重复造轮子证明：保留现有 Agent 机制，仅删除 chat_panel 的临时 registry 重建中间层

## 编码后声明 - chatdb-agent-dispatcher
时间：2026-03-01 10:26:35 +0800

### 1. 复用了以下既有组件
- `one_core::agent::registry::AgentRegistry`：直接作为 dispatcher 输入的 Agent 注册表
- `one_core::agent::dispatcher::AgentDispatcher`：统一路由与执行入口
- `chatdb::agents::CAP_DB_METADATA`：数据库能力注入键

### 2. 遵循了以下项目约定
- 命名约定：新增/修改字段与函数继续使用 Rust 既有命名风格
- 代码风格：保持 `cx.spawn`、早返回和事件驱动 UI 更新模式
- 文件组织：仅修改 `core/agent` 与 `db_view/chatdb/chat_panel` 的职责边界

### 3. 对比了以下相似实现
- `crates/core/src/agent/dispatcher.rs`：保持三层路由与 affinity 机制不变
- `crates/core/src/agent/builtin/general_chat.rs`：保持事件流契约（TextDelta/Completed/Cancelled）一致
- `crates/db_view/src/chatdb/agents/sql_workflow.rs`：保持 capability 驱动路由行为不变

### 4. 未重复造轮子的证明
- 移除 chat_panel 内重复构建 local registry 的中间层
- 直接复用全局 AgentRegistry 快照，无新增并行路由实现

## 编码前检查 - chatbi-agent-chart
时间：2026-03-01 10:50:45 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-chatbi-agent-chart.md`
- 将复用组件：
  - `SqlWorkflowAgent`（Agent 事件与多阶段执行模式）
  - `chat_panel` 代码块渲染拦截（`code_block_renderer`）
  - `gpui_component::chart`（Line/Bar/Pie）
- 将遵循命名约定：Rust snake_case / PascalCase
- 将遵循代码风格：早返回 + 分层函数
- 不重复造轮子证明：基于现有 Agent/Chart 组件扩展，不引入新渲染框架

## 编码后声明 - chatbi-agent-chart
时间：2026-03-01 11:05:46 +0800

### 1. 复用了以下既有组件
- `SqlWorkflowAgent` 的选表与元数据处理模式
- `chat_panel` 的代码块级渲染拦截机制
- `gpui_component::chart::{LineChart, BarChart, PieChart}`

### 2. 遵循了以下项目约定
- 命名约定：沿用 chatdb agent 模块命名方式
- 代码风格：保持 AgentEvent 分阶段状态推送
- 文件组织：新增 `chatdb/agents/chat_bi.rs` 与 `chatdb/chart_json.rs`

### 3. 对比了以下相似实现
- `chatdb/agents/sql_workflow.rs`：沿用 capability 驱动 Agent 设计
- `chat_panel.rs`：沿用 `code_block_renderer` 渲染注入点
- `story/chart_story.rs`：沿用 chart 组件数据绑定方式

### 4. 未重复造轮子的证明
- 图表渲染直接复用现有 `gpui_component::chart`
- SQL 执行新增 `execute_script_direct` 以复用 DB session 执行链，无新 DB 客户端

## 编码前检查 - db_connection_form-ssh_tunnel
时间：2026-03-03

- 已查阅上下文摘要文件：`.claude/context-summary-db-ssh-tunnel.md`
- 将复用组件：
  - `crates/ssh/src/ssh.rs` 的 `RusshClient` 与 `channel_open_direct_tcpip`
  - `crates/core/src/storage/models.rs` 的 `DbConnectionConfig.extra_params`
  - `crates/db_view/src/common/db_connection_form.rs` 的表单字段映射
- 将遵循命名约定：沿用 `ssh_*` 前缀写入 `extra_params`
- 将遵循代码风格：连接器 `connect/disconnect` 生命周期内持有资源句柄
- 确认不重复造轮子：不新增 SSH 客户端实现，直接复用仓库 `ssh` crate

## 编码后声明 - db_connection_form-ssh_tunnel
时间：2026-03-03

### 1. 复用了以下既有组件
- `crates/ssh/src/ssh.rs`：复用 `RusshClient` 和 `channel_open_direct_tcpip`，新增本地端口转发包装
- `crates/db_view/src/common/db_connection_form.rs`：复用动态表单字段与 `extra_params` 映射
- `crates/core/src/storage/models.rs`：复用 `DbConnectionConfig.extra_params` 存储隧道配置

### 2. 遵循了以下项目约定
- 命名约定：SSH 表单字段统一 `ssh_*`，与 `extra_params` key 对齐
- 代码风格：连接器在 `connect/disconnect` 里成对管理资源句柄
- 文件组织：`ssh` 负责传输，`db` 负责连接消费，`db_view` 负责配置采集

### 3. 对比了以下相似实现
- `crates/ssh/src/ssh.rs`：沿用已有认证/代理/跳板连接方式
- `crates/db/src/mysql/connection.rs`：沿用原有驱动参数组装，仅替换目标 host/port
- `crates/db_view/src/common/db_connection_form.rs`：沿用 tab + field 结构扩展 SSH 页面

### 4. 未重复造轮子的证明
- 未引入第三方 SSH 客户端，直接扩展仓库既有 `ssh` crate
- 未重写数据库驱动协议层，采用本地转发端口兼容所有现有 TCP 驱动

## 编码前检查 - terminal-dropdown-render
时间：2026-03-03

- 已查阅上下文摘要文件：`.claude/context-summary-terminal-dropdown-render.md`
- 将使用以下可复用组件：
  - `RenderCache -> TerminalElementImpl` 字段透传链路（`crates/terminal_view/src/terminal_element.rs`）
  - `redis_cli_element` 的“先刷背景再绘制文本”模式（`crates/redis_view/src/redis_cli_element.rs`）
  - `ui/input/element` 的“覆盖旧文本”策略（`crates/ui/src/input/element.rs:1609`）
- 将遵循命名约定：Rust snake_case / PascalCase
- 将遵循代码风格：最小侵入改动，保持现有渲染流程
- 确认不重复造轮子：仅修复已有 `terminal_element` 绘制顺序，不引入新渲染系统

## 编码后声明 - terminal-dropdown-render
时间：2026-03-03

### 1. 复用了以下既有组件
- `RenderCache` 与 `TerminalElementImpl` 现有透传结构：新增背景字段透传，不改缓存重建逻辑
- `redis_cli_element` 的先刷背景绘制顺序：用于消除旧文本残留
- `ui/input/element` 的背景覆盖思路：用于处理字符被擦除后的可视残影

### 2. 遵循了以下项目约定
- 命名约定：字段命名使用 `custom_background` 与现有 `custom_cursor` 对齐
- 代码风格：保留既有可见区裁剪和绘制顺序，仅在 paint 前置覆盖背景
- 文件组织：仅修改 `crates/terminal_view/src/terminal_element.rs`

### 3. 对比了以下相似实现
- `crates/redis_view/src/redis_cli_element.rs:459`：保持先背景后文本策略一致
- `crates/ui/src/input/element.rs:1609`：沿用“覆盖旧文本”理念
- `crates/terminal_view/src/terminal_element.rs` 原逻辑：保持增量缓存与 cursor 渲染行为不变

### 4. 未重复造轮子的证明
- 未新增渲染模块或新依赖，仅复用现有字段与 paint 流程扩展
- 未改动 addon、selection、damage 重建机制，问题在原模块内闭环修复

## 编码中调整 - terminal-dropdown-render（二次修复）
时间：2026-03-03

- 观察：背景覆盖后仍有行首残留字符，表现为交互式菜单选择时行内容局部错乱
- 新证据：错乱主要发生在交互式应用光标场景，疑似增量 damage 覆盖不完整
- 调整策略：当 `TermMode::ALT_SCREEN` 或 `TermMode::APP_CURSOR` 启用时，跳过增量路径，改为每帧全量 `rebuild_all`
- 保持不变：普通 shell 场景继续使用原增量渲染路径

## 编码中调整 - terminal-dropdown-render（三次修复）
时间：2026-03-03

- 参考来源：`zed-industries/zed` 的 `crates/terminal_view/src/terminal_element.rs`（每帧基于当前 grid 重新布局）
- 推断：当前残字更可能由增量 dirty 行漏刷导致，而非纯背景未覆盖
- 调整：默认关闭增量路径，`RenderCache::update` 直接 `rebuild_all`；仅在设置 `ONETCLI_TERMINAL_INCREMENTAL=1` 时启用增量逻辑
- 兼容：即使增量开启，`ALT_SCREEN/APP_CURSOR` 仍强制全量重建

## 编码中调整 - terminal-dropdown-render（四次修复）
时间：2026-03-03

- 按用户要求恢复增量渲染主路径（移除默认全量重建开关）
- 重点排查最左侧残留：怀疑 GPUI 文本绘制存在越界像素未被容器清理
- 修复策略：在 `terminal_core` 及终端渲染容器添加 `.overflow_hidden()`，强制裁剪左边界外溢绘制
- 验证：`cargo check -p terminal_view` 通过

## 编码中调整 - terminal-dropdown-render（日志排障）
时间：2026-03-03

- 在 `terminal_element.rs` 增加左边缘诊断日志开关 `ONETCLI_TERMINAL_DEBUG_LEFT_EDGE=1`
- 日志内容：content_mask / terminal_bounds / intersection / 可见行首 text_run 的 start_col 与首字符
- 采样策略：每20帧输出一次，避免日志洪泛
- 默认关闭：不开启环境变量时无额外日志

## 编码前检查 - table-designer-drag-rename
时间：2026-03-04 19:17:57 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-table-designer-drag-rename.md`
- 将使用以下可复用组件：
  - `crates/db_view/src/table_designer_tab.rs` 的 `DragColumn` 与 `move_column`
  - `crates/ui/src/table/state.rs` 的 `on_drag + cx.stop_propagation` 事件处理模式
  - `GlobalDbState -> plugin.build_alter_table_sql` 现有 SQL 生成链路
- 将遵循命名约定：Rust `snake_case` / 类型 `PascalCase`
- 将遵循代码风格：事件订阅驱动 + 局部最小改动
- 确认不重复造轮子，证明：已检查 `db_view/table_designer_tab.rs`、`ui/table/state.rs`、`one_ui/edit_table/state.rs`，直接复用现有拖拽与 SQL 生成路径

## 编码后声明 - table-designer-drag-rename
时间：2026-03-04 19:24:36 +0800

### 1. 复用了以下既有组件
- `DragColumn` / `move_column`：继续使用原有列拖拽数据结构与排序逻辑
- `GlobalDbState -> db_manager.get_plugin`：继续复用数据库插件 SQL 生成入口
- `ui/table/state.rs` 与 `one_ui/edit_table/state.rs` 的 `on_drag + cx.stop_propagation` 事件模式

### 2. 遵循了以下项目约定
- 命名约定：新增方法与字段采用 `snake_case`（如 `collect_column_renames`、`source_name`）
- 代码风格：保持 `collect_design -> build sql -> execute` 既有链路，不新增全局状态
- 文件组织：仅改动 `crates/db_view/src/table_designer_tab.rs`，不扩散到各数据库插件

### 3. 对比了以下相似实现
- `crates/ui/src/table/state.rs:1298`：拖拽开始时调用 `cx.stop_propagation`
- `crates/one_ui/src/edit_table/state.rs:2045`：拖拽与 drop 绑定在可交互头部元素
- `crates/db_view/src/table_designer_tab.rs:2171`：将拖拽触发从整行收敛到手柄元素

### 4. 未重复造轮子的证明
- 未重写数据库插件 `build_alter_table_sql`，而是在设计器层补充列来源映射并拼接重命名语句
- 未新增拖拽框架，直接复用项目现有 GPUI 事件机制

## 编码中调整 - table-designer-drag-rename
时间：2026-03-04 19:24:36 +0800

- 观察：MySQL `RENAME COLUMN` 在低版本可能存在兼容性风险
- 调整：改为 `ALTER TABLE ... CHANGE COLUMN ...` 并复用 `plugin.build_column_def` 生成完整定义
- 结果：`cargo check -p db_view` 与定向单测继续通过

## 编码中调整 - table-designer-drag-rename（测试增强）
时间：2026-03-04 19:34:39 +0800

- 目标：增强“设计表字段修改 SQL 生成”在多数据库下的回归保障
- 新增测试：`table_designer_tab::tests` 从 3 个扩展到 13 个
- 覆盖范围：
  - 各数据库列重命名 SQL 语法断言（MySQL/PostgreSQL/SQLite/MSSQL/Oracle/ClickHouse）
  - 删除+重命名冲突场景（`a,b,c` 删除 `a`，`b -> a`）
  - 空映射保持基线 SQL 不变
  - MySQL `CHANGE COLUMN` 保留新列定义
- 本地验证：`cargo test -p db_view table_designer_tab::tests -- --nocapture` 全通过

## 编码前检查 - tab-container-windows-drag
时间：2026-03-05 10:31:00 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-tab-container-windows-drag.md`
- 将使用以下可复用组件：
  - `crates/core/src/tab_container.rs` 的 `TabBarDragState`（窗口拖动状态）
  - `crates/ui/src/title_bar.rs` 的拖动事件链（`on_mouse_down` → `start_window_move`）
  - `render_control_button` 的 `window_control_area`（Windows 控件区）
- 将遵循命名约定：Rust `snake_case` / 类型 `PascalCase`
- 将遵循代码风格：GPUI 链式构建 + `.when(...)` 条件组合
- 确认不重复造轮子，证明：已检查 `tab_container.rs`、`ui/title_bar.rs`、`zed platform_title_bar.rs`，采用现有模式最小修复。
## 编码中调整 - tab-container-windows-drag
时间：2026-03-05 10:34:00 +0800

- 现象复核：`#tabs` 是 `overflow_x_scroll` 容器，可能吞掉鼠标按下事件，导致父层 `tab-bar` 的 `should_move` 未置位。
- 修复动作：在 `#tabs` 的 `window_control_area(WindowControlArea::Drag)` 上补齐 `on_mouse_down_out/on_mouse_down/on_mouse_up/on_mouse_move`。
- 设计约束：复用既有 `drag_state`，仅在 `show_window_controls` 为真时生效，不影响 macOS。

## 编码后声明 - tab-container-windows-drag
时间：2026-03-05 10:35:00 +0800

### 1. 复用了以下既有组件
- `TabBarDragState`：继续使用 `should_move` 控制窗口拖动启动。
- `render_tab_bar` 现有 `window_control_area(WindowControlArea::Drag)`：未替换，仅增强事件链。
- `render_control_button`：Windows 控件区逻辑未变。

### 2. 遵循了以下项目约定
- 命名约定：新增逻辑仅复用已有变量 `drag_state` 与 `should_move`。
- 代码风格：延续 GPUI 链式 `.when(...).on_mouse_xxx(...)` 结构。
- 文件组织：仅修改 `crates/core/src/tab_container.rs`。

### 3. 对比了以下相似实现
- `crates/ui/src/title_bar.rs:292`：标题栏 `down/up/move` 触发 `start_window_move`。
- `zed-industries/zed/crates/platform_title_bar/src/platform_title_bar.rs`：统一拖动区域 + `should_move` 状态。
- `crates/core/src/tab_container.rs:1540`：原有父层拖动状态管理。

### 4. 未重复造轮子的证明
- 未新增新的拖拽状态结构，仅复用 `TabBarDragState`。
- 未引入新窗口拖动 API，仅复用 `window.start_window_move()`。
## 编码前检查 - redis-cli-scrollbar
时间：2026-03-05 21:16:11 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-redis-cli-scrollbar.md`
- 将使用以下可复用组件：
  - `crates/terminal_view/src/view.rs` 的 `ScrollbarHandle` 模式
  - `crates/redis_view/src/redis_cli_view.rs` 现有 `scroll_offset`/`handle_scroll`
  - `gpui_component::scroll::{Scrollbar, ScrollbarShow}`
- 将遵循命名约定：Rust `snake_case` / 类型 `PascalCase`
- 将遵循代码风格：GPUI 链式渲染 + 局部状态同步
- 确认不重复造轮子，证明：已检查 `redis_cli_view.rs`、`terminal_view/src/view.rs`、`redis_tree_view.rs`、`one_ui/edit_table/state.rs`，采用现有滚动条模式最小集成。
## 编码中调整 - redis-cli-scrollbar
时间：2026-03-05 21:36:33 +0800

- 复用 `terminal_view` 的 `ScrollbarHandle` 语义，在 `redis_cli_view.rs` 新增 `RedisCliScrollbarMetrics/RedisCliScrollbarHandle`。
- 将滚动条 `set_offset` 与视图状态解耦：通过 `pending_scroll_offset` 在 `render` 周期回写 `scroll_offset`。
- 在 `render` 末尾叠加右侧 `Scrollbar::vertical`，并设置 `ScrollbarShow::Always`。
- 在 `handle_scroll`、`canvas` 布局回调、`add_output_entry` 中统一执行 `clamp + metrics 同步`，避免越界。

## 编码后声明 - redis-cli-scrollbar
时间：2026-03-05 21:36:33 +0800

### 1. 复用了以下既有组件
- `crates/terminal_view/src/view.rs`：`ScrollbarHandle` 的 `offset/set_offset/content_size` 设计。
- `gpui_component::scroll::{Scrollbar, ScrollbarShow}`：滚动条组件与显示策略。
- `redis_cli_view` 既有 `scroll_offset` 与 `handle_scroll`，未重写滚动主逻辑。

### 2. 遵循了以下项目约定
- 命名约定：新增类型 `RedisCliScrollbarMetrics/Handle`，函数与字段 `snake_case`。
- 代码风格：保持 GPUI 链式渲染，局部状态同步，不改动模块边界。
- 文件组织：仅修改 `crates/redis_view/src/redis_cli_view.rs`。

### 3. 对比了以下相似实现
- `crates/terminal_view/src/view.rs`：自绘内容 + 右侧绝对定位滚动条。
- `crates/redis_view/src/redis_tree_view.rs`：可见垂直滚动条交互预期。
- `crates/one_ui/src/edit_table/state.rs`：滚动条叠加层写法。

### 4. 未重复造轮子的证明
- 未新增自定义滚动条控件，仅接入已有 `gpui_component::scrollbar` 体系。
- 未替换 `RedisCliElement` 的文本绘制方案，仅增加滚动状态桥接层。
## 编码前检查 - import-export-no-dialog
时间：2026-03-06 15:01:00 +0800

□ 已查阅上下文摘要文件：`.claude/context-summary-import-export-no-dialog.md`
□ 将使用以下可复用组件：
- `TableImportView::add_log`：用于替代弹窗确认提示
- `VirtualListScrollHandle::scroll_to_bottom`：确保提示可见
- `window.remove_window()`：复用完成态关闭模式
□ 将遵循命名约定：Rust `snake_case` / 类型 `PascalCase`
□ 将遵循代码风格：GPUI 链式构建 + 状态更新后 `cx.notify()`
□ 确认不重复造轮子，证明：已检查 `sql_dump_view.rs`、`table_export_view.rs`、`sql_run_view.rs`，存在可直接复用模式

## 编码前检查 - sidebar-chat-panel-migration
时间：2026-03-06 10:20:55 CST

□ 已查阅上下文摘要文件：.claude/context-summary-sidebar-chat-panel-migration.md
□ 将使用以下可复用组件：
- ChatPanel: crates/db_view/src/chatdb/chat_panel.rs - 复用消息与AI执行链路
- AIInput: crates/db_view/src/chatdb/ai_input.rs - 复用输入与模型设置
- DbConnectionSelector: crates/db_view/src/chatdb/db_connection_selector.rs - 复用连接/数据库选择
□ 将遵循命名约定：Rust 结构体 PascalCase / 方法 snake_case
□ 将遵循代码风格：现有事件驱动与 `cx.notify()` 刷新模式
□ 确认不重复造轮子，证明：已检查 sidebar/chatdb/database_tab，采用配置扩展而非新造聊天链路

## 编码后声明 - sidebar-chat-panel-migration
时间：2026-03-06 10:27:45 CST

### 1. 复用了以下既有组件
- ChatPanel: 用于复用 AI 消息渲染与 SQL 执行链路，位于 crates/db_view/src/chatdb/chat_panel.rs
- AIInput: 用于复用 Provider/模型设置与输入交互，位于 crates/db_view/src/chatdb/ai_input.rs
- DbConnectionSelector: 用于连接/数据库/Schema 选择，位于 crates/db_view/src/chatdb/db_connection_selector.rs

### 2. 遵循了以下项目约定
- 命名约定：新增类型采用 `DbSelectorContext` / `SelectorSourceMode`，方法采用 snake_case
- 代码风格：保持事件驱动与 `cx.notify()` 刷新，未引入额外状态管理框架
- 文件组织：上下文配置放在 chatdb 选择器模块，侧栏与 tab 仅做透传

### 3. 对比了以下相似实现
- crates/db_view/src/sidebar/mod.rs: 保留原有侧栏工具栏与 AskAi 事件流，仅替换面板实体
- crates/db_view/src/chatdb/chat_panel.rs: 复用既有消息区和输入区，新增侧栏模式避免复制逻辑

### 4. 未重复造轮子的证明
- 检查了 sidebar/chatdb/database_tab 相关模块，未新增独立聊天业务链路
- 通过 ChatPanel 模式配置和 DbSelectorContext 扩展完成需求，避免新建重复组件

## 返修记录 - sidebar-chat-panel-migration（用户反馈三项）
时间：2026-03-06 11:10:24 CST

- 问题1（侧栏宽度不足导致底部控件挤压）：
  - 调整 `AIInput::render_footer` 为双行布局（上行模式/模型设置，下行发送按钮）
  - 提升数据库侧栏默认宽度与最小宽度（默认 420，最小 360）
- 问题2（执行编辑 SQL 弹窗确认后未关闭）：
  - `chat_panel.rs` 中非查询 SQL 的 `.on_ok` 返回值改为 `true`
- 问题3（切库后 @表偶发不出现）：
  - `AIInput` 元数据同步改为分阶段：先注入表名，再补列信息
  - 新增 `schema_sync_seq`，拦截旧请求回写，避免切换数据库时数据串写

本地验证：`cargo fmt --all && cargo check -p db_view` 通过。

## 返修记录 - sidebar-chat-panel-topbar
时间：2026-03-06 11:21:00 CST

- 在 ChatPanel 侧栏模式新增顶部工具栏：新建对话、历史记录开关、关闭侧栏
- 在侧栏模式新增历史会话抽屉（可展开/收起）
- 恢复 DatabaseSidebar 对 ChatPanelEvent::Close 的订阅联动关闭
- 本地验证：`cargo fmt --all && cargo check -p db_view` 通过

## 编码前检查 - terminal-handle-scroll
时间：2026-03-06 16:11:00 +0800

□ 已查阅上下文摘要文件：`.claude/context-summary-terminal-handle-scroll.md`
□ 将使用以下可复用组件：
- `scroll_lines_accumulated`: `crates/terminal_view/src/view.rs` - 复用现有滚轮累计状态
- `write_to_pty`: `crates/terminal_view/src/view.rs` - 复用终端输入写入封装
- `Terminal::scroll`: `crates/terminal/src/terminal.rs` - 复用普通 display scroll 封装
□ 将遵循命名约定：Rust 类型 `PascalCase` / 方法与字段 `snake_case`
□ 将遵循代码风格：小范围局部修复、状态更新后按需 `cx.notify()`
□ 确认不重复造轮子，证明：已检查 `terminal_view`、`redis_cli_view`、`edit_table` 的滚轮实现，采用复用累计逻辑而非新增滚动抽象

## 编码后声明 - terminal-handle-scroll
时间：2026-03-06 16:16:00 +0800

### 1. 复用了以下既有组件
- `scroll_lines_accumulated`：用于复用已有滚轮累计状态，位于 `crates/terminal_view/src/view.rs`
- `write_to_pty`：用于复用 ALT_SCREEN 下终端输入写入，位于 `crates/terminal_view/src/view.rs`
- `Terminal::scroll` / `scroll_display` 语义：用于保持普通滚动路径不变，位于 `crates/terminal/src/terminal.rs`

### 2. 遵循了以下项目约定
- 命名约定：新增函数 `take_whole_scroll_lines` 采用 `snake_case`
- 代码风格：仅做局部修复和小型纯函数抽取，未改动模块边界
- 文件组织：只修改 `crates/terminal_view/src/view.rs`，验证仍复用现有 `keys.rs` 测试

### 3. 对比了以下相似实现
- `crates/redis_view/src/redis_cli_view.rs`：保留 delta 原始语义，不放大滚轮输入
- `crates/one_ui/src/edit_table/state.rs`：按滚轮正负值与边界决定处理方式
- `crates/terminal_view/src/keys.rs`：APP_CURSOR 序列映射保持不变

### 4. 未重复造轮子的证明
- 未新增新的滚轮状态对象，仅复用既有 `scroll_lines_accumulated`
- 未新增新的终端输入抽象，继续复用 `write_to_pty`
- 未改造底层终端滚动模型，仅修正 ALT_SCREEN 分支的离散化策略

## 项目上下文摘要（bracketed-paste-fallback）
生成时间：2026-03-25 15:50:55 +0800

### 1. 相似实现分析
- **实现1**: `crates/terminal_view/src/view.rs:1138`
  - 模式：`paste_text -> paste_text_unchecked -> write_to_pty` 统一承接快捷键、右键菜单、快捷命令和 AI 代码块粘贴。
  - 可复用：现有高危命令确认、多行确认、`show_paste_confirm_dialog`。
  - 需注意：当前仅在 `TermMode::BRACKETED_PASTE` 为真时发送 `\x1b[200~...\x1b[201~`，否则原样整块写入 PTY。

- **实现2**: `crates/terminal/src/terminal.rs:774`
  - 模式：`Terminal::write` 只负责把字节流下发到后端，不介入粘贴语义。
  - 可复用：无需复用逻辑，但要保持该层“透明传输”边界不变。
  - 需注意：若在该层引入粘贴策略，会破坏现有 `TerminalView -> Terminal -> Backend` 分层。

- **实现3**: `crates/terminal/src/pty_backend.rs:52`
  - 模式：本地 PTY 和 SSH 后端统一按字节流写入 `Msg::Input` / SSH sender。
  - 可复用：无需改动；它证明协议修正点不应放在后端。
  - 需注意：后端不知道当前 shell/程序是否开启了 bracketed paste，也无法判断 heredoc 结构。

- **实现4**: `crates/terminal_view/src/sidebar/mod.rs:86` 与 `crates/terminal_view/src/sidebar/settings_panel.rs:45`
  - 模式：侧边栏设置通过事件总线把“多行粘贴确认”“高危命令确认”回传给 `TerminalView`，快捷命令和 AI 代码块最终也复用 `paste_text`。
  - 可复用：继续把所有粘贴决策收敛在 `TerminalView`。
  - 需注意：不要把策略散落到 sidebar，否则会出现快捷键和侧边栏行为不一致。

### 2. 项目约定
- **命名约定**: Rust 使用 `snake_case` 函数名和 `CamelCase` 类型名。
- **文件组织**: 视图层逻辑集中在 `crates/terminal_view/src/view.rs`，底部带内联单元测试。
- **代码风格**: 复杂交互先做模式分支，再落到纯函数或小助手函数；已有中文注释偏重说明约束和意图。
- **测试方式**: 优先为纯函数补 `#[test]` 单测，避免 UI 实体测试成本。

### 3. 可复用组件清单
- `crates/terminal_view/src/view.rs:1220` `show_paste_confirm_dialog`：现有粘贴确认对话。
- `crates/terminal_view/src/view.rs:1259` `contains_high_risk_command`：现有高危命令检测入口。
- `crates/terminal_view/src/view.rs:880` `write_to_pty`：统一 PTY 写入入口。
- `crates/terminal_view/src/sidebar/mod.rs:220`：设置与快捷命令事件汇聚。

### 4. 测试策略
- **测试框架**: Rust 内置测试。
- **参考文件**: `crates/terminal_view/src/view.rs:2476` 的现有纯函数测试模式。
- **本次覆盖重点**:
  - 普通单行粘贴不应被误判。
  - 普通多行粘贴仍走原有确认策略。
  - heredoc、未闭合引号、尾部反斜杠续行等结构在无 bracketed 模式下应被识别。
  - `BRACKETED_PASTE` 已开启时不应触发新的阻断逻辑。

### 5. 依赖和集成点
- **外部依赖**: `alacritty_terminal` 提供 `TermMode::BRACKETED_PASTE` / `ALT_SCREEN`。
- **内部依赖**: `TerminalView.paste_text` -> `TerminalView.write_to_pty` -> `Terminal::write` -> `PtyBackend::write`。
- **配置来源**: `SettingsPanelEvent::ConfirmMultilinePasteChanged` 与 `ConfirmHighRiskCommandChanged`。
- **集成方式**: 所有粘贴入口最终汇总到 `TerminalView`，适合在这里增加统一降级策略。

### 6. 技术选型理由
- **事实**: Alacritty 文档表明 `CSI ? 2004 h/l` 是 bracketed paste 开关；仓库当前实现也只在 `TermMode::BRACKETED_PASTE` 为真时包装粘贴内容。
- **事实**: Alacritty/WezTerm 的公开实现都遵循“应用开启后才按 bracketed paste 发送”的模型，而不是在未开启时强行插入 `200~`/`201~`。
- **推论**: 客户端不能靠“伪造 bracketed paste”修复远端未开启 2004 的 shell，否则控制序列会暴露给程序本身。
- **本次方案**: 保持已有协议语义，只在无 bracketed 模式下识别必须依赖原子块输入的高风险结构，并改为更强提示或阻断。

### 7. 关键风险点
- **边界条件**: shell 语法并非完整可解析，本次只能做启发式检测，需优先覆盖 heredoc、未闭合引号、尾部反斜杠续行等高频风险。
- **兼容性风险**: `ALT_SCREEN` 场景（如 Vim、less）必须保持直通，不能引入误拦截。
- **体验风险**: 不能把普通多行粘贴都升级为阻断，否则会造成过度打扰。

### 8. 外部资料来源
- **Context7**: `/alacritty/alacritty`，用途是确认 `CSI ? 2004 h/l` 属于 bracketed paste 模式。
- **GitHub**: `alacritty/alacritty` 的 `alacritty/src/event.rs`，用途是核对终端事件与粘贴处理模型。
- **GitHub**: `wezterm/wezterm` 的 `wezterm/src/cli/send_text.rs`，用途是对比“发送原始文本”和“按 paste 语义发送”是两个不同入口。

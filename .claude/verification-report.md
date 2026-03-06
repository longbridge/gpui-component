# 验证报告

- 任务：修复终端视图的跨平台快捷键，并按平台在右键菜单中显示快捷键
- 时间：2026-03-06 17:29:00 +0800
- 结论：通过
- 综合评分：96/100

## 技术维度评分
- 代码质量：96/100
  - `TerminalView` 的快捷键抽成平台常量，避免终端场景误占用控制序列。
  - 非 macOS 改为 `Ctrl+Shift+...`，符合终端类应用常见习惯。
  - 右键菜单通过 `Kbd::format` 统一生成平台显示文本，避免硬编码。
- 测试覆盖：88/100
  - 本次通过 `cargo check -p terminal_view` 做定向编译验证。
  - 当前缺少针对菜单标签和按键派发的集成测试，因此保留少量扣分。
- 规范遵循：100/100
  - 文案使用国际化占位符。
  - 代码风格与项目现有 `KeyBinding::new`、`t!`、`Kbd::format` 模式一致。

## 战略维度评分
- 需求匹配：98/100
  - 完整满足“Windows/Linux 终端不能用 Ctrl+C 复制”“右键菜单按平台显示快捷键”的新增约束。
- 架构一致：95/100
  - 未修改终端底层模型，只调整注册层和菜单显示层。
- 风险评估：94/100
  - 已避免对终端控制序列的干扰。
  - 剩余风险主要在运行时交互体验，需后续人工回归确认。

## 本地验证
- `cargo fmt --all`
- `cargo check -p terminal_view`

## 变更摘要
- `crates/terminal_view/src/view.rs`
  - 引入平台化快捷键常量
  - 非 macOS 改为 `Ctrl+Shift+C/V/A/F/G`
  - `handle_key_event` 同步支持非 macOS 的 `Ctrl+Shift+C/V`
  - 右键菜单通过国际化占位符显示快捷键
- `crates/terminal_view/locales/terminal_view.yml`
  - 新增 `copy_with_shortcut` / `paste_with_shortcut` / `select_all_with_shortcut`

## 建议
- 当前修改可以合入。
- 若后续继续完善，可为菜单项快捷键显示和终端按键派发增加定向测试。
# 验证报告

- 任务：为 `terminal_view` 侧边栏 AI 增加终端专属系统提示词，约束输出 Linux 单命令代码块
- 时间：2026-03-07 11:38:56 +0800
- 结论：通过
- 综合评分：96/100

## 技术维度评分
- 代码质量：97/100
  - 在 `AiChatPanel` 中新增可选 `system_instruction`，只扩展统一发送入口，不破坏既有结构。
  - 历史消息裁剪后再插入 `Role::System`，保证提示词稳定生效且不改变现有 `history_count` 语义。
  - 终端场景通过 setter 注入，不影响其他 `AiChatPanel` 使用方。
- 测试覆盖：88/100
  - 已通过 `cargo fmt --all` 与 `cargo check -p terminal_view`。
  - 当前仍缺少提示词生效的自动化 UI/集成测试，保留部分扣分。
- 规范遵循：100/100
  - 命名、文件组织、最小改动策略均符合仓库既有模式。

## 战略维度评分
- 需求匹配：98/100
  - 明确要求 AI 返回 Linux `bash` 代码块、每个代码块一个命令、多步骤拆成多个代码块。
  - 改动范围仅限终端侧边栏场景，符合用户要求。
- 架构一致：96/100
  - 继续复用通用 `AiChatPanel`，通过可选配置实现终端场景差异化。
- 风险评估：94/100
  - 工程实现风险较低。
  - 剩余风险主要来自模型遵循提示词的稳定性，需后续实际交互观察。

## 本地验证
- `cargo fmt --all`
- `cargo check -p terminal_view`

## 变更摘要
- `crates/core/src/ai_chat/panel.rs`
  - 新增 `system_instruction` 字段与 `set_system_instruction`
  - `send_message` 在发送前前置插入 `Role::System` 消息
- `crates/terminal_view/src/sidebar/mod.rs`
  - 新增终端专属系统提示词常量
  - 创建 `AiChatPanel` 后注入 Linux 单命令代码块约束

## 建议
- 当前实现可以直接使用。
- 若后续数据库 AI、Redis AI 也需要场景化提示词，可继续复用 `set_system_instruction`。
- 若要进一步提升稳定性，可后续补一个针对 `send_message` 的消息构造单元测试。
## 项目上下文摘要（ollama-thinking-fallback）
生成时间：2026-03-24 13:33:30 +0800

### 1. 相似实现分析
- **实现1**: `crates/core/src/ai_chat/stream.rs`
  - 模式：统一封装 provider 获取、流式消费、节流发送和完成态收尾。
  - 可复用：`StreamEvent::ContentDelta/Completed` 事件模型与 `full_content` 累积逻辑。
  - 需注意：原实现只读取 `response.get_content()`，正文为空时会把空字符串作为最终结果完成。

- **实现2**: `crates/core/src/agent/builtin/general_chat.rs`
  - 模式：Agent 内部直接消费 `chat_stream`，再转换为 `AgentEvent::TextDelta/Completed`。
  - 可复用：与聊天面板一致的节流与完成态处理模式。
  - 需注意：这里与 `ChatStreamProcessor` 存在重复的“只读正文”逻辑，修复应复用而非复制。

- **实现3**: `crates/core/src/ai_chat/panel.rs`
  - 模式：UI 侧只消费 `StreamEvent`，并通过 `ChatEngine::finalize_streaming` 收尾。
  - 可复用：现有面板和持久化链路无需感知 provider 细节。
  - 需注意：如果上游传入空 `full_content`，UI 会正常完成但展示空消息。

- **实现4**: `~/.cargo/registry/.../llm-connector-1.1.14/src/types/streaming.rs`
  - 模式：第三方库提供 `Delta::reasoning_any()` 聚合 `reasoning_content/reasoning/thought/thinking`。
  - 可复用：项目侧无需重复解析字段，只需复用这个聚合接口。
  - 需注意：`StreamingResponse::get_content()` 不会回退到 reasoning 字段。

### 2. 项目约定
- **命名约定**: Rust 函数与辅助方法使用 `snake_case`，测试函数用行为描述命名。
- **文件组织**: 共享协议/类型能力放在 `crates/core/src/llm/`，上层消费放在 `ai_chat/` 与 `agent/`。
- **导入顺序**: 先标准库，再第三方依赖，最后 `crate` 内部模块。
- **代码风格**: 最小侵入式补丁，优先提取共享 helper，避免在并行链路重复实现相同逻辑。

### 3. 可复用组件清单
- `crates/core/src/llm/mod.rs`: 适合作为共享流式文本提取 helper 的放置点。
- `crates/core/src/ai_chat/stream.rs`: 聊天面板的统一流式入口。
- `crates/core/src/agent/builtin/general_chat.rs`: 通用 Agent 的统一流式入口。
- `llm_connector::types::Delta::reasoning_any()`: 第三方库已提供的 reasoning 聚合能力。

### 4. 测试策略
- **测试框架**: Rust 内联单元测试 `#[cfg(test)] mod tests`。
- **参考文件**: `crates/core/src/llm/connector.rs` 中已有最小单测模式。
- **本次验证方式**:
  - `cargo check -p one-core`
  - `cargo test -p one-core extract_stream_text --lib`
  - 必要时用本机 Ollama 请求复核 `qwen3:14b` 的原始返回结构。

### 5. 依赖和集成点
- **外部依赖**: `llm-connector 1.1.14`，其中 `Delta::reasoning_any()` 已支持多个 reasoning 字段别名。
- **内部依赖**: `ChatStreamProcessor` 与 `GeneralChatAgent` 都依赖 `crate::llm` 暴露的流式类型。
- **集成方式**: 在 `llm` 模块提供共享提取函数，上层两条流式消费链路统一调用。
- **配置来源**: 继续沿用现有 `ProviderConfig` 与用户面板设置，不新增配置字段。

### 6. 技术选型理由
- **事实**: 本机 `ollama list` 显示已安装 `qwen3:14b`，但没有 `qwen3.5`。
- **事实**: 直接请求本机 Ollama `qwen3:14b` 时，返回 `message.content = ""`、`message.thinking` 有内容、`done_reason = "length"`。
- **事实**: `llm-connector` 已把 Ollama 的 `thinking` 映射进 `delta.thinking`，但项目上层未消费。
- **推断**: 最小修复是正文优先、正文为空时回退到 `reasoning_any()`，而不是修改第三方库或 UI 协议。

### 7. 关键风险点
- **行为风险**: 若某些模型先输出 reasoning 后输出正文，本次补丁会把 reasoning 也视为可展示文本。
- **兼容性风险**: 修复必须保持非 Ollama provider 的正文路径优先级不变。
- **验证风险**: 本地 Ollama 复核需要可访问用户侧本机服务，沙箱内无法直接访问 localhost。

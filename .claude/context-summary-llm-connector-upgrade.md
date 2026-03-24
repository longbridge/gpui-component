## 项目上下文摘要（llm-connector 升级适配）
生成时间：2026-03-24

### 1. 相似实现分析
- **实现1**: `crates/core/src/llm/connector.rs`
  - 模式：按 `ProviderType` 分支创建第三方 `LlmClient`，再通过 `LlmProvider` trait 暴露统一接口。
  - 可复用：`build_request`、`create_message`、`user_message`、`assistant_message`、`system_message`。
  - 需注意：旧实现假定第三方库会提供默认 `base_url`，升级后该假设已失效。

- **实现2**: `crates/core/src/llm/onet_cli_provider.rs`
  - 模式：自定义 provider 不直接依赖 `LlmConnector`，而是通过 `CloudApiClient` 适配到相同 trait。
  - 可复用：错误映射模式 `map_err(|e| anyhow::anyhow!("{}", e))`。
  - 需注意：`LlmProvider` trait 的签名未变，修复应保持接口兼容。

- **实现3**: `crates/core/src/llm/manager.rs`
  - 模式：`ProviderManager` 负责缓存和按配置实例化 provider。
  - 可复用：`ProviderType::OnetCli` 与其他第三方 provider 的分流逻辑。
  - 需注意：`LlmConnector::from_config` 是集成入口，改动应局限在这里。

### 2. 项目约定
- **命名约定**: Rust 类型使用 `PascalCase`，函数/字段使用 `snake_case`，provider 名称通过 `ProviderType::as_str()` 统一输出。
- **文件组织**: `crates/core/src/llm/` 下按职责拆分为 connector、manager、provider、storage、types。
- **导入顺序**: 先标准库，再第三方依赖，最后 `super`/`crate` 内部模块。
- **代码风格**: `anyhow::Result` + `anyhow::bail!`/`anyhow!`，trait 用 `async_trait`，注释以简体中文为主。

### 3. 可复用组件清单
- `crates/core/src/llm/types.rs`: `ProviderType` 与 `ProviderConfig`，定义支持的 provider 和配置字段。
- `crates/core/src/llm/manager.rs`: `ProviderManager::get_provider`，说明 `LlmConnector::from_config` 是统一构造入口。
- `crates/core/src/cloud_sync/client.rs`: `CloudApiClient`，说明 `ChatStream` 和 `ChatRequest` 是跨模块复用接口。

### 4. 测试策略
- **测试框架**: Rust 内联单元测试，模式为 `#[cfg(test)] mod tests` + `#[test]`。
- **参考文件**: `crates/core/src/crypto.rs`、`crates/core/src/ai_chat/services.rs`。
- **本次验证方式**: 先执行 `cargo check -p one-core` 进行编译验证；若需要，再补充针对默认 URL 解析的单元测试。

### 5. 依赖和集成点
- **外部依赖**: `llm-connector = 1.1.14`（workspace 依赖，启用 `streaming` feature）。
- **内部依赖**: `connector.rs` 依赖 `types.rs` 的 `ProviderConfig/ProviderType`，并被 `manager.rs` 调用。
- **集成方式**: 通过 `LlmProvider` trait 向上游暴露 `chat`、`chat_stream`、`models`。
- **配置来源**: `ProviderConfig.api_base`、`api_version`、`api_key`。

### 6. 技术选型理由
- **事实**: `cargo check -p one-core` 报错显示 `LlmClient::openai_with_base_url` 与 `ollama_with_base_url` 已不存在，且 `openai/anthropic/aliyun/zhipu/ollama/volcengine/moonshot/deepseek/google` 均要求显式 `base_url`。
- **事实来源**: 本地 crate 源码 `~/.cargo/registry/.../llm-connector-1.1.14/src/client.rs` 与 `README.md`。
- **事实**: `README.md` 明确声明 `Zero Hardcoded URLs`，且示例要求调用方提供 `base_url`。
- **推断**: 为保持项目原有“`api_base` 可省略”的配置体验，需要在本项目侧维护 provider 默认 URL 映射，而不是依赖第三方库默认值。

### 7. 关键风险点
- **边界条件**: `ProviderType::OpenAICompatible` 和 `AzureOpenAI` 仍然必须依赖用户提供地址，不能错误套用默认值。
- **兼容性风险**: Moonshot、DeepSeek、Volcengine、Google 等默认地址若取错，会造成运行时请求失败，即使编译通过。
- **验证风险**: 当前 `llm` 模块缺少现成单测，至少需要本地编译验证；如后续需要增强，可为默认地址解析补测试。
- **外部资料情况**: Context7 未检索到 `llm-connector` 的精确库 ID，因此本次接口依据退回到本地 crate 源码与 GitHub 开源仓库 `lipish/llm-connector` 的示例。
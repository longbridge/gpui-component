## 项目上下文摘要（aliyun-qwen35-url）
生成时间：2026-03-25 10:32:13 +0800

### 1. 相似实现分析
- **实现1**: `crates/core/src/llm/connector.rs`
  - 模式：按 `ProviderType` 分支创建第三方 `LlmClient`，是所有 provider URL 选择的唯一入口。
  - 可复用：`provider_base_url`、现有 provider 常量和 `match ProviderType` 分发结构。
  - 需注意：Aliyun 分支当前默认走 `LlmClient::aliyun(...)`，会落到第三方库内置 DashScope 原生路径。

- **实现2**: `~/.cargo/registry/.../llm-connector-1.1.14/src/protocols/adapters/aliyun/mod.rs`
  - 模式：`AliyunProtocol::chat_endpoint` 固定把聊天请求发送到 `/api/v1/services/aigc/text-generation/generation`。
  - 可复用：无需项目侧重复实现协议，只需决定何时不走这个协议。
  - 需注意：流式解析器要求服务端返回 `output` 字段；若 URL 本身错误，服务端会返回错误 JSON，进而触发当前 parse error。

- **实现3**: `~/.cargo/registry/.../llm-connector-1.1.14/src/lib.rs`
  - 模式：同时支持 `aliyun(...)` 与 `openai_compatible(...)` 两种客户端构造。
  - 可复用：项目侧可以直接切换到 `openai_compatible`，不必修改第三方依赖。
  - 需注意：`openai_compatible` 要求基地址本身就是 OpenAI 兼容前缀，例如 `/compatible-mode/v1`。

### 2. 项目约定
- **命名约定**: 新增 helper 使用 `snake_case`，常量使用全大写下划线命名。
- **文件组织**: provider 路由逻辑继续留在 `connector.rs`，不扩散到 manager 或 UI。
- **代码风格**: 维持最小补丁，优先加 helper 进行条件分流，而不是重构整个 provider 层。

### 3. 可复用组件清单
- `crates/core/src/llm/connector.rs`: 本次唯一改动文件。
- `LlmClient::openai_compatible`: 现有通用 OpenAI 兼容客户端。
- `LlmClient::aliyun` / `LlmClient::aliyun_private`: 继续保留普通 DashScope 原生模型与自定义私有地址路径。

### 4. 测试策略
- **测试框架**: Rust 内联单元测试。
- **本次验证方式**:
  - `cargo test -p one-core aliyun_prefers_compatible_mode --lib`
  - `cargo check -p one-core`

### 5. 依赖和集成点
- **外部依赖**: `llm-connector 1.1.14`
- **内部依赖**: `ProviderManager -> LlmConnector::from_config` 仍是唯一集成入口。
- **配置来源**: `ProviderConfig.model` 与 `ProviderConfig.api_base`

### 6. 技术选型理由
- **事实**: 用户错误响应是 `InvalidParameter` + `url error`，说明服务端拒绝了当前 URL。
- **事实**: 阿里云官方文档中，`qwen3.5-plus` 的 OpenAI 兼容调用地址是 `https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions`。
- **事实**: 当前第三方 `AliyunProtocol` 固定命中 DashScope 原生文本生成路径，而不是 compatible-mode。
- **推断**: 对 `qwen3.5-*` 默认切换到 `openai_compatible` 是最小且足够精确的修复。

### 7. 关键风险点
- **兼容性风险**: 普通 Aliyun 文本模型不应被误切到 compatible-mode，因此只匹配 `qwen3.5-*` 或显式 `compatible-mode` 地址。
- **行为风险**: 用户若手工把 `api_base` 配成 `compatible-mode`，必须同步切 openai-compatible，否则仍会拼错路径。

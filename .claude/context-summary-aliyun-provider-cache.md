## 项目上下文摘要（aliyun-provider-cache）
生成时间：2026-03-25 10:38:33 +0800

### 1. 相似实现分析
- **实现1**: `crates/core/src/llm/manager.rs`
  - 模式：`ProviderManager` 统一负责 provider 构造和缓存。
  - 可复用：现有 `DashMap` 缓存结构和 `get_provider/remove_provider/clear_cache` 生命周期管理。
  - 需注意：原逻辑只按 `provider_id` 命中缓存，不比较模型和地址。

- **实现2**: `crates/core/src/ai_chat/stream.rs`
  - 模式：从仓库读取 provider 配置，构建请求，再通过 `ProviderManager` 获取 provider 发起流式请求。
  - 可复用：`selected_model` 已在请求层生效。
  - 需注意：原先创建 provider 时仍使用数据库默认 `config.model`，与请求层模型可能不一致。

- **实现3**: `crates/db_view/src/chatdb/chat_panel.rs`
  - 模式：构建 `ProviderConfig` 时会把 `selected_model` 覆盖进 `model` 字段。
  - 可复用：说明项目里已经存在“运行时 provider 构造应使用当前选中模型”的模式。
  - 需注意：`ai_chat` 这条链路之前没有同步这个模式。

### 2. 项目约定
- **命名约定**: helper 使用 `snake_case`，内部缓存结构使用简洁名词。
- **文件组织**: provider 构造与缓存留在 `llm/manager.rs`，请求层覆盖留在 `ai_chat/stream.rs`。
- **代码风格**: 以最小增强方式扩展现有结构，不重构模块边界。

### 3. 可复用组件清单
- `ProviderManager::get_provider`
- `ChatStreamProcessor::run_stream`
- `ProviderConfig`

### 4. 测试策略
- **测试框架**: Rust 内联单元测试
- **本次验证方式**:
  - `cargo test -p one-core provider_cache_signature_changes_with_model --lib`
  - `cargo check -p one-core`

### 5. 依赖和集成点
- **内部依赖**: `ai_chat/stream -> llm/manager -> llm/connector`
- **配置来源**: repo 中持久化的 provider 配置 + UI 运行时 `selected_model`

### 6. 技术选型理由
- **事实**: `ProviderManager` 当前缓存 key 只有 `provider_id`。
- **事实**: `ChatStreamProcessor` 原先创建 provider 时没有把 `selected_model` 写回 `config.model`。
- **推断**: 阿里云 qwen3.5-plus 即使已有正确 URL 路由补丁，也会因为 provider 仍按旧模型构造而继续复用旧 client。

### 7. 关键风险点
- **兼容性风险**: 缓存签名需要覆盖足够字段，避免错误复用。
- **行为风险**: 若 provider 配置频繁变化，缓存重建次数会增加，但成本远小于错误请求带来的影响。

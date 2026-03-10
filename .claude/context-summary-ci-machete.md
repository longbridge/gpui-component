## 项目上下文摘要（ci-machete）
生成时间：2026-03-09 23:01:51 +0800

### 1. 相似实现分析
- **实现1**: `.github/workflows/ci.yml:1`
  - 模式：CI 在 macOS job 中单独执行 `cargo-machete`，其余平台只跑测试。
  - 可复用：保留现有 workflow 结构，仅修复被检查对象。
  - 需注意：失败点发生在依赖声明治理，不是测试矩阵本身。

- **实现2**: `Cargo.toml:217`
  - 模式：工作区级 `cargo-machete` 配置，使用 `[workspace.metadata.cargo-machete]` 统一忽略已知误报依赖。
  - 可复用：若未来出现工作区公共误报，可沿用根级 `ignored` 配置。
  - 需注意：当前根配置只忽略 `log`、`anyhow`、`serde`，未覆盖 `one-core` 被报出的 7 个依赖。

- **实现3**: `crates/macros/Cargo.toml:20`
  - 模式：包级 `cargo-machete` 配置，使用 `[package.metadata.cargo-machete]` 处理局部误报。
  - 可复用：若 `one-core` 存在真实误报，可在该 crate 内局部忽略。
  - 需注意：只有在确认依赖通过宏或特殊路径间接使用时才应加 ignore。

- **实现4**: `main/src/update.rs:806`
  - 模式：Rust 测试采用 `#[cfg(test)] mod tests` 与 `#[test]` 组织。
  - 可复用：本次验证若需补测试，应沿用该风格。
  - 需注意：当前任务更偏依赖治理，优先做构建与 machete 验证。

### 2. 项目约定
- **命名约定**: Rust 函数与模块使用 `snake_case`，crate 名称使用短横线或下划线与现有包保持一致。
- **文件组织**: 工作流放在 `.github/workflows/`，依赖治理集中在各 crate `Cargo.toml` 或根 `Cargo.toml` 的 metadata 段。
- **导入顺序**: Rust 源码普遍按标准库、第三方、项目内模块组织；本次若仅改 `Cargo.toml` 不涉及 import。
- **代码风格**: 倾向最小改动，优先删除无效声明，而不是扩大全局例外。

### 3. 可复用组件清单
- `Cargo.toml:217`: 工作区级 `cargo-machete` 忽略配置样例。
- `crates/macros/Cargo.toml:20`: 包级 `cargo-machete` 忽略配置样例。
- `.github/workflows/ci.yml:32`: CI 中 `Machete` 步骤定义，可用于定位失败入口。

### 4. 测试策略
- **测试框架**: Rust 内建测试框架。
- **测试模式**: 以 `cargo test`、`cargo check` 为主，仓库规范另要求运行 `cargo machete`。
- **参考文件**: `main/src/update.rs:806`
- **覆盖要求**: 本次至少覆盖未使用依赖检查通过与受影响 crate 可编译。

### 5. 依赖和集成点
- **外部依赖**: `cargo-machete v0.9.1` 在 GitHub Actions 中执行。
- **内部依赖**: 受影响对象为 `crates/core/Cargo.toml`，不涉及其他 crate 接口变化。
- **集成方式**: 通过 CI workflow 的 `Machete` 步骤扫描工作区依赖。
- **配置来源**: `.github/workflows/ci.yml`、根 `Cargo.toml`、`crates/core/Cargo.toml`、`crates/macros/Cargo.toml`。

### 6. 技术选型理由
- **为什么用这个方案**: 本地精确搜索未发现 `reqwest::`、`rustls::`、`regex::`、`http_body_util::`、`bytes::`、`urlencoding::` 等在 `crates/core/src` 中的引用，优先删除声明比新增 ignore 更符合依赖治理目标。
- **优势**: 减少依赖图、降低维护成本、让 CI 直接恢复绿色。
- **劣势和风险**: 若存在宏展开、特性门控或生成代码的间接使用，直接删除会在编译阶段暴露问题。

### 7. 关键风险点
- **并发问题**: 无。
- **边界条件**: `cargo-machete` 可能对宏或隐式使用产生误报。
- **性能瓶颈**: 无运行时性能风险，只有本地验证的构建耗时。
- **安全考虑**: 本次任务不新增安全逻辑，也不以安全为验收条件。

### 8. 已知证据
- GitHub Actions 截图显示 `cargo-machete` 报告 `crates/core/Cargo.toml` 中 `bytes`、`http-body-util`、`regex`、`reqwest`、`rustls`、`rustls-platform-verifier`、`urlencoding` 未使用。
- 本地搜索结果显示 `crates/core/src` 中未命中 `reqwest::`、`rustls::`、`regex::`、`http_body_util::`、`bytes::`、`urlencoding::`。
- 根 `Cargo.toml` 与 `crates/macros/Cargo.toml` 均已存在 `cargo-machete` 配置，证明仓库接受基于证据的局部例外策略。

## 项目上下文摘要（ci-machete-db-once-cell）
生成时间：2026-03-20 15:10:42 +0800

### 1. 相似实现分析
- 实现1：`.github/workflows/ci.yml`
  - 模式：仅 `macos-latest` job 执行 `Machete`
  - 可复用：确认失败入口就是 `cargo machete`
  - 注意点：修复目标应聚焦依赖治理，不要误改测试逻辑
- 实现2：`crates/macros/Cargo.toml`
  - 模式：对 proc-macro 场景使用 `[package.metadata.cargo-machete].ignored`
  - 可复用：当出现宏/工具误报时可按包级配置 ignore
  - 注意点：只有在确认为误报时才应使用 ignore
- 实现3：`crates/db/Cargo.toml`
  - 模式：普通业务 crate 直接声明工作区依赖
  - 可复用：若依赖无实际使用，应直接删除而非增加 ignore
  - 注意点：当前 `once_cell.workspace = true` 在源码中未见使用

### 2. 项目约定
- 命名约定：Rust crate 依赖按功能分组顺序排列，未使用依赖直接删除
- 文件组织：CI workflow 在 `.github/workflows/`，crate 依赖声明在各自 `Cargo.toml`
- 代码风格：优先删除真实无用依赖，仅在工具误报时使用 `cargo-machete` metadata

### 3. 可复用组件清单
- `.github/workflows/ci.yml`：CI 失败入口与执行条件
- `crates/macros/Cargo.toml`：现有 `cargo-machete` ignore 配置范式
- `crates/db/Cargo.toml`：本次修复目标

### 4. 测试策略
- 静态搜索 `crates/db/src` 中是否存在 `once_cell`/`OnceCell`/`Lazy` 使用
- 执行 `cargo check -p db` 验证删除依赖后 crate 仍能编译
- 当前本机未安装 `cargo-machete`，无法直接本地复跑子命令；以代码证据和编译验证替代，并等待 CI 闭环

### 5. 依赖和集成点
- CI 入口：`.github/workflows/ci.yml` 的 `Machete` 步骤
- 修复点：`crates/db/Cargo.toml`
- 参考模式：`crates/macros/Cargo.toml` 的 `package.metadata.cargo-machete`

### 6. 技术选型理由
- 选择删除 `once_cell`，因为 `crates/db/src` 内没有实际引用，符合“真实未使用依赖”的特征
- 不新增 ignore，因为当前证据不支持这是 `cargo-machete` 误报
- 不改 workflow，因为失败来自 crate 依赖清单而非 CI 编排

### 7. 关键风险点
- 当前未安装 `cargo-machete`，无法本机直接验证工具输出是否已清空
- 如果 CI 后续还报出别的 crate 未使用依赖，需要按同样方法继续清理

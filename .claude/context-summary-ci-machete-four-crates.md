## 项目上下文摘要（ci-machete-four-crates）
生成时间：2026-03-20 17:39:45 +0800

### 1. 相似实现分析
- **实现1**: [.github/workflows/ci.yml](/Users/hufei/RustroverProjects/onetcli/.github/workflows/ci.yml)
  - 模式：`macos-latest` job 在测试前先执行 `cargo machete`
  - 可复用：本地验证以 `cargo machete` 为准，而不是 `--with-metadata`
  - 需注意：只要 `Machete` 失败，后续 `clippy` 和 `test` 都不会进入

- **实现2**: [Cargo.toml](/Users/hufei/RustroverProjects/onetcli/Cargo.toml)
  - 模式：工作区统一声明依赖，crate 内按需引用
  - 可复用：真实未使用依赖应直接从目标 crate 的 `Cargo.toml` 删除
  - 需注意：根级目前没有针对这四个 crate 的全局 ignore 约定

- **实现3**: [crates/macros/Cargo.toml](/Users/hufei/RustroverProjects/onetcli/crates/macros/Cargo.toml)
  - 模式：仅对 proc-macro 误报使用 `[package.metadata.cargo-machete]`
  - 可复用：只有确认误报时才使用局部 ignored
  - 需注意：本次目标不是宏 crate，优先删依赖而不是加 ignored

- **实现4**: [crates/one_ui/src/lib.rs](/Users/hufei/RustroverProjects/onetcli/crates/one_ui/src/lib.rs)
  - 模式：`one_ui` 目前只暴露 `edit_table` 与 `resize_handle`
  - 可复用：通过源码反查实际依赖，仅保留 `gpui`、`gpui-component`、`tracing`
  - 需注意：`time/mod.rs` 为空，说明日历类依赖没有实际接线

### 2. 项目约定
- **命名约定**: crate 依赖声明保持工作区依赖写法，最小化声明集
- **文件组织**: 依赖治理只改各 crate `Cargo.toml`
- **代码风格**: 优先删除真实未使用依赖，只有误报才加 metadata ignore

### 3. 可复用组件清单
- [Cargo.toml](/Users/hufei/RustroverProjects/onetcli/Cargo.toml): 工作区依赖源
- [crates/macros/Cargo.toml](/Users/hufei/RustroverProjects/onetcli/crates/macros/Cargo.toml): 包级 `cargo-machete` ignore 样例
- [crates/one_ui/src/edit_table/mod.rs](/Users/hufei/RustroverProjects/onetcli/crates/one_ui/src/edit_table/mod.rs): `one_ui` 的实际导出入口

### 4. 测试策略
- 先运行 `cargo check -p db_view`
- 再运行 `cargo check -p redis_view`
- 再运行 `cargo check -p terminal_view`
- 再运行 `cargo check -p one-ui`
- 最后运行根目录 `cargo machete`

### 5. 依赖和集成点
- **外部依赖**: `bnjbvr/cargo-machete@v0.9.1`
- **内部依赖**: `db_view`、`redis_view`、`terminal_view`、`one_ui`
- **集成方式**: GitHub Actions `CI` workflow 的 `Machete` 步骤

### 6. 技术选型理由
- **为什么用删除方案**: 本地 `cargo machete` 输出与源码搜索一致，说明这四个 crate 主要是声明冗余
- **优势**: 不引入新的 ignore 维护成本，直接缩小依赖图
- **劣势和风险**: 如果后续有隐藏的特性门控引用，会在 `cargo check` 阶段暴露

### 7. 关键风险点
- `cargo machete --with-metadata` 会报更多 crate，但当前 CI 不使用该模式
- `one_ui` 依赖收缩幅度较大，必须用依赖它的 crate 一并编译回归

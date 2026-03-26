## 项目上下文摘要（ci-followup-build-ssh）
生成时间：2026-03-26 10:36:52 +0800

### 1. 相似实现分析
- **实现1**: `crates/core/build.rs`
  - 模式：遍历环境变量列表，打印 `cargo:rerun-if-env-changed` 与 `cargo:rustc-env`。
  - 需注意：当前嵌套 `if let` + `if !val.is_empty()` 会触发 `clippy::collapsible_if`。
- **实现2**: `main/build.rs`
  - 模式：与 `crates/core/build.rs` 相同，额外包含 Windows 资源编译逻辑。
  - 需注意：虽然本次报错先出在 `crates/core/build.rs`，但这里存在同样写法，适合一并统一。
- **实现3**: `crates/ssh/src/ssh.rs:360-399`
  - 模式：`#[cfg(test)] mod tests` 中定义了一个仅供 `#[cfg(unix)]` 测试使用的 helper 和环境锁。
  - 需注意：在 Windows 测试编译时，`Mutex`、`OnceLock` 与 `test_auth_failure_messages` 不再被引用，因此会触发 `unused-imports` 和 `dead-code`。

### 2. 项目约定
- **代码风格**: 优先最小语义等价修改，直接修复 lint，而不是加 `allow`。
- **文件组织**: build script 各自自包含；平台专属测试辅助应按 `#[cfg(unix)]` 收紧作用域。

### 3. 可复用组件清单
- `AuthFailureMessages`：`ssh.rs` 测试 helper 的构造目标类型。
- `connect_agent_client`：`ssh.rs` 当前 Unix 测试真正验证的函数。

### 4. 测试策略
- **验证命令**:
  - `cargo clippy -p one-core -p main --all-targets -- -D warnings`
  - `cargo test -p ssh --lib`
- **验证目标**: 消除 build script 的 `collapsible_if`，并确保 `ssh` 测试模块在当前平台可编译；Windows 问题通过 `#[cfg(unix)]` 作用域收紧静态消除。

### 5. 风险点
- `main/build.rs` 与 `crates/core/build.rs` 若只修其中一个，后续仍可能在同一 lint 阶段继续报错。
- `ssh.rs` 若只删导入而不处理 helper 的 `cfg`，Windows 仍会因 dead code 失败。

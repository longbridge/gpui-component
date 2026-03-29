## 项目上下文摘要（objc-cargo-clippy）
生成时间：2026-03-28 22:19:18 +0800

### 1. 相似实现分析
- **实现1**: `main/src/onetcli_app.rs:103`
  - 模式：`restore_window` 在 macOS 下通过 `raw-window-handle` 拿到 `ns_view`，再用 `objc::msg_send!` 获取 `NSWindow` 并调用 `deminiaturize` / `makeKeyAndOrderFront`。
  - 可复用：说明本仓库已有稳定的 AppKit 桥接路径，问题不在业务流程，而在 `msg_send!` 宏展开。
  - 需注意：这里的 3 个 `msg_send!` 都会触发同类 `unexpected_cfgs` 警告。

- **实现2**: `main/src/main.rs:25`
  - 模式：`macos_activation_restore` 模块统一使用 `class!`、`sel!`、`msg_send!` 与 `cocoa` / `objc` 做 AppKit 交互。
  - 可复用：证明 `main` crate 内多处复用同一套 `objc 0.2.7` 宏，不适合逐调用点加属性绕过。
  - 需注意：`register` 中的 `class!(NSObject)`、`sel!(applicationDidBecomeActive:)` 和多处 `msg_send!` 都会命中同一警告。

- **实现3**: `main/src/main.rs:98`
  - 模式：`restore_first_minimized_window`、`app_is_active`、`has_visible_window` 等纯辅助函数保留现有窗口恢复逻辑，核心风险只在宏展开的编译阶段。
  - 可复用：说明修复应优先落在 Cargo lint / 依赖兼容层，而不是改动这些运行时逻辑。
  - 需注意：这些函数内部也继续使用 `msg_send!`，只修一处源码无法覆盖全部报错点。

- **实现4**: `main/src/onetcli_app.rs:570`
  - 模式：本仓库 Rust 测试习惯是在同文件 `#[cfg(test)] mod tests` 中补纯函数或行为测试。
  - 可复用：本次如需补回归验证，应优先沿用 `cargo check` / `cargo test` 的最小范围验证，而不是新增外部脚本。
  - 需注意：当前问题属于编译期 lint 兼容性，验证重点是构建命令而非运行时单测。

### 2. 项目约定
- **命名约定**: Rust 代码使用 `snake_case`；Cargo 配置沿用标准段落命名。
- **文件组织**: `main/Cargo.toml` 负责 `main` crate 依赖，根 `Cargo.toml` 负责工作区级依赖与 lint 约定。
- **导入顺序**: macOS 平台导入集中在 `#[cfg(target_os = "macos")]` 块，说明平台差异集中管理。
- **代码风格**: 历史修复倾向最小范围改动，优先在配置层一次性解决公共问题，而不是为每个调用点加例外。

### 3. 可复用组件清单
- `main/Cargo.toml:38`：macOS 目标依赖声明，确认当前依赖为 `cocoa = "0.25.0"`、`objc = "0.2.7"`。
- `Cargo.toml:135`：工作区已存在 `[workspace.lints.clippy]`，说明仓库接受集中式 lint 配置。
- `main/src/onetcli_app.rs:103`：macOS 窗口恢复路径。
- `main/src/main.rs:25`：macOS 激活恢复与全局热键路径。

### 4. 测试策略
- **测试框架**: Rust 内建测试框架 + Cargo 构建检查。
- **测试模式**: 以 `cargo check -p main` 和严格 warnings 构建验证为主。
- **参考文件**: `main/src/onetcli_app.rs:570`、`main/src/main.rs:294`
- **覆盖要求**: 至少验证默认 `cargo check -p main` 不再出现该警告，并验证严格 warnings 模式不再因此失败。

### 5. 依赖和集成点
- **外部依赖**: `objc 0.2.7`、`cocoa 0.25.0`、Rust `check-cfg` / `unexpected_cfgs` lint。
- **内部依赖**: `main` crate 通过 `[lints] workspace = true` 继承工作区 lint 配置。
- **集成方式**: `class!` / `sel!` / `msg_send!` 宏在 `main` crate 源码展开时触发 `unexpected_cfgs`。
- **配置来源**: 根 `Cargo.toml`、`main/Cargo.toml`、`Cargo.lock`。

### 6. 技术选型理由
- **为什么用这个方案**: Rust 官方文档支持在 Cargo lint 配置中显式声明允许的 `cfg` 值；当前仓库又已经采用工作区级 lint 配置，因此优先用 `check-cfg` 兼容旧依赖宏最符合现有模式。
- **优势**: 一处配置即可覆盖 `main` crate 内所有 `objc` 宏调用；不改业务逻辑；无运行时成本。
- **劣势和风险**: 这是对旧依赖宏的兼容声明，不是升级依赖的根治方案；未来若升级 `objc`，应评估是否删除该兼容项。

### 7. 关键风险点
- **并发问题**: 无，变更仅涉及编译期 lint 配置。
- **边界条件**: 若严格 warnings 下还有其他独立告警，修复本项后仍可能继续失败。
- **性能瓶颈**: 无运行时影响，仅增加极少量 Cargo 配置解析。
- **安全考虑**: 本次任务不涉及安全逻辑。

### 8. 已知证据
- 本地 `cargo check -p main` 已稳定复现 `unexpected cfg condition value: cargo-clippy`，覆盖 `main/src/onetcli_app.rs` 与 `main/src/main.rs` 多处宏调用。
- `Cargo.lock:6333` 显示工作区当前锁定 `objc 0.2.7`。
- Rust 官方 `check-cfg` 文档说明可通过 `[lints.rust.unexpected_cfgs]` 的 `check-cfg` 声明预期值。
- 上游 `rust-objc` 宏源码保留 `#[cfg(feature = "cargo-clippy")]` 分支，佐证问题来自旧宏实现与新版 `check-cfg` 的兼容性。

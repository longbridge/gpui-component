## 项目上下文摘要（typos-ci-fix）
生成时间：2026-03-26 10:09:42 +0800

### 0. 需求变更说明
- 初始方向是通过 `Cargo.toml` 白名单修复 `typos` 误报。
- 用户随后明确要求“`typos` 去掉这个检查”，因此最终实施方向改为移除 `typos` 检查链路，而不是继续保留白名单方案。
- 下文保留的是做出该决策前的上下文依据；最终落地以移除 CI、配置和文档中的 `typos` 入口为准。

### 1. 相似实现分析
- **实现1**: `Cargo.toml:205-209`
  - 模式：仓库把 `typos` 配置直接放在根 `Cargo.toml` 的 `workspace.metadata.typos` 中，而不是单独维护 `_typos.toml`。
  - 可复用：现有 `files.extend-exclude` 与 `default.extend-identifiers` 配置组织方式。
  - 需注意：本次修复应继续集中在这个配置入口，避免分散到多个配置文件。
- **实现2**: `.github/workflows/ci.yml:49-53`
  - 模式：CI 在 macOS 任务中直接执行 `typos`，没有传入额外参数。
  - 可复用：只要根配置生效，CI 无需同步修改。
  - 需注意：任何方案都必须兼容命令行直接读取 `Cargo.toml` 元数据的现有行为。
- **实现3**: `crates/db/src/mssql/plugin.rs:117-126`
  - 模式：数据库插件通过 `SqlCompletionInfo.functions` 暴露方言函数说明。
  - 可复用：`IIF(cond, then, else)` 属于 SQL Server 合法函数签名，应保留字面量。
  - 需注意：这里不是英文拼写错误，不能为了过 `typos` 去改业务字符串。
- **实现4**: `crates/db/src/sqlite/plugin.rs:286-292`
  - 模式：SQLite 插件同样通过 `functions` 列表暴露 SQL 函数说明。
  - 可复用：`IIF(cond, x, y)` 是 SQLite 合法函数签名，说明多个方言会共享同一保留词。
  - 需注意：白名单应该覆盖跨文件重复出现的同类词，而不是做局部注释规避。
- **实现5**: `crates/db_view/src/sql_inline_completion.rs:17-58` 与 `:733-979`
  - 模式：`COMPOUND_KEYWORDS` 和测试用例故意保存 `SEL/SELEC/INSER/...` 这类未完成 SQL 片段，用于生成 ghost text 补全。
  - 可复用：补全实现与测试都依赖这些前缀/后缀字面量，不能改写语义。
  - 需注意：`ECT/ect`、`SELEC`、`INSER`、`UPDAT`、`DELET`、`WHE`、`WHER`、`HAV`、`HAVIN` 都是有意设计的 token。

### 2. 项目约定
- **命名约定**: Rust 代码使用 `snake_case` / `PascalCase`，工具配置延续 TOML 分段命名。
- **文件组织**: 根级工具配置集中放在 `Cargo.toml`；CI 定义位于 `.github/workflows/ci.yml`；任务留痕放在项目本地 `.claude/`。
- **代码风格**: 优先最小改动和集中配置，不为工具误报重构业务代码。

### 3. 可复用组件清单
- `Cargo.toml` 中现有 `[workspace.metadata.typos]`：本次唯一需要修改的运行配置入口。
- `ci.yml` 中现有 `typos` 命令：作为本地复现和最终验证的对照。
- `sql_inline_completion.rs` 的现有测试：用于证明前缀片段是功能需求，而非脏数据。

### 4. 官方文档与外部参考
- Context7 `/crate-ci/typos` 文档确认：可通过 `[default.extend-words]` 与 `[default.extend-identifiers]` 扩展词典，也可用 `extend-ignore-words-re` 做正则忽略。
- GitHub `search_code` 检索结果表明：Rust 仓库普遍直接在 `Cargo.toml` 的 `workspace.metadata.typos` 中维护白名单，符合当前仓库做法。

### 5. 测试策略
- **验证命令**: 直接运行 `typos`，与 CI 工作流保持一致。
- **覆盖要求**: 至少覆盖本次报错列出的 `IIF/ECT/ect/SELEC/INSER/UPDAT/DELET/WHE/WHER/HAV/HAVIN`。
- **回归要求**: 不修改 `crates/db` 与 `crates/db_view` 的运行逻辑和测试语义。

### 6. 依赖和集成点
- **外部依赖**: `typos-cli` 通过根配置读取白名单。
- **内部依赖**: `crates/db` 的插件说明字符串、`crates/db_view` 的内联补全前缀和测试断言。
- **集成方式**: 仅通过根配置扩展词典，无需修改 CI 和业务模块接口。

### 7. 技术选型理由
- **为什么用 `extend-words`**: 报错 token 都出现在字符串文本里，不是 Rust 标识符；集中白名单比改代码更直接，也更符合现有 `Cargo.toml` 配置模式。
- **优势**: 运行时零成本、变更面最小、后续维护集中。
- **风险**: 如果 `typos` 对大小写处理与预期不一致，需要补充对应大小写词条；可通过本地执行立即验证。

### 8. 关键风险点
- **边界条件**: `ECT` 与 `ect` 同时出现在常量和测试中，需要确认白名单覆盖大小写场景。
- **维护风险**: 后续若新增更多 SQL 前缀片段，仍需同步更新白名单。
- **验证缺口**: 本次不涉及运行时代码路径，本地验证主要依赖 `typos` 静态检查。 

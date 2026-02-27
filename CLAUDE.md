# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**onetcli** (One Net Client) is a cross-platform desktop application built on [GPUI](https://gpui.rs) that provides a unified interface for database management, SSH/SFTP, terminal, and AI tools.

Key capabilities:
- Multi-protocol database management (PostgreSQL, MySQL, SQLite, SQL Server, Oracle, ClickHouse)
- Redis and MongoDB views
- SSH terminal and SFTP file management
- Local terminal with multi-tab workflows
- Cloud sync and account system with encrypted key storage
- Built-in AI chat (OnetCli Provider via `llm-connector`)

## Common Commands

```bash
# Run the application
cargo run -p main

# Build
cargo build

# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test -p gpui-component
cargo test -p db
cargo test -p one-core

# Run doc tests
cargo test -p gpui-component --doc

# Lint
cargo clippy -- --deny warnings

# Format check
cargo fmt --check

# Spell check
typos

# Unused dependency check
cargo machete

# Performance profiling (macOS)
MTL_HUD_ENABLED=1 cargo run -p main
samply record cargo run -p main

# Install system dependencies (Linux/macOS)
script/bootstrap
```

## Workspace Structure

The workspace has four default members (`crates/ui`, `crates/story`, `crates/assets`, `main`) and 25+ total crates.

### Application Layer

- **`main/`** — Application entry point and main UI. Orchestrates all subsystems: auth, settings, licensing, updates, home page. Entry: `main/src/main.rs` → `OnetCliApp`.

### Core Infrastructure

- **`crates/core` (one-core)** — Core logic: connection management, cloud sync, AI integration, configuration, encryption (AES-GCM, Ed25519).
- **`crates/ui` (gpui-component)** — Reusable UI component library (60+ components). Published to crates.io.
- **`crates/one_ui`** — Application-specific UI components extending gpui-component.
- **`crates/macros` (gpui-component-macros)** — Procedural macros.
- **`crates/assets` (gpui-component-assets)** — Bundled static assets (rust-embed).

### Feature Crates (Backend + View Pairs)

| Domain | Backend Crate | View Crate |
|--------|--------------|------------|
| Database | `crates/db` | `crates/db_view` |
| Terminal | `crates/terminal` | `crates/terminal_view` |
| SSH/SFTP | `crates/ssh`, `crates/sftp` | `crates/sftp_view` |
| Redis | — | `crates/redis_view` |
| MongoDB | — | `crates/mongodb_view` |

### Utilities

- **`crates/reqwest_client`** — HTTP client wrapper around Zed's custom reqwest fork.
- **`crates/webview` (gpui-wry)** — WebView integration via Wry.
- **`crates/license_tool`** — License key generation and management.
- **`crates/story`** — Component gallery/showcase app (runs with `cargo run` from default members).

## Application Initialization Flow

The startup sequence in `main/src/main.rs` is order-sensitive:

1. Handle update commands → Load `.env.local` / `.env`
2. Create `Application` with `Assets`
3. `onetcli_app::init(cx)` — tracing, theme, HTTP client
4. `setting_tab::init_settings(cx)` — settings system
5. `GlobalDbState::new()` + cleanup task → `cx.set_global()`
6. `DatabaseViewPluginRegistry` → `cx.set_global()`
7. `gpui_component::init(cx)` — **must be called before any UI component usage**
8. `one_core::init(cx)`, `one_ui::init(cx)` — core and UI subsystem init
9. Auth, license, terminal, Redis, MongoDB subsystem init
10. Open window with `Root::new(OnetCliApp, window, cx)` — **Root must be the first view**

## Architecture Patterns

### Root View System

Every window's outermost view must be a `Root`. It manages sheets, dialogs, notifications, and keyboard navigation (Tab/Shift-Tab).

### Dock System

Panel layout system with drag-and-drop, zoom, serialization:
- `DockArea` → `DockItem` tree (`Split` | `Tabs` | `Panel`)
- Panels implement `PanelView` trait
- `PanelRegistry` handles serialization/deserialization

### Component Design

- **Stateless preferred**: Use `RenderOnce` trait when possible
- **Size system**: `xs`, `sm`, `md` (default), `lg` via `Sizable` trait
- **Cursor convention**: Buttons use `default` cursor (desktop convention), not `pointer`, unless link-style
- **Styling**: CSS-like API via `Styled` trait and `ElementExt`

### Theme System

- `Theme` global singleton, light/dark mode
- Access via `ActiveTheme` trait: `cx.theme()`
- Covers colors (`ThemeColor`), syntax highlighting, fonts, border radius, shadows, scrollbar mode

### Input System

Text input based on Rope (`ropey` crate) with:
- LSP integration (diagnostics, completion, hover)
- Tree-sitter syntax highlighting
- Variants: `Input`, `NumberInput`, `OtpInput`

## Configuration

- **Environment files**: `.env.local` (priority) → `.env` (fallback)
- **Build-time config**: `SUPABASE_URL`, `SUPABASE_ANON_KEY` can be baked in at compile time, overridden at runtime
- **Update URL**: `ONETCLI_UPDATE_URL` env var

## Language Convention

- **简体中文**：AI 回复、代码注释、Git 提交信息、文档等一律使用简体中文
- **唯一例外**：代码标识符（变量名、函数名、类名等）遵循项目既有英文命名约定

## Code Style

- Follow existing patterns and naming conventions
- Reference macOS/Windows control API design for component naming
- AI-generated code must be refactored to match project style; mark AI portions in PRs
- Clippy: `dbg_macro` and `todo` are **denied**
- All source files must use UTF-8 encoding (no BOM)
- Comments should describe intent, constraints, and usage — not restate code logic
- Do not write "changelog-style" comments; let version control track changes

## Icon System

`Icon` does not bundle SVGs. Use [Lucide](https://lucide.dev) icons, naming files per the `IconName` enum in `crates/ui/src/icon.rs`.

## Internationalization

Uses `rust-i18n`. Locale files in `crates/ui/locales/`. Default locales: `en`, `zh-CN`, `zh-HK`.

## Platform Support

- macOS (aarch64, x86_64) — Metal backend
- Linux (x86_64) — Vulkan, client-side decorations, gtk dependencies
- Windows (x86_64) — 8MB stack size configured in `.cargo/config.toml`

CI runs clippy (macOS), machete (macOS), typos (macOS), and full test suite on all three platforms.

## Key Dependencies

- **GPUI**: Git dependency from `zed-industries/zed`
- **reqwest**: Zed's custom fork (`zed-reqwest`)
- **Database drivers**: tokio-postgres, mysql_async, rusqlite, tiberius+bb8, oracle, clickhouse
- **SSH/SFTP**: russh ecosystem (russh, russh-sftp, russh-keys)
- **Terminal**: alacritty_terminal
- **AI**: llm-connector (with streaming)
- **Text**: ropey (Rope), tree-sitter (syntax highlighting)

## Testing Guidelines

See `.claude/COMPONENT_TEST_RULES.md`:
- Focus on complex logic, avoid excessive simple tests
- Every component needs a `test_*_builder` test for the builder pattern
- Test state transitions, branching, and edge cases
- Use `#[gpui::test]` macro for GPUI tests
- Tests must cover normal flow, boundary conditions, and error recovery
- Missing test coverage should be documented as a risk with a remediation plan

## Design Principles

- Follow SOLID, DRY, and separation of concerns; shared logic should be abstracted into reusable components
- Prefer dependency inversion and interface isolation; avoid hard-coding implementation details
- Break complex logic into separate responsibilities before coding
- Each function or type should have a single responsibility
- Avoid premature abstraction — only generalize after three or more repetitions
- Prioritize readability over cleverness; if extra explanation is needed, simplify further
- **Standardization + ecosystem reuse has highest priority**: always look for official SDKs, mature community solutions, or existing modules before building custom implementations; only build new abstractions when existing solutions cannot meet requirements and the justification is documented

## Implementation Standards

- No MVP stubs, minimal implementations, or placeholders — deliver complete functionality before committing
- Proactively remove obsolete, duplicated, or dead code to keep the codebase clean
- Evaluate time complexity, memory usage, and I/O impact during design; avoid unnecessary overhead
- Identify potential bottlenecks and provide monitoring or optimization suggestions
- Do not introduce unevaluated expensive dependencies or blocking operations
- For breaking changes, provide migration steps or rollback plan

## Development Workflow

- **Research before coding**: Read existing code and documentation before implementing; analyze at least 3 similar implementations to identify reusable interfaces and constraints
- **Prefer reuse over reinvention**: Use existing libraries, utilities, and helper functions first; only build new abstractions when justified
- **Consistency over preference**: Follow project conventions for naming, imports, formatting, and testing patterns
- **Incremental iteration**: Keep each change compilable and verifiable; commit in small, working increments
- **Use context7 for library documentation**: When working with external libraries or frameworks, first call `resolve-library-id` to get the library ID, then call `query-docs` with an optional topic to retrieve up-to-date API references
- **Use github.search_code for reference implementations**: Search open-source examples to learn best practices when implementing common patterns
- **Halt on repeated failure**: After 3 consecutive failures on the same task, stop and reassess the strategy before continuing
- **Task decomposition**: For cross-module work or tasks with more than 5 subtasks, generate a structured task breakdown and track progress

## MCP Tool Integration

可用的 MCP 工具及使用场景：

- **context7**：编程库/框架/SDK 文档查询（最高优先级）。先 `resolve-library-id` 获取库 ID，再 `query-docs` 获取文档，可用 topic 参数聚焦特定主题
- **sequential-thinking**：复杂问题的深度分析与推理。在架构决策、多方案权衡、问题排查等场景下使用，帮助梳理思路和识别风险
- **desktop-commander**：本地文件操作与进程管理。支持文件读写（`read_file`/`write_file`）、精确文本替换（`edit_block`）、目录管理、流式搜索（`start_search`）、交互式进程（`start_process` + `interact_with_process`）
- **github**：GitHub 仓库操作。代码搜索（`search_code`）、PR/Issue 管理、代码审查、文件操作等
- **shrimp-task-manager**：结构化任务管理。适用于复杂任务的规划（`plan_task`）、分解（`split_tasks`）、执行跟踪（`execute_task`）和验证（`verify_task`）

### 工具选择优先级

| 需求场景             | 优先工具            | 备选      |
|---------------------|--------------------|-----------|
| 编程库文档查询        | context7           | WebSearch |
| 本地文件分析/数据处理  | desktop-commander  | Bash      |
| 代码搜索（开源参考）   | github.search_code | WebSearch |
| 复杂问题推理          | sequential-thinking | —         |
| 任务规划与跟踪        | shrimp-task-manager | —         |

## Skills Reference

Custom Claude Code skills in `.claude/skills/` cover:
- **Component development**: `new-component`, `generate-component-story`, `generate-component-documentation`
- **GPUI framework**: `gpui-action`, `gpui-async`, `gpui-context`, `gpui-element`, `gpui-entity`, `gpui-event`, `gpui-focus-handle`, `gpui-global`, `gpui-layout-and-style`, `gpui-style-guide`, `gpui-test`
- **Other**: `github-pull-request-description`

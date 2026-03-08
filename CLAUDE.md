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

# Run the component gallery/story showcase
cargo run  # (default members include crates/story)

# Run a single example
cargo run --example table

# Performance profiling (macOS)
MTL_HUD_ENABLED=1 cargo run -p main
samply record cargo run -p main

# Install system dependencies (Linux/macOS)
script/bootstrap

# Install system dependencies (Windows PowerShell)
.\script\install-window.ps1

# Bump version for release
./script/bump-version.sh x.y.z
```

## Workspace Structure

The workspace has four default members (`crates/ui`, `crates/story`, `crates/assets`, `main`) and 25+ total crates. Edition is 2024.

### Application Layer

- **`main/`** — Application entry point and main UI. Orchestrates all subsystems: auth, settings, licensing, updates, home page. Entry: `main/src/main.rs` → `OnetCliApp`.

### Core Infrastructure

- **`crates/core` (one-core)** — Core logic: connection management, cloud sync, AI integration, configuration, encryption (AES-GCM, Ed25519), tab container/persistence.
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
- **`examples/`** — Standalone examples (`hello_world`, `input`, `dialog_overlay`, `webview`, `system_monitor`, `focus_trap`, etc.).

## Application Initialization Flow

The startup sequence in `main/src/main.rs` and `main/src/onetcli_app.rs` is order-sensitive:

1. `update::handle_update_command()` — handle self-update CLI commands
2. `load_env_files()` — load `.env.local` / `.env` from CWD + exe directory + Resources
3. `Application::new().with_assets(Assets)` — create app with bundled assets
4. `onetcli_app::init(cx)` — tracing, HTTP client, then subsystem init:
   - `gpui_component::init(cx)` — **must be called before any UI component usage**
   - `one_core::init(cx)`, `one_ui::init(cx)` — core and UI subsystem init
   - `db_view::chatdb::agents::init(cx)` — chat DB agents
   - Auth init → license init → AI provider state setup
   - `db::init_cache(cx)` + disk cache cleanup task
   - `terminal_view::init(cx)`, `redis_view::init(cx)`, `mongodb_view::init(cx)`
   - Key bindings (`shift-escape` → ToggleZoom, `ctrl-w` → ClosePanel)
   - `TabContentRegistry` global setup
5. `setting_tab::init_settings(cx)` — settings system
6. `GlobalDbState::new()` + cleanup task → `cx.set_global()`
7. `db_view::init_ask_ai_notifier(cx)` — Ask AI notifier
8. `DatabaseViewPluginRegistry` → `cx.set_global()`
9. Open window with `Root::new(OnetCliApp, window, cx)` — **Root must be the outermost view**

## Architecture Patterns

### Root View System

Every window's outermost view must be a `Root`. It manages sheets, dialogs, notifications, and keyboard navigation (Tab/Shift-Tab).

### Dock System

Panel layout system with drag-and-drop, zoom, serialization:
- `DockArea` → `DockItem` tree (`Split` | `Tabs` | `Panel`)
- Panels implement `PanelView` trait
- `PanelRegistry` handles serialization/deserialization

### Tab Container System

`TabContainer` (in `one-core`) manages multi-tab workflows with:
- `TabContentRegistry` global for registering tab content types
- `TabContainerEvent` for inter-tab communication
- Tab state persistence via `one_core::tab_persistence` (save/load)

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

- **Environment files**: `.env.local` (priority) → `.env` (fallback), loaded from CWD + exe directory + macOS Resources directory
- **Build-time config**: `SUPABASE_URL`, `SUPABASE_ANON_KEY` can be baked in at compile time, overridden at runtime
- **Update URL**: `ONETCLI_UPDATE_URL` env var
- **Log level**: `RUST_LOG` env var (default: `info`)

## Language Convention

- **简体中文**：AI 回复、代码注释、Git 提交信息、文档等一律使用简体中文
- **唯一例外**：代码标识符（变量名、函数名、类名等）遵循项目既有英文命名约定

## 编码规约与开发准则

**开始编码前必须先阅读 `AGENTS.md`**，其中包含完整的编码规约、质量标准、工作流程、MCP 工具集成规则和验证机制。

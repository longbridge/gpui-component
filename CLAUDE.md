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

## Code Style

- Follow existing patterns and naming conventions
- Reference macOS/Windows control API design for component naming
- AI-generated code must be refactored to match project style; mark AI portions in PRs
- Clippy: `dbg_macro` and `todo` are **denied**

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

## Skills Reference

Custom Claude Code skills in `.claude/skills/` cover:
- **Component development**: `new-component`, `generate-component-story`, `generate-component-documentation`
- **GPUI framework**: `gpui-action`, `gpui-async`, `gpui-context`, `gpui-element`, `gpui-entity`, `gpui-event`, `gpui-focus-handle`, `gpui-global`, `gpui-layout-and-style`, `gpui-style-guide`, `gpui-test`
- **Other**: `github-pull-request-description`

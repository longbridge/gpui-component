# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GPUI Component is a UI component library for building desktop applications using [GPUI](https://gpui.rs). It provides 60+ cross-platform desktop UI components, inspired by macOS/Windows controls and combined with shadcn/ui design.

This is a Rust workspace project with the following main crates:

- `crates/ui` - Core UI component library (published as `gpui-component`)
- `crates/story` - Gallery application for showcasing and testing components
- `crates/macros` - Procedural macros (`IntoPlot` derive)
- `crates/assets` - Static assets
- `crates/webview` - WebView component support
- `examples/` - Various example applications

## Common Commands

### Development and Testing

```bash
# Run Story Gallery (component showcase application)
cargo run

# Run individual examples
cargo run --example hello_world
cargo run --example table

# Build the project
cargo build

# Lint check
cargo clippy -- --deny warnings

# Format check
cargo fmt --check

# Spell check
typos

# Check for unused dependencies
cargo machete
```

### Testing

**Note**: Per user configuration, tests do not need to be run.

```bash
# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test -p gpui-component

# Run doc tests
cargo test -p gpui-component --doc
```

### Performance Profiling

```bash
# View FPS on macOS (using Metal HUD)
MTL_HUD_ENABLED=1 cargo run

# Profile performance using samply
samply record cargo run
```

## Core Architecture

### Component Initialization

**Critical requirement**: You must call `gpui_component::init(cx)` at your application's entry point before using any GPUI Component features.

```rust
fn main() {
    let app = Application::new();
    app.run(move |cx| {
        // This must be called first
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| MyView);
                // The first level view in a window must be a Root
                cx.new(|cx| Root::new(view, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        }).detach();
    });
}
```

### Root View System

`Root` is the top-level view for a window and manages:

- Sheet (side panels)
- Dialog (dialogs)
- Notification (notifications)
- Keyboard navigation (Tab/Shift-Tab)

The first view of every window must be a `Root`.

### Theme System

- Uses `Theme` global singleton for theme configuration
- Supports light/dark mode switching
- Access theme via `ActiveTheme` trait: `cx.theme()`
- Theme configuration includes:
  - Colors (`ThemeColor`)
  - Syntax highlighting theme (`HighlightTheme`)
  - Font configuration (system font and monospace font)
  - UI parameters like border radius, shadows
  - Scrollbar display mode

### Dock System

A complex panel layout system supporting:

- **DockArea**: Main container managing center area and left/bottom/right docks
- **DockItem**: Tree-based layout structure
  - `Split`: Split layout (horizontal/vertical)
  - `Tabs`: Tab layout
  - `Panel`: Individual panel
- **Panel**: Defined via `PanelView` trait
- **PanelRegistry**: Global panel registry for serializing/deserializing layouts
- **StackPanel**: Resizable split panel container
- **TabPanel**: Tab panel container

The Dock system supports:

- Panel drag-and-drop reordering
- Panel zoom
- Layout locking
- Layout serialization/restoration

### Input System

Text input system based on Rope data structure:

- **InputState**: Input state management
- **Rope**: Efficient text storage (from ropey crate)
- LSP integration support (diagnostics, completion, hover)
- Syntax highlighting support (Tree-sitter)
- Multiple input modes:
  - Regular input (`Input`)
  - Number input (`NumberInput`)
  - OTP input (`OtpInput`)

### Component Design Principles

1. **Stateless design**: Use `RenderOnce` trait, components should be stateless when possible
2. **Size system**: Supports `xs`, `sm`, `md` (default), `lg` sizes via `Sizable` trait.
3. **Mouse cursor**: Buttons use `default` cursor not `pointer` (desktop app convention), unless it's a link button
4. **Style system**: Provides CSS-like styling API via `Styled` trait and `ElementExt` extensions

## Code Style

- Follow naming and organization patterns from existing code
- Reference macOS/Windows control API design for naming
- AI-generated code must be refactored to match project style
- Mark AI-generated portions when submitting PRs

## Icon System

The `Icon` element does not include SVG files by default. You need to:

- Use [Lucide](https://lucide.dev) or other icon libraries
- Name SVG files according to the `IconName` enum definition (located in `crates/ui/src/icon.rs`)

## Dependencies

- GPUI: Git version from Zed repository
- Tree-sitter: For syntax highlighting
- Ropey: Rope data structure for text, and `RopeExt` trait with more features.
- Markdown rendering: `markdown` crate
- HTML rendering: `html5ever` (basic support)
- Charts: Built-in chart components
- LSP: `lsp-types` crate

## Internationalization

Uses `rust-i18n` crate.

- Localization files are located in `crates/ui/locales/`.
- Only add `en`, `zh-CN`, `zh-HK` by default.

## Platform Support

- macOS (aarch64, x86_64)
- Linux (x86_64)
- Windows (x86_64)

CI runs full test suite on each platform.

## Skills Reference

This project has custom Claude Code skills in `.claude/skills/` to assist with common development tasks:

### Component Development Skills

- **new-component** - Creating new GPUI components with proper structure and patterns
- **generate-component-story** - Creating story examples for components in the gallery
- **generate-component-documentation** - Generating documentation for components

### GPUI Framework Skills

- **gpui-action** - Working with actions and keyboard shortcuts
- **gpui-async** - Async operations and background tasks
- **gpui-context** - Context management (App, Window, AsyncApp)
- **gpui-element** - Implementing custom elements using low-level Element API
- **gpui-entity** - Entity management and state handling
- **gpui-event** - Event handling and subscriptions
- **gpui-focus-handle** - Focus management and keyboard navigation
- **gpui-global** - Global state management
- **gpui-layout-and-style** - Layout and styling systems
- **gpui-test** - Writing tests for GPUI applications

### Other Skills

- **github-pull-request-description** - Writing PR descriptions

When working on tasks related to these areas, Claude Code will automatically use the appropriate skill to provide specialized guidance and patterns.

## Testing Guidelines

See `.claude/COMPONENT_TEST_RULES.md` for detailed testing principles:

- **Simplicity First**: Focus on complex logic and core functionality, avoid excessive simple tests
- **Builder Pattern Testing**: Every component should have a `test_*_builder` test covering the builder pattern
- **Complex Logic Testing**: Test conditional branching, state transitions, and edge cases

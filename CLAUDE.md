# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GPUI Component is a UI component library for building desktop applications using [GPUI](https://gpui.rs). It provides 60+ cross-platform desktop UI components, inspired by macOS/Windows controls and combined with shadcn/ui design.

This is a Rust workspace project with the following main crates:

- `crates/ui` - Core UI component library (published as `gpui-component`)
- `crates/story` - Gallery application for showcasing and testing components
- `crates/story-web` - Web version of the story gallery (using WebAssembly)
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

For pure UI visual or sizing adjustments, do not add automated tests solely to
assert presentation dimensions. Add tests when the change affects behavior,
interaction, data flow, or prevents a meaningful regression.

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

### Architecture Refactoring Constraints

The implemented foundation architecture is documented in
`docs/ARCHITECTURE.md`, with styling and motion rules in
`docs/STYLING-AND-MOTION.md`. Preserve these constraints when designing or
implementing this architecture:

- Do not modify `gpui-base` unless the user explicitly requests a Base-layer
  change. By default, implement component behavior and visual styling in
  `crates/ui` or the application layer.

- Keep `gpui-component` as the ecosystem and product brand.
- Name the foundation crate `gpui-base`.
- Follow the ownership boundary: the framework owns behavior and infrastructure;
  the application owns component source and visual style.
- Keep the base layer visually unopinionated. It may provide interaction behavior,
  accessibility, focus, overlay and popup infrastructure, positioning, animation,
  virtual lists, dock infrastructure, and semantic design tokens.
- Theme APIs must expose semantic tokens (colors, spacing, radius, typography, and
  shadows), not an ever-growing set of component-specific styling fields.
- Keep source distribution or registry tooling above the `gpui-base` seam; no
  registry or CLI crate is currently part of the workspace.
- Preserve 100% backward compatibility for existing consumers, including current
  imports such as `use gpui_component::button::Button;`.

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
            })
            .expect("Failed to open window");
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
5. **Base controls are no-style**: Base controls and parts do not install layout,
   positioning, colors, sizing, gaps, radius, borders, shadows, variants, or animation.
   Complete presentation belongs to `crates/ui` or the application. The deliberate
   exception is the foundational Base Input frame, which provides only a semantic
   one-pixel input border and semantic radius baseline; UI/application layers own
   its background, sizing, padding, typography, adornments, and richer focus style.
6. **GPUI builder style**: Keep element construction as one fluent builder chain. Express
   conditions with `when`, `when_some`, `when_none`, and `map`; do not split a chain into a
   mutable temporary element followed by imperative reassignment when the builder API can
   express the same operation.
7. **No `pub` fields on public data types**: A public struct handed across the
   `gpui-base`/application seam — a state snapshot, capability set, render context, or
   option set — keeps its fields private, is constructed with a builder, and is read
   through methods. Adding a `pub` field is a breaking change; adding one behind a builder
   is not. Setters and readers must not collide: an all-boolean type names setters after
   the field and readers `is_`/`has_`/`can_`; a type with non-boolean fields prefixes every
   setter with `with_` and keeps the field name for readers. Value types whose fields are
   the definition and cannot grow (`Point`, `Selection`, `Edges`) are exempt. See the
   "Public Data Types Across the Seam" section of `docs/ARCHITECTURE.md`.
8. **Spell `Context` out**: Name a context type `ComboboxTriggerContext`, never `…Ctx`.
   `cx` is reserved for GPUI's `App`, `Context<T>`, and `AsyncApp`, so `ctx` for anything
   else reads as a competing context. A callback receiving both takes the GPUI one as `cx`
   and names the other after what it holds (`trigger`, `state`).

## Code Style

- Follow naming and organization patterns from existing code
- Reference macOS/Windows control API design for naming
- AI-generated code must be refactored to match project style
- Mark AI-generated portions when submitting PRs
- When creating a PR, inspect previous PR titles in the repository and match
  that style. Do not blindly use conventional prefixes like `fix:` or `feat:`
  unless the existing PR title style uses them.
- When a PR changes the public API of `crates/ui`, add a `## Breaking Changes`
  section with `diff` blocks showing the old and new usage. See PR #2691 and
  `.claude/skills/gpui-component-dev/references/pr-description.md`.

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

## Documentation

- The documentation site source is in `website/`.
- Site docs have two locales: English (`website/docs/`) and Chinese (`website/zh-CN/docs/`).
- When modifying any documentation file, always sync changes to both `en` and `zh-CN` versions.
- `docs/` holds internal architecture specifications (RFC, migration status, reviews).
  These are single-language and are not published to the site; see `docs/README.md`.

## Platform Support

- macOS (aarch64, x86_64)
- Linux (x86_64)
- Windows (x86_64)

CI runs full test suite on each platform.

## Skills Reference

This project has custom Claude Code skills to assist with common development tasks:

- **gpui** (`skills/`) - GPUI framework knowledge: actions/keybindings, async, context, custom elements, entity state, events, focus, global state, layout/styling, testing
- **gpui-component** (`skills/`) - How to use gpui-component: setup, stateless/stateful patterns, common component APIs, theming
- **gpui-component-dev** (`.claude/skills/`) - Contributing to gpui-component: creating new components, writing stories, writing documentation, writing PR descriptions

When working on tasks related to these areas, Claude Code will automatically use the appropriate skill to provide specialized guidance and patterns.

## Testing Guidelines

See `.claude/COMPONENT_TEST_RULES.md` for detailed testing principles:

- **Simplicity First**: Focus on complex logic and core functionality, avoid excessive simple tests
- **Builder Pattern Testing**: Every component should have a `test_*_builder` test covering the builder pattern
- **Complex Logic Testing**: Test conditional branching, state transitions, and edge cases

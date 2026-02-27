# WASM Compilation Progress

## Summary

We've made significant progress solving WASM compilation blockers:

### ✅ Solved Issues

1. **psm (Portable Stack Manipulation)**
   - **Solution**: Patched with stacker's master branch version
   - **Status**: ✅ Compiling successfully

2. **aws-lc-sys (AWS libcrypto)**
   - **Root cause**: reqwest_client dependency
   - **Solution**: Made http-client feature optional in story crate, disabled by default for web
   - **Status**: ✅ No longer compiled for WASM

3. **errno (Error numbers)**
   - **Root cause**: No WASM support in errno crate
   - **Solution**: Created stub implementation at `crates/errno-stub` with public constructor
   - **Status**: ✅ Stub working

4. **smol/async-io/polling**
   - **Root cause**: Native async runtime not compatible with WASM
   - **Solution**: Made smol, tree-sitter, tree-sitter-navi, and color-lsp native-only dependencies
   - **Status**: ✅ No longer compiled for WASM

5. **tokio features**
   - **Root cause**: color-lsp enabled unsupported tokio features for WASM
   - **Solution**: Made color-lsp native-only dependency
   - **Status**: ✅ Resolved

6. **wasm_thread unstable feature**
   - **Root cause**: wasm_thread 0.3.3 requires nightly Rust
   - **Solution**: Use nightly Rust compiler for WASM builds
   - **Status**: ✅ Compiling with nightly

### ⚠️ Current Blocker

**tree-sitter integration** (deep code dependency)
- **Error**: tree-sitter and tree-sitter-* crates require C standard library
- **Root cause**: gpui-component's highlighter and input modules directly use tree-sitter types
- **Affected modules**:
  - `highlighter/` - syntax highlighting
  - `input/display_map/` - editor integration
  - Uses `tree-sitter::Language`, `tree-sitter::Tree`, `tree-sitter::Point`
- **Challenge**: Requires source-level conditional compilation across multiple modules

### Analysis

The current blocker is more architectural than the previous issues:

```
gpui-component (WASM target)
  ├── highlighter/ (uses tree-sitter directly)
  ├── input/display_map/ (uses tree-sitter::Point, tree-sitter::Tree)
  └── All syntax highlighting features depend on tree-sitter
```

tree-sitter is a C library that:
- Requires C standard library (stdio.h, etc.)
- Not available in WASM without Emscripten
- Deeply integrated into gpui-component's core features

## Solutions

### Option 1: Disable Editor Features for WASM

**Approach**: Add `#[cfg(not(target_arch = "wasm32"))]` to affected modules

**Files requiring changes**:
- `crates/ui/src/highlighter/` (entire module)
- `crates/ui/src/input/display_map/display_map.rs`
- `crates/ui/src/input/display_map/text_wrapper.rs`
- `crates/ui/src/input/lsp/` (entire module)
- `crates/ui/src/input/popovers/hover_popover.rs`
- `crates/ui/src/input/state.rs`
- `crates/ui/src/text/format/markdown.rs`
- `crates/ui/src/text/format/html.rs`

**Effort**: Medium (2-3 days)
**Trade-off**: No code editor, syntax highlighting, or LSP features in WASM

### Option 2: Wait for GPUI Web Maturity

GPUI team is actively developing `gpui_web` and may address tree-sitter support.

**Timeline**: Unknown (monitor https://github.com/zed-industries/zed/tree/main/crates/gpui_web)
**Effort**: Low (just wait)
**Trade-off**: Uncertain timeline

### Option 3: Use tree-sitter WASM Build

tree-sitter has WASM support via `tree-sitter-web-api` package.

**Approach**:
- Use JS/WASM tree-sitter bindings
- Create Rust ↔ JS bridge for tree-sitter functionality
- Requires significant refactoring

**Effort**: High (1-2 weeks)
**Benefit**: Full editor features in WASM

## What We've Achieved

1. ✅ Complete infrastructure (Vite, Bun, scripts, docs)
2. ✅ Gallery component implementation
3. ✅ Solved 6 major compilation blockers:
   - psm
   - aws-lc-sys
   - errno
   - smol/async-io/polling
   - tokio features
   - wasm_thread
4. ✅ Properly separated native-only dependencies
5. ✅ Feature flags for optional dependencies
6. ✅ Can compile with nightly Rust

## Next Steps

### Recommended Path

1. **Implement Option 1** - Disable editor features for WASM
   - Allows most UI components to work
   - Simplest solution
   - Clear trade-offs

2. **Create minimal story-web**
   - Showcase non-editor components
   - Still demonstrates 90% of functionality
   - Working demo site

3. **Monitor GPUI Web progress**
   - Re-evaluate quarterly
   - Consider Option 3 when needed

## Technical Details

### Patches Applied

```toml
[patch.crates-io]
psm = { git = "https://github.com/rust-lang/stacker", branch = "master" }
errno = { path = "crates/errno-stub" }
```

### Feature Flags Added

```toml
# crates/story/Cargo.toml
[features]
http-client = ["dep:reqwest_client"]
tree-sitter = ["gpui-component/tree-sitter-languages"]
default = ["http-client", "tree-sitter"]

# crates/ui/Cargo.toml
[features]
tree-sitter-languages = [...]  # Optional tree-sitter support
```

### Native-Only Dependencies

```toml
# crates/ui/Cargo.toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
smol.workspace = true
tree-sitter = "0.25.4"

# crates/story/Cargo.toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
smol.workspace = true
tree-sitter-navi = "0.2.2"
color-lsp = "0.2.0"
```

### Files Created

- `crates/errno-stub/` - WASM-compatible errno stub with public constructor
- Feature flags in story and ui crates
- Native-only dependency configuration
- Comprehensive documentation

## Compilation Requirements

- **Rust toolchain**: Nightly (for wasm_thread support)
- **Target**: wasm32-unknown-unknown
- **Command**: `cargo +nightly build --target wasm32-unknown-unknown --release`

## Conclusion

We've successfully solved most WASM compatibility issues. The remaining blocker (tree-sitter) requires architectural decisions about which features to support in WASM.

**Recommended approach**: Disable editor-specific features for WASM, allowing the majority of UI components to work in the browser.

Last Updated: 2026-02-27

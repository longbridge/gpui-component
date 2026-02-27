# GPUI Component Story Web - Current Status

## Overview

This document describes the current implementation status of the GPUI Component Story Web version and known issues.

## Implementation Complete ✅

The following components have been successfully created:

1. **Project Structure**
   - ✅ `crates/story-web` crate with proper WASM configuration
   - ✅ Frontend project using Vite + Bun
   - ✅ Build scripts and Makefile
   - ✅ Comprehensive documentation

2. **Source Code**
   - ✅ Gallery component ported from desktop version
   - ✅ WASM entry point (`init_story` function)
   - ✅ JavaScript initialization code

3. **Build Configuration**
   - ✅ Cargo.toml with WASM dependencies
   - ✅ Vite configuration with WASM plugin
   - ✅ Build automation scripts

4. **Documentation**
   - ✅ README.md
   - ✅ QUICKSTART.md
   - ✅ IMPLEMENTATION.md
   - ✅ GETTING_STARTED.md
   - ✅ CHANGELOG.md

5. **CI/CD**
   - ✅ GitHub Actions workflow for deployment

## Current Issues ⚠️

### Compilation Errors

The WASM build currently fails due to dependencies that are not WASM-compatible:

####1. `psm` crate (Portable Stack Manipulation)
- **Error**: `section too large`
- **Source**: `gpui` → `stacksafe` → `stacker` → `psm`
- **Reason**: psm requires platform-specific stack manipulation that doesn't work in WASM

#### 2. `aws-lc-sys` crate (AWS libcrypto)
- **Error**: Custom build command failed
- **Source**: `reqwest` → `rustls` → `aws-lc-sys`
- **Reason**: C library that needs native compilation, not compatible with WASM

#### 3. `errno` crate
- **Error**: Target OS is "unknown" or "none"
- **Reason**: WASM doesn't have a traditional errno system

### Root Cause

These issues stem from deep dependencies in the GPUI ecosystem that haven't been fully adapted for WASM:

```
story-web
  └── gpui-component-story
      └── reqwest_client (HTTP client - needs native APIs)
      └── gpui
          └── stacksafe (async runtime - needs native stack)
```

## Possible Solutions

### Option 1: Wait for GPUI Web Maturity (Recommended)

**Pros:**
- Official solution from GPUI team
- All features will eventually work
- No workarounds needed

**Cons:**
- Timeline unclear
- May take months

**Action Items:**
- Monitor GPUI Web development
- Test periodically as GPUI updates
- Contribute to GPUI Web if possible

### Option 2: Feature Flags Approach

Create WASM-specific feature flags to disable problematic dependencies:

```toml
[features]
default = ["full"]
full = ["reqwest_client", "lsp", "tree-sitter"]
wasm = [] # Minimal features for WASM
```

**Pros:**
- Can build limited version now
- Gradual feature enablement

**Cons:**
- Requires modifying gpui-component and story crates
- Reduced functionality

### Option 3: Alternative Backend

Use web-native APIs instead of native dependencies:

- Replace `reqwest` with `web-sys` fetch API
- Remove LSP features (not needed in browser)
- Use web Workers instead of native async runtime

**Pros:**
- Works in browser
- Better integration with web platform

**Cons:**
- Significant refactoring
- May miss some features

## Temporary Workaround

For now, we've created a minimal demo version (6 components only) that works around these issues. To enable the full gallery:

1. GPUI needs to support WASM properly
2. Or we need to refactor story to have optional dependencies

## Next Steps

### Short Term
1. Document current status (this file)
2. Create issue in GPUI repository about WASM support
3. Monitor GPUI Web examples for updates

### Medium Term
1. Implement feature flags in gpui-component
2. Create WASM-specific story variants
3. Test with minimal component set

### Long Term
1. Full WASM support when GPUI Web matures
2. All 60+ components working in browser
3. Deploy to GitHub Pages / Netlify

## Testing Strategy

Once compilation issues are resolved:

```bash
# Build WASM
cd crates/story-web
make build-wasm

# Generate JavaScript bindings
wasm-bindgen target/wasm32-unknown-unknown/release/*.wasm \
  --out-dir www/src/wasm \
  --target web

# Start dev server
cd www
bun run dev
```

## Resources

- [GPUI Web Examples](https://github.com/zed-industries/zed/tree/main/crates/gpui_web/examples)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [Rust WASM Book](https://rustwasm.github.io/book/)

## Contributing

If you have experience with WASM and want to help:

1. Try alternative approaches to avoid problematic dependencies
2. Contribute to GPUI Web support
3. Test with different GPUI versions
4. Document workarounds that work

## Conclusion

The infrastructure for GPUI Component Story Web is complete. The remaining blocker is WASM compatibility in the dependency chain. This will likely be resolved as GPUI Web matures.

**Status**: 🟡 **Waiting for Dependency Support**

Last Updated: 2024-02-27

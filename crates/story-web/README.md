# GPUI Component Story Web

Web version of the GPUI Component Story Gallery, compiled to WASM to run in browsers.

> ⚠️ **Status**: Currently blocked by WASM compatibility issues in dependencies (psm, aws-lc-sys, errno). See [STATUS.md](./STATUS.md) for details. Infrastructure is complete and ready once upstream dependencies support WASM.

## Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- [Bun](https://bun.sh/) (recommended) or Node.js
- wasm-bindgen-cli

### Install Dependencies

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Install Bun (macOS/Linux)
curl -fsSL https://bun.sh/install | bash

# Or use npm
npm install -g bun
```

## Development

### 1. Build WASM

```bash
# In story-web directory
cargo build --target wasm32-unknown-unknown --release

# Generate JavaScript bindings
wasm-bindgen ../../../target/wasm32-unknown-unknown/release/gpui_component_story_web.wasm \
  --out-dir ./www/src/wasm \
  --target web \
  --no-typescript
```

Or use the provided script:

```bash
./scripts/build-wasm.sh
```

### 2. Start Development Server

```bash
cd www
bun install
bun run dev
```

The browser will automatically open at http://localhost:3000

## Build for Production

```bash
# Build WASM (optimized)
./scripts/build-wasm.sh --release

# Build frontend
cd www
bun run build
```

Built files will be output to the `www/dist` directory.

## Project Structure

```
story-web/
├── Cargo.toml          # Rust WASM project config
├── src/
│   └── lib.rs          # WASM entry and Gallery implementation
├── www/                # Frontend project
│   ├── package.json    # Bun/Node.js dependencies
│   ├── vite.config.js  # Vite configuration
│   ├── index.html      # HTML entry
│   ├── src/
│   │   ├── main.js     # JavaScript entry
│   │   └── wasm/       # Generated WASM bindings
│   └── dist/           # Build output
└── scripts/            # Build scripts
```

## Deployment

After building, deploy the `www/dist` directory to any static file server:

- Netlify
- Vercel
- GitHub Pages
- Cloudflare Pages
- Or any CDN

## Notes

1. WASM files are large; initial load may take some time
2. Some features may be limited in web environment (e.g., file system access)
3. Recommended browsers: Chrome 90+, Firefox 88+, Safari 15+

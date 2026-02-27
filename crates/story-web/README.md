# GPUI Component Story Web

## Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- [Bun](https://bun.sh) (recommended) or Node.js
- wasm-bindgen-cli

### Install Dependencies

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Install Bun (macOS/Linux)
curl -fsSL https://bun.sh/install | bash
```

### Start Development Server

```bash
make dev
```

#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building GPUI Component Story Web...${NC}"

# Get the script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/.."

# Parse arguments
RELEASE_FLAG=""
if [[ "$1" == "--release" ]]; then
    RELEASE_FLAG="--release"
    echo -e "${YELLOW}Building in release mode${NC}"
fi

# Step 1: Build WASM
echo -e "\n${GREEN}Step 1: Building WASM...${NC}"
cd "$PROJECT_ROOT"
cargo build --target wasm32-unknown-unknown $RELEASE_FLAG

# Determine the build directory
if [[ "$RELEASE_FLAG" == "--release" ]]; then
    WASM_PATH="../../../target/wasm32-unknown-unknown/release/gpui_component_story_web.wasm"
else
    WASM_PATH="../../../target/wasm32-unknown-unknown/debug/gpui_component_story_web.wasm"
fi

# Step 2: Generate JavaScript bindings
echo -e "\n${GREEN}Step 2: Generating JavaScript bindings...${NC}"
wasm-bindgen "$WASM_PATH" \
    --out-dir "$PROJECT_ROOT/www/src/wasm" \
    --target web \
    --no-typescript

echo -e "\n${GREEN}✓ Build completed successfully!${NC}"
echo -e "${YELLOW}Next steps:${NC}"
echo -e "  cd www"
echo -e "  bun install"
echo -e "  bun run dev"

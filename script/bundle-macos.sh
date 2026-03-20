#!/bin/bash
set -euo pipefail

APP_NAME="OnetCli"
BINARY_NAME="onetcli"
TARGET="${1:-aarch64-apple-darwin}"
VERSION="${ONETCLI_VERSION:-0.1.0}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="${PROJECT_DIR}/target/${APP_NAME}.app"

echo "Bundling ${APP_NAME}.app for ${TARGET} (version: ${VERSION})..."

# Clean previous bundle
rm -rf "$APP_DIR"

# Create .app directory structure
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
BINARY_PATH="${PROJECT_DIR}/target/${TARGET}/release/${BINARY_NAME}"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at ${BINARY_PATH}"
    echo "Run: cargo build --release -p main --target ${TARGET}"
    exit 1
fi
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/${BINARY_NAME}"

# Copy Info.plist and substitute version
sed "s/\${ONETCLI_VERSION}/${VERSION}/g" \
    "${PROJECT_DIR}/resources/macos/Info.plist" \
    > "$APP_DIR/Contents/Info.plist"

# Regenerate macOS icon from logo.svg before bundling to avoid stale .icns assets.
bash "${PROJECT_DIR}/script/generate-macos-icon.sh"

# Copy icon
ICNS_PATH="${PROJECT_DIR}/resources/macos/OnetCli.icns"
if [ -f "$ICNS_PATH" ]; then
    cp "$ICNS_PATH" "$APP_DIR/Contents/Resources/OnetCli.icns"
else
    echo "Warning: Icon file not found at ${ICNS_PATH}"
fi

# Write PkgInfo
echo -n "APPL????" > "$APP_DIR/Contents/PkgInfo"

echo "Successfully built: ${APP_DIR}"
echo "Contents:"
ls -la "$APP_DIR/Contents/"
ls -la "$APP_DIR/Contents/MacOS/"
ls -la "$APP_DIR/Contents/Resources/"

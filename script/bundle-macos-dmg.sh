#!/bin/bash
set -euo pipefail

APP_NAME="OnetCli"
TARGET="${1:-aarch64-apple-darwin}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="${PROJECT_DIR}/target/${APP_NAME}.app"
TMP_DIR="${PROJECT_DIR}/target/dmg"
DMG_NAME="onetcli-${TARGET}.dmg"
DMG_PATH="${PROJECT_DIR}/${DMG_NAME}"

if [ ! -d "$APP_DIR" ]; then
    echo "Error: App bundle not found at ${APP_DIR}"
    echo "Run: script/bundle-macos.sh ${TARGET}"
    exit 1
fi

rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"
cp -R "$APP_DIR" "$TMP_DIR/${APP_NAME}.app"
ln -s /Applications "$TMP_DIR/Applications"

# 生成可分发的压缩 DMG（UDZO）
rm -f "$DMG_PATH"
hdiutil create \
    -volname "${APP_NAME}" \
    -srcfolder "$TMP_DIR" \
    -ov \
    -size 200m \
    -format UDZO \
    "$DMG_PATH"

# 可选：如果提供签名身份，则对 DMG 执行签名
if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
    echo "Signing DMG with identity: ${MACOS_SIGN_IDENTITY}"
    codesign --force --sign "${MACOS_SIGN_IDENTITY}" "$DMG_PATH"
fi

echo "Successfully built DMG: ${DMG_PATH}"
ls -lh "$DMG_PATH"

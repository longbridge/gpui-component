---
title: Packaging
description: 把脚本应用发布成桌面应用——预编译的 gpui-shell 二进制加上应用目录，做成 macOS .app、Linux 压缩包或 Windows 文件夹。
order: 16
maturity: [preview]
---

# Packaging

一个打包好的应用由两样东西并排组成：预编译的 `gpui-shell` 二进制，以及应用目录。整个过程不需要编译。二进制在不带参数启动时，会在自己旁边寻找应用；Finder、`.desktop` 启动项、开始菜单快捷方式都是这样启动程序的，它们都没法传路径。

| 平台    | 可执行文件位置                     | 应用目录位置                         |
| ------- | ---------------------------------- | ------------------------------------ |
| macOS   | `Clock.app/Contents/MacOS/`        | `Clock.app/Contents/Resources/app/`  |
| Linux   | 包目录                             | 可执行文件旁边的 `app/`              |
| Windows | 包目录                             | 可执行文件旁边的 `app\`              |

可执行文件可以改名，比如把 `gpui-shell` 改成 `Clock`：应用是按可执行文件所在的位置找到的，和它叫什么名字无关。

## 打包之前

应用需要一个带稳定 `id` 的 manifest。打包后的应用按这个 `id` 保存数据，而不是按路径，所以移动 `.app` 或安装更新都不会丢失用户的数据。

```json
{
  "id": "com.example.clock",
  "name": "Clock",
  "version": "1.0.0",
  "shell-version": "0.7.3",
  "entry": "main.js",
  "capabilities": { "storage": true }
}
```

其中的 `capabilities` 会按原样授予，所以只申请应用真正用到的部分，见 [Capabilities](./capabilities.md)。打包前先运行 `gpui-shell check -d clock`，并去掉只在开发时需要的文件：`gpui-kit.d.ts`、`jsconfig.json`、`node_modules/` 和 `AGENTS.md`。

`gpui-shell new clock` 会按这个结构创建一个起步目录。

## 获取二进制

每个 GitHub Release 都为每个平台附带一个压缩包，旁边附有 `.sha256`：

| 目标平台              | 产物                                         |
| --------------------- | -------------------------------------------- |
| macOS，Apple 芯片     | `gpui-shell-aarch64-apple-darwin.tar.gz`     |
| macOS，Intel          | `gpui-shell-x86_64-apple-darwin.tar.gz`      |
| Linux，x86-64         | `gpui-shell-x86_64-unknown-linux-gnu.tar.gz` |
| Windows，x86-64       | `gpui-shell-x86_64-pc-windows-msvc.zip`      |

```bash
curl -fsSLO https://github.com/longbridge/gpui-kit/releases/latest/download/gpui-shell-aarch64-apple-darwin.tar.gz
tar -xzf gpui-shell-aarch64-apple-darwin.tar.gz
```

在构建脚本里要固定版本时，把 `latest/download` 换成 `download/v0.7.3`。如果想自己构建，在仓库里运行 `cargo build -p gpui-shell --release`，产物是 `target/release/gpui-shell`。

## macOS

```text
Clock.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── Clock              # 改名后的 gpui-shell 二进制
    └── Resources/
        ├── Clock.icns
        └── app/
            ├── gpui-shell.json
            └── main.js
```

`Info.plist` 指定可执行文件和图标。bundle identifier 用 manifest 的 `id`，让两者保持同一个身份：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>Clock</string>
  <key>CFBundleIdentifier</key>
  <string>com.example.clock</string>
  <key>CFBundleName</key>
  <string>Clock</string>
  <key>CFBundleIconFile</key>
  <string>Clock</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
```

组装只需要几次复制：

```bash
set -e
app=Clock.app
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp Info.plist "$app/Contents/"
cp gpui-shell-aarch64-apple-darwin/gpui-shell "$app/Contents/MacOS/Clock"
cp Clock.icns "$app/Contents/Resources/"
rsync -a --exclude node_modules --exclude gpui-kit.d.ts --exclude jsconfig.json \
  --exclude AGENTS.md clock/ "$app/Contents/Resources/app/"
```

`iconutil -c icns Clock.iconset` 可以把一组 PNG 生成 `Clock.icns`。要做一个同时支持 Apple 芯片和 Intel 的 bundle，先把两个 release 二进制合并：

```bash
lipo -create -output Clock \
  gpui-shell-aarch64-apple-darwin/gpui-shell \
  gpui-shell-x86_64-apple-darwin/gpui-shell
```

要分发给别人的 bundle 必须签名并公证，否则 Gatekeeper 会拒绝打开。签名要覆盖 bundle 里的全部内容，脚本也是其中一部分：

```bash
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: Example Inc (TEAMID)" Clock.app
ditto -c -k --keepParent Clock.app Clock.zip
xcrun notarytool submit Clock.zip --keychain-profile notary --wait
xcrun stapler staple Clock.app
```

签名也会封存脚本，签名之后再改动 `Resources/app` 会让签名失效，所以签名放在最后一步。

## Linux

```text
clock/
├── clock                     # 改名后的 gpui-shell 二进制
├── clock.png
└── app/
    ├── gpui-shell.json
    └── main.js
```

把这个目录打成压缩包发布：

```bash
mkdir -p clock/app
cp gpui-shell-x86_64-unknown-linux-gnu/gpui-shell clock/clock
cp -r my-clock-source/* clock/app/
tar -czf clock-1.0.0-x86_64.tar.gz clock
```

再加一个 desktop entry，它就会出现在启动器里。把它装成 `~/.local/share/applications/com.example.clock.desktop`（所有用户可用则放在 `/usr/share/applications`），`Exec` 指向解压后的可执行文件：

```ini
[Desktop Entry]
Type=Application
Name=Clock
Exec=/opt/clock/clock
Icon=/opt/clock/clock.png
Categories=Utility;
StartupWMClass=clock
```

同样的目录结构可以原样放进 `.deb`、`.rpm` 或 AppImage，关键是 `app/` 要留在可执行文件旁边。运行时需要的依赖和任何 GPUI 应用一样：Vulkan 驱动、`libxkbcommon`，以及 Wayland 或 X11 会话。

## Windows

```text
Clock\
├── Clock.exe                 # 改名后的 gpui-shell.exe
└── app\
    ├── gpui-shell.json
    └── main.js
```

把文件夹打成 zip，或交给 Inno Setup、WiX 这类安装器；开始菜单快捷方式指向 `Clock.exe`，不带参数。

## 相关

| 页面 | 内容 |
| --- | --- |
| [快速开始](./getting-started.md) | 安装运行时，`run`、`new`、`check` 与已安装的应用 |
| [Capabilities](./capabilities.md) | manifest 能申请什么，打包后的应用会被授予什么 |
| [依赖](./dependencies.md) | Git 依赖，首次启动时抓取到 `~/.gpui-shell/cache` |

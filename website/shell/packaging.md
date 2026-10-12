---
title: Packaging
description: Ship a script application as a desktop application — the prebuilt gpui-shell binary plus the application's directory, as a macOS .app, a Linux archive or a Windows folder.
order: 16
maturity: [preview]
---

# Packaging

A packaged application is two things side by side: the prebuilt `gpui-shell` binary and the application's directory. Nothing is compiled. The binary looks for an application next to itself when it is started with no arguments, which is how a desktop launcher — Finder, a `.desktop` entry, a Start menu shortcut — starts it, since none of them can pass a path.

| Platform | Where the executable is            | Where the application goes   |
| -------- | ---------------------------------- | ---------------------------- |
| macOS    | `Clock.app/Contents/MacOS/`        | `Clock.app/Contents/Resources/app/` |
| Linux    | the package directory              | `app/` beside the executable |
| Windows  | the package directory              | `app\` beside the executable |

The executable may be renamed — `Clock` instead of `gpui-shell` — because the application is found by where the executable is, not by what it is called.

## Before packaging

The application needs a manifest with a stable `id`, because a packaged application keeps its storage under that `id` rather than under its path: moving the `.app` or installing an update does not lose the user's data.

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

Its `capabilities` are granted as written, so ask only for what the application uses — see [Capabilities](./capabilities.md). Run `gpui-shell check -d clock` before packaging, and leave out what only development needs: `gpui-kit.d.ts`, `jsconfig.json`, `node_modules/` and `AGENTS.md`.

`gpui-shell new clock` creates a directory in this shape to start from.

## Getting the binary

Each GitHub release carries one archive per platform, with a `.sha256` beside it:

| Target                      | Asset                                        |
| --------------------------- | -------------------------------------------- |
| macOS, Apple silicon        | `gpui-shell-aarch64-apple-darwin.tar.gz`     |
| macOS, Intel                | `gpui-shell-x86_64-apple-darwin.tar.gz`      |
| Linux, x86-64               | `gpui-shell-x86_64-unknown-linux-gnu.tar.gz` |
| Windows, x86-64             | `gpui-shell-x86_64-pc-windows-msvc.zip`      |

```bash
curl -fsSLO https://github.com/longbridge/gpui-kit/releases/latest/download/gpui-shell-aarch64-apple-darwin.tar.gz
tar -xzf gpui-shell-aarch64-apple-darwin.tar.gz
```

Pin a version in a build script by replacing `latest/download` with `download/v0.7.3`. To build it yourself instead, run `cargo build -p gpui-shell --release` in the repository; the binary is `target/release/gpui-shell`.

## macOS

```text
Clock.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── Clock              # the gpui-shell binary, renamed
    └── Resources/
        ├── Clock.icns
        └── app/
            ├── gpui-shell.json
            └── main.js
```

`Info.plist` names the executable and the icon. Use the manifest's `id` as the bundle identifier so the two stay one identity:

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

Assembling it is a few copies:

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

`iconutil -c icns Clock.iconset` makes `Clock.icns` from a folder of PNGs. For one bundle that runs on both Apple silicon and Intel, merge the two release binaries first:

```bash
lipo -create -output Clock \
  gpui-shell-aarch64-apple-darwin/gpui-shell \
  gpui-shell-x86_64-apple-darwin/gpui-shell
```

A bundle given to other people has to be signed and notarized, or Gatekeeper refuses to open it. Sign everything the bundle contains — the scripts are part of it:

```bash
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: Example Inc (TEAMID)" Clock.app
ditto -c -k --keepParent Clock.app Clock.zip
xcrun notarytool submit Clock.zip --keychain-profile notary --wait
xcrun stapler staple Clock.app
```

Signing seals the scripts too, so any change to `Resources/app` after signing breaks the signature: sign last.

## Linux

```text
clock/
├── clock                     # the gpui-shell binary, renamed
├── clock.png
└── app/
    ├── gpui-shell.json
    └── main.js
```

Ship the directory as an archive:

```bash
mkdir -p clock/app
cp gpui-shell-x86_64-unknown-linux-gnu/gpui-shell clock/clock
cp -r my-clock-source/* clock/app/
tar -czf clock-1.0.0-x86_64.tar.gz clock
```

A desktop entry makes it a launcher item. Install it as `~/.local/share/applications/com.example.clock.desktop` (or under `/usr/share/applications` for every user), with `Exec` pointing at the unpacked executable:

```ini
[Desktop Entry]
Type=Application
Name=Clock
Exec=/opt/clock/clock
Icon=/opt/clock/clock.png
Categories=Utility;
StartupWMClass=clock
```

The same layout goes into a `.deb`, an `.rpm` or an AppImage unchanged: what matters is that `app/` stays beside the executable. The binary needs what any GPUI application needs at run time — a Vulkan driver, `libxkbcommon`, and a Wayland or X11 session.

## Windows

```text
Clock\
├── Clock.exe                 # gpui-shell.exe, renamed
└── app\
    ├── gpui-shell.json
    └── main.js
```

Zip the folder, or hand it to an installer such as Inno Setup or WiX; a Start menu shortcut targets `Clock.exe` with no arguments.

## Related

| Page | What it covers |
| --- | --- |
| [Getting Started](./getting-started.md) | Installing the runtime, `run`, `new`, `check` and installed applications |
| [Capabilities](./capabilities.md) | What a manifest may ask for, and what a packaged application is granted |
| [Dependencies](./dependencies.md) | Git dependencies, fetched on first launch into `~/.gpui-shell/cache` |

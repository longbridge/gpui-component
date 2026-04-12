---
title: 安装
order: -1
---

# 安装

在开始使用 `gpui-component` 构建应用之前，需要先准备对应的开发环境并安装依赖。

## 系统要求

目前可以在 macOS、Windows 和 Linux 上进行开发。

### macOS

- macOS 15 或更高版本
- Xcode Command Line Tools

## Windows

- Windows 10 或更高版本

仓库提供了一个脚本用于安装所需工具链和依赖。可以在 PowerShell 中运行：

```ps
.\script\install-window.ps1
```

## Linux

在 Linux 上，可以运行下面的脚本安装系统依赖：

```bash
./script/bootstrap
```

## Rust 和 Cargo

`gpui-component` 使用 Rust 构建，因此请确保系统已经安装 Rust 和 Cargo。

- Rust 1.90 或更高版本
- Cargo（通常随 Rust 一起安装）

安装库时，只需要在 `Cargo.toml` 的 `[dependencies]` 中加入：

```toml
gpui = { git = "https://github.com/zed-industries/zed" }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
```

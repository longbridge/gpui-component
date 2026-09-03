---
title: Installation
order: -1
---

# Installation

Before you start to build your application with `gpui-component`, you need to install the library.

## System Requirements

We can development application on macOS, Windows or Linux.

### macOS

- macOS 15 or later
- Xcode command line tools

## Windows

- Windows 10 or later

There have a bootstrap script to help install the required toolchain and dependencies.

You can run the script in PowerShell:

```ps
.\script\install-window.ps1
```

## Linux

Run `./script/bootstrap` to install system dependencies.

## Rust and Cargo

We use Rust programming language to build the `gpui-component` library. Make sure you have Rust and Cargo installed on your system.

- Rust 1.90 or later
- Cargo (comes with Rust)

To install the `gpui-component` library, you can use Cargo, the Rust package manager. Add the following line to your `Cargo.toml` file under the `[dependencies]` section:

```toml
gpui-kit = "0.6"
```

`gpui-kit` depends on the matching GPUI crates for you and re-exports them as `gpui_kit::gpui`, `gpui_kit::platform` and `gpui_kit::component`. The rest of these docs write `gpui::…` and `gpui_component::…` paths; they resolve through `use gpui_kit::prelude::*;`, which brings those crate names into scope.

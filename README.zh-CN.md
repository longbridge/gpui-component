# GPUI Component

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/longbridge/gpui-component/actions/workflows/ci.yml/badge.svg)](https://github.com/longbridge/gpui-component/actions/workflows/ci.yml) [![Docs](https://docs.rs/gpui-component/badge.svg)](https://docs.rs/gpui-component/) [![Crates.io](https://img.shields.io/crates/v/gpui-component.svg)](https://crates.io/crates/gpui-component)

使用 Rust 和 [GPUI](https://gpui.rs) 构建出色、高性能的桌面应用。

GPUI Component 是一个综合性的 Rust 桌面应用开发框架。它将生产级 UI
系统、应用级数据与布局能力、编辑能力，以及可复用的行为、状态和基础设施整合在一起。

## 特性

- **60+ 组件**：覆盖表单、导航、浮层、反馈和布局等场景，提供成熟交互与高效默认值。
- **生产就绪**：从第一天起用于构建 Longbridge Pro，并在公开发布的商业桌面应用中持续打磨。
- **原生体验**：现代控件设计灵感来自 macOS 与 Windows，并提供语义化主题和多种尺寸。
- **120 FPS**：GPU 加速界面，在高负载下依然保持流畅。
- **数据表格**：虚拟滚动、固定列、列宽调整、排序与单元格选择，可承载数十万行数据。
- **虚拟列表**：只渲染可见区域，并支持不同尺寸的列表项。
- **代码编辑器**：20 万行规模下仍保持稳定，集成 Tree-sitter 高亮与 LSP 诊断、补全和悬浮提示。
- **Dock 布局**：可调整面板、可拖拽标签、嵌套分割、边缘停靠，以及可序列化的 Tiles 自由布局。
- **丰富内容**：原生 Markdown 与 HTML 渲染、语法高亮和内置图表。
- **设计自由**：使用完整视觉系统，或基于 `gpui-base` 的行为与基础设施构建自己的系统。
- **跨平台**：通过一份 Rust 代码交付 macOS、Windows 和 Linux。

## 框架架构

### 两层架构，一个生态

使用 `gpui-component`，让整个应用保持统一、完整的视觉与交互风格；当产品需要创建并拥有自己的设计系统时，使用 `gpui-base`。

| **`gpui-component`**     | **`gpui-base`**            |
| ------------------------ | -------------------------- |
| 完整且带样式的组件       | 无预设样式的行为与基础设施 |
| 开箱即用，并支持主题定制 | 完全掌控结构与视觉设计     |
| 适合直接构建应用         | 适合构建设计系统           |

```text
                             APPLICATION
                                  │
                ┌─────────────────┴─────────────────┐
                │                                   │
                ▼                                   ▼
       ┌──────────────────┐               ┌──────────────────┐
       │  gpui-component  │               │ Your Design      │
       │    Styled UI     │               │ System           │
       └────────┬─────────┘               └────────┬─────────┘
                │                                  │
                └────────────────┬─────────────────┘
                                 ▼
                       ┌──────────────────┐
                       │    gpui-base     │
                       │ Behavior · State │
                       │ Infrastructure   │
                       └────────┬─────────┘
                                ▼
                              GPUI
```

> **行为属于基础层，呈现属于应用。**

如果希望使用精致、开箱即用且风格统一的控件，请选择 **`gpui-component`**。如果应用需要拥有组件源码、布局、样式和动效，同时复用复杂且可靠的交互行为，请直接构建于 **`gpui-base`**。

这种分层方式与 [shadcn](https://ui.shadcn.com) 生态的灵活性来源一致：

| GPUI Component 生态                  | Web 生态                       |
| ------------------------------------ | ------------------------------ |
| [GPUI](https://gpui.rs)              | HTML + Tailwind CSS            |
| [`gpui-base`](crates/base/README.md) | [Base UI](https://base-ui.com) |
| `gpui-component`                     | shadcn 的完整样式组件层        |

[深入了解架构 →](docs/ARCHITECTURE.md)

## Showcase

GPUI Component 从第一天起就用于构建 [Longbridge Pro](https://longbridge.com/desktop)。
这个框架不是脱离应用场景凭空设计出来的，而是从一款公开发布的商业桌面应用中持续提炼而成。

> **GPUI 为渲染打下基础，Longbridge 为生产实践打下基础。**

<img width="1763" alt="Image" src="https://github.com/user-attachments/assets/e1ecb9c3-2dd3-431e-bd97-5a819c30e551" />

## Usage

```toml
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
```

### 基础示例

```rs
use gpui::*;
use gpui_component::{button::*, *};

pub struct HelloWorld;
impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    gpui_platform::application().run(move |cx| {
        // 使用任何 GPUI Component 功能之前必须先调用此函数。
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // 窗口的第一层应该是一个 Root。
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

### 图标

GPUI Component 提供了 `Icon` 元素，但默认不包含 SVG 文件。

示例使用 [Lucide](https://lucide.dev) 图标，但你可以使用任意喜欢的图标。只需按照 [IconName](https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/icon.rs#L86) 中的定义命名 SVG 文件，然后将所需图标添加到项目中即可。

## AI 编码 Agent 技能 (Skills)

为你的 AI 编码助手（Cursor, Claude Code, Gemini CLI, Codex 等）安装 GPUI Component 技能库：

```bash
npx skills add longbridge/gpui-component
```

| 技能 | 描述 |
| --- | --- |
| `gpui-component` | 完整组件目录、常用使用模式与组件编写规范。 |
| `gpui` | GPUI 底层框架机制（Element 渲染、Entity 状态、异步、焦点、Actions、测试）。 |

## Development

### 桌面 Gallery（Story）

`story` crate 是一个展示所有可用组件的画廊应用程序，通过以下命令运行：

```bash
cargo run
```

### Examples

一些重要的示例内置在 `story` crate 中，可以直接运行：

```bash
# 支持 LSP 和语法高亮的代码编辑器
cargo run --example editor

# Dock 布局系统（面板、分割视图、标签页）
cargo run --example dock

# Markdown 渲染
cargo run --example markdown

# HTML 渲染
cargo run --example html
```

`examples` 目录还包含独立示例，每个示例专注于单一功能。每个示例是一个独立的 crate，使用 `cargo run -p <name>` 运行：

```bash
# 基础 Hello World
cargo run -p hello_world

# 系统监控器（实时 CPU/内存数据图表）
cargo run -p system_monitor

# 窗口标题自定义
cargo run -p window_title
```

查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解更多详情。

## 与其他框架对比

| 特性                | GPUI Component       | [Iced]             | [egui]                | [Qt 6]                                            |
| ------------------- | -------------------- | ------------------ | --------------------- | ------------------------------------------------- |
| 语言                | Rust                 | Rust               | Rust                  | C++/QML                                           |
| 核心                | GPUI                 | wgpu               | wgpu                  | QT                                                |
| 许可证              | Apache 2.0           | MIT                | MIT/Apache 2.0        | [Commercial/LGPL](https://www.qt.io/qt-licensing) |
| 最小二进制大小 [^1] | 12MB                 | 11MB               | 5M                    | 20MB [^2]                                         |
| 跨平台              | 是                   | 是                 | 是                    | 是                                                |
| 文档                | 一般                 | 一般               | 一般                  | 良好                                              |
| Web 支持            | 是（WASM）           | 是                 | 是                    | 是                                                |
| UI 风格             | 现代                 | 基础               | 基础                  | 基础                                              |
| CJK 支持            | 是                   | 是                 | 差                    | 是                                                |
| Chart               | 是                   | 否                 | 否                    | 是                                                |
| Table（大数据集）   | 是<br>（虚拟行、列） | 否                 | 是<br>（虚拟行）      | 是<br>（虚拟行、列）                              |
| Table 列宽调整      | 是                   | 否                 | 是                    | 是                                                |
| 文本基础            | Rope                 | [COSMIC Text] [^3] | trait TextBuffer [^4] | [QTextDocument]                                   |
| Code Editor         | 简单                 | 简单               | 简单                  | 基础 API                                          |
| Dock 布局           | 是                   | 是                 | 是                    | 是                                                |
| 语法高亮            | [Tree Sitter]        | [Syntect]          | [Syntect]             | [QSyntaxHighlighter]                              |
| Markdown 渲染       | 是                   | 是                 | 基础                  | 否                                                |
| Markdown 混合 HTML  | 是                   | 否                 | 否                    | 否                                                |
| HTML 渲染           | 基础                 | 否                 | 否                    | 基础                                              |
| 文本选择            | TextView             | 否                 | 任意 Label            | 是                                                |
| 自定义主题          | 是                   | 是                 | 是                    | 是                                                |
| 内置主题            | 是                   | 否                 | 否                    | 否                                                |
| 国际化              | 是                   | 是                 | 是                    | 是                                                |

> 如发现任何错误或过时信息，请提交 issue 或 PR。

[Iced]: https://github.com/iced-rs/iced
[egui]: https://github.com/emilk/egui
[QT 6]: https://www.qt.io/product/qt6
[Tree Sitter]: https://tree-sitter.github.io/tree-sitter/
[Syntect]: https://github.com/trishume/syntect
[QSyntaxHighlighter]: https://doc.qt.io/qt-6/qsyntaxhighlighter.html
[QTextDocument]: https://doc.qt.io/qt-6/qtextdocument.html
[COSMIC Text]: https://github.com/pop-os/cosmic-text

[^1]: 使用简单 Hello World 示例的 Release 构建。

[^2]: [减小 Qt 应用程序的二进制大小](https://www.qt.io/blog/reducing-binary-size-of-qt-applications-part-3-more-platforms)

[^3]: Iced Editor: <https://github.com/iced-rs/iced/blob/db5a1f6353b9f8520c4f9633d1cdc90242c2afe1/graphics/src/text/editor.rs#L65-L68>

[^4]: egui TextBuffer: <https://github.com/emilk/egui/blob/0a81372cfd3a4deda640acdecbbaf24bf78bb6a2/crates/egui/src/widgets/text_edit/text_buffer.rs#L20>

## 许可证

Apache-2.0

- UI 设计基于 [shadcn/ui](https://ui.shadcn.com)，部分来自 [Reui](https://reui.io)。
- 图标来自 [Lucide](https://lucide.dev)。

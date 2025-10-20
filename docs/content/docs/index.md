---
title: Introduction
description: Rust GUI components for building fantastic cross-platform desktop application by using GPUI.
---

# Welcome to GPUI Component

GPUI Component is a Rust UI component library for building fantastic desktop applications using [GPUI](https://gpui.rs).

## Getting Started

New to GPUI Component? Start here:

- [Getting Started](/getting-started) - Installation, setup, and your first component

## Components

### Basic Components

- [Accordion](/components/accordion) - Collapsible content panels
- [Alert](/components/alert) - Alert messages with different variants
- [Avatar](/components/avatar) - User avatars with fallback text
- [Badge](/components/badge) - Count badges and indicators
- [Button](/components/button) - Interactive buttons with multiple variants
- [Checkbox](/components/checkbox) - Binary selection control
- [Icon](/components/icon) - Icon display component
- [Image](/components/image) - Image display with fallbacks
- [Indicator](/components/indicator) - Loading and status indicators
- [Kbd](/components/kbd) - Keyboard shortcut display
- [Label](/components/label) - Text labels for form elements
- [Progress](/components/progress) - Progress bars
- [Radio](/components/radio) - Single selection from multiple options
- [Skeleton](/components/skeleton) - Loading placeholders
- [Slider](/components/slider) - Value selection from a range
- [Switch](/components/switch) - Toggle on/off control
- [Tag](/components/tag) - Labels and categories
- [Toggle](/components/toggle) - Toggle button states
- [Tooltip](/components/tooltip) - Helpful hints on hover

### Form Components

- [ColorPicker](/components/color-picker) - Color selection interface
- [DatePicker](/components/date-picker) - Date selection with calendar
- [Dropdown](/components/dropdown) - Select from dropdown options
- [Form](/components/form) - Form container and layout
- [Input](/components/input) - Text input with validation
- [NumberInput](/components/number-input) - Numeric input with increment/decrement
- [OtpInput](/components/otp-input) - One-time password input
- [Textarea](/components/textarea) - Multi-line text input

### Layout Components

- [DescriptionList](/components/description-list) - Key-value pair display
- [Drawer](/components/drawer) - Slide-in panel from edges
- [GroupBox](/components/group-box) - Grouped content with borders
- [Modal](/components/modal) - Dialog and modal windows
- [Notification](/components/notification) - Toast notifications
- [Popover](/components/popover) - Floating content display
- [Resizable](/components/resizable) - Resizable panels and containers
- [Scrollable](/components/scrollable) - Scrollable containers
- [Sidebar](/components/sidebar) - Navigation sidebar

### Advanced Components

- [Calendar](/components/calendar) - Calendar display and navigation
- [Chart](/components/chart) - Data visualization charts (Line, Bar, Area, Pie)
- [List](/components/list) - List display with items
- [Menu](/components/menu) - Menu and context menu
- [Table](/components/table) - High-performance data tables
- [Tabs](/components/tabs) - Tabbed interface
- [Tree](/components/tree) - Hierarchical tree data display
- [VirtualList](/components/virtual-list) - Virtualized list for large datasets
- [WebView](/components/webview) - Embedded web browser

### Utility Components

- [Clipboard](/components/clipboard) - Clipboard operations
- [TitleBar](/components/title-bar) - Custom window title bar

## Features

### Richness

40+ cross-platform desktop UI components for building comprehensive applications.

### Native

Inspired by macOS and Windows controls, combined with modern shadcn/ui design for a native experience.

### Ease of Use

Stateless `RenderOnce` components that are simple and user-friendly, following GPUI's design principles and Fluent API.

### Customizable

Built-in `Theme` and `ThemeColor` supporting multi-theme and variable-based configurations, and with built-in [20+ themes](https://github.com/longbridge/gpui-component/tree/main/themes).

### 📏 Versatile

Supports sizes like `xs`, `sm`, `md`, and `lg` across components.

### Flexible Layout

Dock layout for panel arrangements, resizing, and freeform (Tiles) layouts.

### High Performance

Virtualized Table and List components for smooth rendering of large datasets.

### Content Rendering

Native support for Markdown and simple HTML rendering.

### Charting

Built-in charts for data visualization.

### Editor

High-performance code editor with LSP support (diagnostics, completion, hover).

### Syntax Highlighting

Powered by Tree Sitter for accurate syntax highlighting.

## Quick Example

Add `gpui` and `gpui-component` to your `Cargo.toml`:

```toml
[dependencies]
gpui = "0.2.0"
gpui-component = "0.2.0"
```

Then create a simple "Hello, World!" application with a button:

```rust
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
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

## Community & Support

- [GitHub Repository](https://github.com/longbridge/gpui-component)
- [Issue Tracker](https://github.com/longbridge/gpui-component/issues)
- [Contributing Guide](https://github.com/longbridge/gpui-component/blob/main/CONTRIBUTING.md)

## License

Apache-2.0

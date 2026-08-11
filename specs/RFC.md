# RFC: gpui-component Open-Code Architecture

- **Status:** Draft
- **Project:** GPUI Components
- **Affected crates:** `gpui-component`, `gpui-component-base`
- **Goal:** Application-owned UI with reusable behavior and infrastructure

---

## 1. Summary

`gpui-component` 当前采用传统 Component Library 模式：

```text
gpui-component
      │
      ├── Button
      ├── Input
      ├── Dialog
      ├── Table
      └── ...
              │
              ▼
          Application
```

组件的：

- Behavior
- Structure
- Layout
- Style

基本都由 Library 拥有。

Application 主要通过：

- Theme
- Component Properties
- `Styled`
- Builder APIs

进行有限定制。

这种模式适合快速开发，但随着使用场景增加，会产生两个长期问题：

1. 为满足不同设计需求，组件需要不断增加配置 API。
2. Application 很难建立与 gpui-component 默认视觉完全不同的 Design System。

本 RFC 提议引入新的架构：

```text
GPUI
 │
 ▼
gpui-component-base
 │
 │ Behavior / State / Interaction
 │ Infrastructure
 │
 ▼
Application-owned Components
 │
 │ Structure / Layout / Style
 │
 ▼
Application
```

核心原则：

> **Behavior belongs to the framework. Style belongs to the application.**

最终组件源码通过 Registry + CLI 分发给 Application。

---

# 2. Goals

本 RFC 的目标：

### 2.1 Application owns UI

最终的：

```rust
Button
Input
Dialog
Select
Sidebar
```

源码可以直接存在于：

```text
src/ui/
```

Application 可以自由修改。

---

### 2.2 gpui-component-base owns reusable behavior

复杂且通用的：

- State
- Interaction
- Focus
- Keyboard Navigation
- Overlay
- Positioning
- Accessibility
- Virtualization

仍然由 Library 维护。

避免 Application Copy 大量复杂基础实现。

---

### 2.3 Preserve existing gpui-component API

现有：

```rust
use gpui_component::button::Button;
```

继续工作。

本次重构不要求现有 Application 迁移。

---

### 2.4 Avoid configuration-driven styling

不通过不断增加：

```rust
button_background
button_hover_background
button_border
button_radius
button_padding
```

解决定制问题。

组件 Style 属于组件源码，而不是 Theme Configuration。

---

# 3. Non-Goals

本 RFC 不计划：

- 创建 CSS Runtime
- 创建 CSS Selector System
- 创建类似 Web CSS Cascade 的系统
- 强制现有用户迁移
- 删除或改变现有 `gpui_component::button::Button`
- 将所有组件实现 Copy 到 Application
- 将复杂基础设施放进 Registry

特别是：

```text
VirtualList
Dock
Overlay
Editor
Input State
Focus Management
Positioning
```

仍然应该由 crate 提供。

---

# 4. Project Structure

整个项目继续使用：

```text
GPUI Components
```

作为品牌。

Repository / Workspace：

```text
gpui-component/
│
├── crates/
│   │
│   ├── ui
│   │
│   ├── base
│   │
│   ├── cli
│   │
├── registry/
│   │
│   ├── ui/
│   ├── blocks/
│   └── themes/
│
├── examples/
│
└── docs/
```

其中两个核心 crate：

```text
gpui-component
gpui-component-base
```

职责完全不同。

---

# 5. gpui-component-base

`gpui-component-base` 是新架构最重要的基础 crate。

Application 可以直接依赖：

```toml
[dependencies]
gpui-component-base = "..."
```

Rust：

```rust
use gpui_component_base::Button;
```

注意这里：

```rust
gpui_component_base::Button
```

不是最终 Styled Button。

它是：

> Button 的基础行为和交互实现。

---

# 6. Base Component API

以 Button 为例。

Base 提供：

```rust
use gpui_component_base::Button;
```

Application-owned Button：

```rust
use gpui::*;
use gpui_component_base as base;

pub struct Button {
    label: SharedString,
}

impl Button {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        base::Button::new("button")
            .child(self.label)
            .h_9()
            .px_4()
            .rounded_md()
            .bg(cx.theme().primary)
            .text_color(cx.theme().primary_foreground)
    }
}
```

这里：

```rust
base::Button
```

负责：

- Mouse interaction
- Press
- Focus
- Keyboard
- Disabled state
- Click event
- Accessibility

Application Button 负责：

- Height
- Padding
- Color
- Border
- Radius
- Typography
- Layout
- Visual states

---

# 7. Base Button API

初步 API：

```rust
pub struct Button {
    id: ElementId,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}
```

使用：

```rust
Button::new("save")
    .disabled(false)
    .on_click(|_, window, cx| {
        // ...
    })
    .child("Save")
```

同时实现：

```rust
impl Styled for Button {}
```

因此：

```rust
Button::new("save")
    .h_9()
    .px_4()
    .rounded_md()
    .bg(...)
```

Application 可以完全控制视觉。

---

# 8. Styling Priority

必须建立明确 Style Priority：

```text
Base behavior style
        ↓
Application component style
        ↓
Variant style
        ↓
Instance override
```

最终原则：

> **The closest style to the application always wins.**

例如：

```rust
Button::new("Save")
    .variant(ButtonVariant::Primary)
    .bg(red())
```

最终：

```text
background = red
```

而不是被内部 Primary Style 覆盖。

Base 尽可能不设置 Visual Style。

---

# 9. Visual State

Base 必须提供状态能力，但不决定状态的视觉表现。

例如：

```rust
base::Button::new("save")
    .hover(|style| {
        style.bg(...)
    })
    .active(|style| {
        style.bg(...)
    })
```

或者直接复用 GPUI 已有 Styled State API。

关键原则：

```text
Base owns state

Application owns state appearance
```

例如：

```text
Base:
hover = true

Application:
hover → background = primary_hover
```

---

# 10. Variants

Variant 不属于 Base。

因此 Base 不应该包含：

```rust
ButtonVariant::Primary
ButtonVariant::Secondary
ButtonVariant::Ghost
ButtonVariant::Destructive
```

这些是 Design System 概念。

应该存在于 Application：

```rust
#[derive(Default, Clone, Copy)]
pub enum ButtonVariant {
    #[default]
    Default,
    Secondary,
    Outline,
    Ghost,
    Destructive,
}
```

以及：

```rust
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
    Icon,
}
```

Registry Button：

```rust
pub struct Button {
    variant: ButtonVariant,
    size: ButtonSize,
}
```

因此不同 Registry Style 可以拥有完全不同的 Variant。

例如某个 Design System：

```rust
ButtonVariant::Primary
ButtonVariant::Secondary
```

另一个：

```rust
ButtonVariant::Filled
ButtonVariant::Tinted
ButtonVariant::Plain
```

Base 不关心这些差异。

---

# 11. Compound Components

Button 很简单。

真正需要 Base 层的地方是：

```text
Dialog
Popover
Dropdown
Select
Tabs
Menu
Tooltip
```

例如：

```rust
use gpui_component_base as base;

base::Dialog::new(state)
    .trigger(...)
    .content(...)
```

或者拆成：

```rust
base::Dialog::new(state)
    .trigger(
        base::DialogTrigger::new(...)
    )
    .content(
        base::DialogContent::new(...)
    )
```

Base 负责：

```text
Open / Close
Focus Trap
Escape
Outside Click
Overlay
Position
Keyboard
Accessibility
```

Application：

```text
Dialog Background
Radius
Shadow
Padding
Animation
Title Layout
Close Button
```

---

# 12. Base vs Registry Boundary

一个重要原则：

> **复杂 Behavior 留在 Base，容易理解和修改的 Presentation 放到 Registry。**

例如：

### Base

```text
Button behavior
Dialog state
Popover positioning
Select state
Keyboard navigation
Focus management
VirtualList
Overlay
Input state
Dock state
Accessibility
```

### Registry

```text
Button
Input
Checkbox
Dialog layout
Select trigger
Dropdown appearance
Card
Badge
Sidebar
Settings Panel
Data Table presentation
```

判断标准：

如果 Copy 后用户很难安全维护：

```text
→ Base
```

如果 Copy 后用户很可能希望修改：

```text
→ Registry
```

---

# 13. Theme Architecture

Theme 不应该继续向 Component Configuration 演进。

错误方向：

```rust
Theme {
    button_height,
    button_radius,
    button_background,
    button_hover_background,
    input_height,
    input_radius,
}
```

推荐：

```rust
pub struct Theme {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub typography: TypographyTokens,
    pub shadow: ShadowTokens,
}
```

---

# 14. Semantic Colors

例如：

```rust
pub struct ColorTokens {
    pub background: Hsla,
    pub foreground: Hsla,

    pub surface: Hsla,
    pub surface_foreground: Hsla,

    pub primary: Hsla,
    pub primary_foreground: Hsla,

    pub secondary: Hsla,
    pub secondary_foreground: Hsla,

    pub muted: Hsla,
    pub muted_foreground: Hsla,

    pub accent: Hsla,
    pub accent_foreground: Hsla,

    pub destructive: Hsla,
    pub destructive_foreground: Hsla,

    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
}
```

Registry Button 使用：

```rust
.bg(cx.theme().colors.primary)
.text_color(cx.theme().colors.primary_foreground)
```

而不是：

```rust
.bg(cx.theme().button_background)
```

---

# 15. Registry

Registry 是 Open-Code 架构的分发中心。

目录：

```text
registry/
│
├── ui/
│   ├── button.json
│   ├── input.json
│   ├── checkbox.json
│   ├── dialog.json
│   └── select.json
│
├── blocks/
│   ├── sidebar.json
│   ├── settings.json
│   ├── command-palette.json
│   └── data-table.json
│
└── themes/
    ├── default.json
    ├── compact.json
    └── native.json
```

---

# 16. Registry JSON

建议 Registry Item 使用如下结构：

```json
{
  "$schema": "https://gpui.rs/registry-item.schema.json",
  "name": "button",
  "type": "registry:ui",
  "description": "A customizable button component.",
  "dependencies": ["gpui-component-base"],
  "registryDependencies": [],
  "files": [
    {
      "path": "ui/button.rs",
      "type": "registry:ui",
      "target": "src/ui/button.rs"
    }
  ]
}
```

复杂组件：

```json
{
  "$schema": "https://gpui.rs/registry-item.schema.json",
  "name": "dialog",
  "type": "registry:ui",
  "description": "A modal dialog with focus management.",
  "dependencies": ["gpui-component-base"],
  "registryDependencies": ["button"],
  "files": [
    {
      "path": "ui/dialog.rs",
      "type": "registry:ui",
      "target": "src/ui/dialog.rs"
    }
  ]
}
```

---

# 17. Registry Item Types

第一版建议支持：

```text
registry:ui
registry:block
registry:theme
registry:lib
```

### registry:ui

基础 UI：

```text
button
input
checkbox
dialog
select
tabs
```

### registry:block

更大的 UI Pattern：

```text
sidebar
settings
command-palette
data-table
ai-chat
```

### registry:theme

Theme / Tokens：

```text
default
compact
native
```

### registry:lib

Registry Component 共享工具：

```text
utils
theme
icons
```

---

# 18. Registry Dependency

Registry Item 可以依赖其他 Registry Item。

例如：

```text
dialog
 ├── button
 └── icon
```

JSON：

```json
{
  "registryDependencies": ["button", "icon"]
}
```

CLI 自动解析 dependency graph。

---

# 19. Project Configuration

项目根目录创建：

```text
gpui-components.json
```

例如：

```json
{
  "$schema": "https://gpui.rs/components.schema.json",
  "style": "default",
  "ui": "src/ui",
  "theme": "src/theme",
  "icons": "lucide"
}
```

这个文件定义：

- Registry Style
- UI 输出目录
- Theme 位置
- Icon provider

---

# 20. CLI

CLI 名称：

```bash
gpui-component
```

或者未来提供更短 alias：

```bash
gpui
```

第一阶段建议不要占用 `gpui`，因此：

```bash
gpui-component
```

最安全。

---

# 21. Initialize

新项目：

```bash
gpui-component init
```

CLI：

1. 检测 Cargo project
2. 创建 `gpui-components.json`
3. 添加 Base dependency
4. 创建 `src/ui`
5. 创建 Theme
6. 创建 `ui/mod.rs`

例如：

```text
src/
├── main.rs
├── theme.rs
└── ui/
    └── mod.rs
```

---

# 22. Add Component

```bash
gpui-component add button
```

输出：

```text
✔ Checking registry
✔ Installing dependencies
✔ Adding button

Created:
  src/ui/button.rs
```

同时修改：

```rust
// src/ui/mod.rs

mod button;

pub use button::*;
```

使用：

```rust
use crate::ui::Button;
```

---

# 23. Add Multiple Components

支持：

```bash
gpui-component add button input dialog
```

CLI 自动计算：

```text
dialog
   ↓
button
```

避免重复安装。

---

# 24. Add Block

例如：

```bash
gpui-component add sidebar
```

Registry：

```text
sidebar
├── button
├── tooltip
├── separator
└── icon
```

最终：

```text
src/ui/
├── button.rs
├── tooltip.rs
├── separator.rs
├── sidebar.rs
└── ...
```

用户拥有所有 Presentation Source。

---

# 25. Diff / Update

Open-Code 最大的问题之一是：

> Library 更新以后，Application Component 怎么更新？

不能直接覆盖。

建议提供：

```bash
gpui-component diff button
```

显示：

```text
Registry Button
        ↕
Local Button
```

以及：

```bash
gpui-component update button
```

如果 Local 没修改：

```text
→ automatic update
```

如果修改过：

```text
→ interactive diff
```

不自动覆盖用户源码。

这是 Registry 模式长期能否成功的重要能力。

---

# 26. Registry Metadata

安装时记录：

```json
{
  "button": {
    "registry": "default",
    "version": "0.2.0",
    "hash": "..."
  }
}
```

CLI 因此可以知道：

```text
Installed Version
Local Modified
Registry Updated
```

未来可以实现：

```bash
gpui-component status
```

输出：

```text
Component        Local       Registry
─────────────────────────────────────
button           modified    update available
input            clean       latest
dialog           modified    latest
```

---

# 27. Custom Registry

长期应该允许第三方 Registry：

```bash
gpui-component add acme/button
```

或者：

```text
https://example.com/r/button.json
```

项目配置：

```json
{
  "registries": {
    "acme": "https://ui.acme.com/registry/{name}.json"
  }
}
```

使用：

```bash
gpui-component add acme/button
```

这会让 gpui-component 从：

```text
Component Library
```

演化成：

```text
GPUI Design System Ecosystem
```

第三方可以发布自己的：

```text
Components
Blocks
Themes
Templates
```

而不需要进入 gpui-component 主仓库。

---

# 28. Project Directory

一个完整项目最终可能是：

```text
my-app/
│
├── Cargo.toml
├── gpui-components.json
│
└── src/
    │
    ├── main.rs
    │
    ├── theme.rs
    │
    ├── app.rs
    │
    └── ui/
        │
        ├── mod.rs
        ├── button.rs
        ├── input.rs
        ├── checkbox.rs
        ├── dialog.rs
        ├── select.rs
        ├── sidebar.rs
        └── data_table.rs
```

# 29. 验收

1. `gpui-component-base` 提供基础行为和交互实现。
2. `gpui-component` 提供 Registry + CLI，并确保 gpui-component 现有 API 继续工作。任何改为包装或组合 Base 的现有 UI 组件，其行为、交互、设计、功能和公开 API 必须与迁移前 100% 一模一样；绝对不能改变现有 UI 或 UX。
3. `gpui-component` CLI 可以在 Application 中创建 `src/ui`，并添加组件源码。
4. Application 可以自由修改 `src/ui` 中的组件源码，形成自己的 Design System。
5. 确保 themes 完成迁移，支持 semantic color tokens。

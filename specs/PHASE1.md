下面是整理后的方案文档，采用 **Markdown**，以目前确定的方向为基础。

# gpui-component 重构方案（Draft）

## 背景

目前 `gpui-component` 已经提供了较完整的桌面 UI 组件库，包括：

- 60+ Components
- Theme
- Styled API
- Overlay
- Dock
- VirtualList
- Table
- Editor 等

对于普通应用已经足够使用。

但是随着更多第三方项目接入，一个问题越来越明显：

> **目前的组件拥有完整的实现，使用者只能修改 Theme 或有限的 API，因此很难构建属于自己的 Design System。**

最终导致：

- 大部分项目长得都很像
- 每增加一种 UI 风格，都需要继续扩充 Theme API
- 每出现新的定制需求，就需要增加新的 Builder 方法
- Component API 越来越庞大

而这正是 `shadcn/ui` 成功解决的问题。

---

# 目标

本次重构不是为了复制 shadcn，而是学习它最核心的设计思想：

> **Ownership belongs to the application.**

也就是说：

不是 Component Library 拥有组件。

而是：

**应用拥有自己的组件源码。**

gpui-component 提供的是：

- UI Foundation
- Behavior
- Infrastructure
- 官方默认实现
- Registry

---

# 新架构

```text
                 gpui-component
               (Project / Brand)

 ┌────────────────────────────────────┐
 │ gpui-base                │
 │                                    │
 │ Button                             │
 │ Dialog                             │
 │ Overlay                            │
 │ VirtualList                        │
 │ Focus                              │
 │ Accessibility                      │
 │ Animation                          │
 │ Tokens                             │
 └────────────────────────────────────┘
                  ▲
                  │
                  │
      ┌───────────────────────┐
      │ Registry Templates    │
      │                       │
      │ Button.rs             │
      │ Dialog.rs             │
      │ Sidebar.rs            │
      │ DataTable.rs          │
      └───────────────────────┘
                  ▲
                  │
                  │
         User Project (owns UI)
```

整个品牌仍然保持：

```
gpui-component
```

而基础 crate 命名为：

```
gpui-base
```

---

# 为什么选择 gpui-base

最终决定使用：

```
gpui-base
```

原因：

- 简洁
- Cargo 名称不会太长
- 比 `core` 更有语义
- 比 `primitives` 更短
- 不限制未来能力范围

未来里面不仅有 Button，还会包含：

- Overlay
- Focus
- VirtualList
- Dock
- Animation
- Accessibility
- Design Tokens

因此使用 **Base** 比 **Primitive** 更准确。

以后：

```rust
use gpui_base::Button;
use gpui_base::Dialog;
```

这里的 Button 并不是官方风格，而是 Headless Foundation。

---

# gpui-component 的定位

未来：

```
gpui-component
```

不再只是一个 Rust crate。

而是整个生态品牌。

包括：

```
gpui-component

├── base
├── registry
├── blocks
├── icons
├── themes
└── cli
```

以后用户只需要记住一个名字：

```
gpui-component
```

---

# Base 层职责

Base 不负责提供固定视觉风格。

它负责：

## Behavior

例如：

- Hover
- Pressed
- Disabled
- Keyboard
- Focus
- Mouse

---

## Infrastructure

例如：

- Overlay
- Popup
- Positioning
- Focus Scope
- Portal
- Animation
- VirtualList
- Dock

---

## Compound Components

例如：

```
Dialog

Button

Popover

Select

Dropdown
```

这些组件：

- 不绑定品牌颜色
- 不绑定尺寸
- 不绑定圆角
- 不绑定视觉语言

只负责：

> Interaction

---

## Design Tokens

Base 提供统一 Token。

例如：

```rust
Theme {
    colors,
    spacing,
    radius,
    typography,
    shadow,
}
```

而不是：

```rust
button_primary_bg
button_hover_bg
button_radius
```

Theme 只提供：

Semantic Tokens。

---

# Registry

新增 Registry。

例如：

```
gpui-component add button

gpui-component add dialog

gpui-component add sidebar
```

生成：

```
src/ui/

button.rs

dialog.rs

sidebar.rs
```

这些源码属于：

Application。

不是 Library。

以后修改：

- Layout
- Radius
- Hover
- Animation
- Variant

全部直接改源码。

不需要等待 Library 更新。

---

# Component Ownership

旧模式：

```
Library owns components

↓

Application configures Theme
```

新模式：

```
Library owns behavior

↓

Application owns components
```

这是整个重构最重要的一点。

---

# 为什么不是继续扩 Theme

继续扩 Theme：

```
Theme

↓

Button Background

↓

Button Hover

↓

Button Border

↓

Button Radius

↓

Button Shadow
```

最终：

Theme 会越来越大。

每个组件都会不断增加新的配置项。

长期不可维护。

因此：

Theme 应只保留：

Semantic Design Tokens。

具体组件样式应该属于：

Component Source。

---

# 官方组件

未来官方仍然维护：

默认 Button

默认 Dialog

默认 Input

默认 Table

但是它们通过 Registry 分发。

不是唯一实现。

用户可以：

- 使用官方版本
- 修改官方版本
- 自己维护版本

而不是只能依赖 Library。

---

# 迁移策略

原则：

> **100% 向后兼容。**

现有：

```rust
use gpui_component::button::Button;
```

无需修改。

内部逐步迁移到：

```
gpui-base
```

作为实现基础。

老项目：

无需升级代码。

新项目：

推荐：

```
gpui-component init

gpui-component add button
```

即可拥有：

自己的 UI。

---

# 分阶段实施

## 第一阶段

抽离 Base。

包括：

- Theme Tokens
- Overlay
- Focus
- Behavior
- Infrastructure

保持 API 不变。

---

## 第二阶段

抽离简单组件。

例如：

- Button
- Checkbox
- Badge
- Input

建立：

Registry。

---

## 第三阶段

抽离复杂组件。

例如：

- Dialog
- Select
- Dropdown
- Popover
- Tabs

---

## 第四阶段

推出：

Blocks。

例如：

- Sidebar
- Command Palette
- Settings
- AI Chat
- DataTable
- Dock Layout

---

## 第五阶段

支持：

Design Styles。

例如：

```
gpui-component init --style default

gpui-component init --style macos

gpui-component init --style linear

gpui-component init --style vscode
```

实现真正意义上的：

Application-owned Design System。

---

# 最终目标

gpui-component 不再只是一个 Component Library。

而是 GPUI 生态的 UI 基础设施。

其中：

- **gpui-base**：提供行为、交互、基础设施与 Design Tokens。
- **Registry**：提供官方组件源码与 Blocks。
- **Application**：拥有最终 UI，实现自己的 Design System。

最终形成如下关系：

```
GPUI
    │
    ▼
gpui-base
    │
    ▼
Registry Components / Blocks
    │
    ▼
Application-owned UI
```

核心理念只有一句话：

> **Behavior belongs to the framework. Style belongs to the application.**

如果需要，我也可以进一步补充一份 **API 设计草案**（包括 `gpui-base::Button`、Registry JSON 格式、CLI 工作流、目录结构以及迁移示例），作为重构 RFC 文档。

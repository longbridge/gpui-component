---
title: DropdownButton
description: DropdownButton 由一个主按钮和一个触发下拉菜单的按钮组合而成。
---

# DropdownButton

[DropdownButton] 是一个组合型按钮组件。点击左侧主按钮时可以执行独立动作，点击右侧触发按钮时则会展开下拉菜单。

它同时继承了 [Button] 的许多能力，例如变体、尺寸、图标和加载状态。

## 导入

```rust
use gpui_component::button::{Button, DropdownButton};
```

## 用法

```rust
use gpui::Anchor;

DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
            .menu("Option 2", Box::new(MyAction))
            .separator()
            .menu("Option 3", Box::new(MyAction))
    })
```

### 变体

与 [Button] 一样，DropdownButton 支持不同视觉变体：

```rust
DropdownButton::new("dropdown")
    .primary()
    .button(Button::new("btn").label("Primary"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### 自定义锚点

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Anchor::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[DropdownButton]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.DropdownButton.html
[ButtonCustomVariant]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonCustomVariant.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

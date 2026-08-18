---
title: DropdownButton
description: DropdownButton 由一个主按钮和一个触发下拉菜单的按钮组合而成。
---

# DropdownButton

[DropdownButton] 是一个组合型按钮组件。点击左侧主按钮时可以执行独立动作，点击右侧触发按钮时则会展开下拉菜单。

用 [ButtonVariants] 设置的变体和用 [Sizable] 设置的尺寸会同时作用于两半；其余能力——文案、图标、提示、加载状态、点击回调——都属于内层的 [Button]。

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

未被选中的 `ghost` 按钮会呈现为两个相互独立的按钮，而不是拼接在一起的一对，这在工具栏中更易辨认。

### 内层按钮的选项

属于动作本身的选项写在内层 [Button] 上：

```rust
DropdownButton::new("dropdown")
    .button(
        Button::new("btn")
            .label("Save")
            .compact()
            .loading(is_saving)
            .tooltip("Save the current view")
            .on_click(|_, _, _| println!("Saved")),
    )
    .dropdown_menu(|menu, _, _| {
        menu.menu("Save as…", Box::new(MyAction))
    })
```

DropdownButton 上不设置变体时，内层按钮的变体会应用到两半；不设置尺寸时，内层按钮自己的尺寸会被保留。

### 自定义锚点

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Anchor::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### 用 ButtonGroup 组合

DropdownButton 只是 [ButtonGroup] 上的一层薄封装——按钮组可以直接容纳一个打开菜单的按钮。当拆分按钮需要超过两个成员，或者两半需要各自不同的样式时，直接使用按钮组：

```rust
ButtonGroup::new("save")
    .child(Button::new("save").label("Save").on_click(|_, _, _| {}))
    .child(
        Button::new("save-options")
            .dropdown_caret(true)
            .dropdown_menu(|menu, _, _| menu.menu("Save as…", Box::new(MyAction))),
    )
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[ButtonGroup]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonGroup.html
[ButtonVariants]: https://docs.rs/gpui-component/latest/gpui_component/button/trait.ButtonVariants.html
[DropdownButton]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.DropdownButton.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

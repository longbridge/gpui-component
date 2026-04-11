---
title: Input
description: 带校验、掩码和多种扩展能力的文本输入组件。
---

# Input

Input 是一个灵活的文本输入组件，支持校验、输入掩码、前后缀元素以及多种交互状态。

## 导入

```rust
use gpui_component::input::{InputState, Input};
```

## 用法

### 基础输入框

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### 默认值

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### 可清空

```rust
Input::new(&input)
    .cleanable(true)
```

### 前缀和后缀

```rust
use gpui_component::{Icon, IconName};

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

### 密码输入

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .mask_toggle()
```

### 尺寸

```rust
Input::new(&input).large()
Input::new(&input)
Input::new(&input).small()
```

### 禁用态

```rust
Input::new(&input).disabled(true)
```

### 按 ESC 清空

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape()
);

Input::new(&input)
```

### 输入校验

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);
```

### 输入掩码

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

use gpui_component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### 监听事件

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### 自定义外观

```rust
Input::new(&input).appearance(false)

div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

## 示例

### 搜索输入框

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### 金额输入

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### 多输入表单

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```

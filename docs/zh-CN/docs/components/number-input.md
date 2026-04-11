---
title: NumberInput
description: 带增减按钮与数字格式化能力的数值输入组件。
---

# NumberInput

NumberInput 是针对数值输入场景设计的组件，内置递增/递减按钮，并支持最小值、最大值、步进值以及千分位格式化等能力。

## 导入

```rust
use gpui_component::input::{InputState, NumberInput, NumberInputEvent, StepAction};
```

## 用法

### 基础数值输入

```rust
let number_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter number")
        .default_value("1")
);

NumberInput::new(&number_input)
```

### 最小值 / 最大值校验

```rust
let integer_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Integer value")
        .pattern(Regex::new(r"^\d+$").unwrap())
);

NumberInput::new(&integer_input)
```

### 数字格式化

```rust
use gpui_component::input::MaskPattern;

let currency_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Amount")
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

NumberInput::new(&currency_input)
```

### 不同尺寸

```rust
NumberInput::new(&input).large()
NumberInput::new(&input)
NumberInput::new(&input).small()
```

### 前缀与后缀

```rust
use gpui_component::{button::{Button, ButtonVariants}, IconName};

NumberInput::new(&input)
    .prefix(div().child("$"))

NumberInput::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )
```

### 禁用状态

```rust
NumberInput::new(&input).disabled(true)
```

### 关闭默认外观

```rust
div()
    .w_full()
    .bg(cx.theme().secondary)
    .rounded(cx.theme().radius)
    .child(NumberInput::new(&input).appearance(false))
```

### 处理 NumberInput 事件

```rust
let number_input = cx.new(|cx| InputState::new(window, cx));
let mut value: i64 = 0;

cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            if let Ok(new_value) = text.parse::<i64>() {
                view.value = new_value;
            }
        }
        _ => {}
    }
});

cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        NumberInputEvent::Step(step_action) => {
            match step_action {
                StepAction::Increment => {
                    view.value += 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
                StepAction::Decrement => {
                    view.value -= 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
            }
        }
    }
});
```

### 程序化控制

```rust
NumberInput::increment(&number_input, window, cx);
NumberInput::decrement(&number_input, window, cx);
```

## API 参考

### NumberInput

| 方法 | 说明 |
| ------------------------------ | ------------------------------------------ |
| `new(state)` | 使用 `InputState` 创建数值输入组件 |
| `placeholder(str)` | 设置占位文案 |
| `size(size)` | 设置尺寸 |
| `prefix(el)` | 添加前缀元素 |
| `suffix(el)` | 添加后缀元素 |
| `appearance(bool)` | 开启或关闭默认样式 |
| `disabled(bool)` | 设置禁用状态 |
| `increment(state, window, cx)` | 以代码方式递增 |
| `decrement(state, window, cx)` | 以代码方式递减 |

### NumberInputEvent

| 事件 | 说明 |
| ------------------ | ---------------------------------- |
| `Step(StepAction)` | 点击增减按钮时触发 |

### StepAction

| 动作 | 说明 |
| ----------- | ------------------------- |
| `Increment` | 数值增加 |
| `Decrement` | 数值减少 |

### InputState（数值相关方法）

| 方法 | 说明 |
| ----------------------------------- | ------------------------------------------------------- |
| `pattern(regex)` | 设置校验正则，例如只允许数字 |
| `mask_pattern(MaskPattern::Number)` | 设置数字格式化规则 |
| `value()` | 获取当前展示值 |
| `unmask_value()` | 获取未格式化的真实数值 |

### MaskPattern::Number

| 字段 | 类型 | 说明 |
| ----------- | --------------- | -------------------------------------- |
| `separator` | `Option<char>` | 千分位分隔符 |
| `fraction` | `Option<usize>` | 小数位数 |

## 键盘导航

| 按键 | 行为 |
| ----------- | -------------------------- |
| `↑` | 增加数值 |
| `↓` | 减少数值 |
| `Tab` | 切换到下一个字段 |
| `Shift+Tab` | 切换到上一个字段 |
| `Enter` | 提交或确认当前值 |
| `Escape` | 清空输入（若启用） |

## 示例

### 整数计数器

```rust
struct CounterView {
    counter_input: Entity<InputState>,
    counter_value: i32,
}

impl CounterView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let counter_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("Count")
                .default_value("0")
                .pattern(Regex::new(r"^-?\d+$").unwrap())
        );

        let _subscription = cx.subscribe_in(&counter_input, window, Self::on_number_event);

        Self {
            counter_input,
            counter_value: 0,
        }
    }

    fn on_number_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                self.counter_value += 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                self.counter_value -= 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
        }
    }
}

NumberInput::new(&self.counter_input)
```

### 货币输入

```rust
struct PriceInput {
    price_input: Entity<InputState>,
    price_value: f64,
}

impl PriceInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let price_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("0.00")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
        );

        Self {
            price_input,
            price_value: 0.0,
        }
    }
}

h_flex()
    .gap_2()
    .child(div().child("$"))
    .child(NumberInput::new(&self.price_input))
```

### 带上下限的数量选择器

```rust
struct QuantitySelector {
    quantity_input: Entity<InputState>,
    quantity: u32,
    min_quantity: u32,
    max_quantity: u32,
}

impl QuantitySelector {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let min_quantity = 1;
        let max_quantity = 99;

        let quantity_input = cx.new(|cx|
            InputState::new(window, cx)
                .default_value(min_quantity.to_string())
                .pattern(Regex::new(&format!(r"^[{}-{}]\d*$", min_quantity, max_quantity)).unwrap())
        );

        let _subscription = cx.subscribe_in(&quantity_input, window, Self::on_quantity_event);

        Self {
            quantity_input,
            quantity: min_quantity,
            min_quantity,
            max_quantity,
        }
    }

    fn on_quantity_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                if self.quantity < self.max_quantity {
                    self.quantity += 1;
                    state.update(cx, |input, cx| {
                        input.set_value(self.quantity.to_string(), window, cx);
                    });
                }
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                if self.quantity > self.min_quantity {
                    self.quantity -= 1;
                    state.update(cx, |input, cx| {
                        input.set_value(self.quantity.to_string(), window, cx);
                    });
                }
            }
        }
    }
}

NumberInput::new(&self.quantity_input).small()
```

### 浮点数输入

```rust
struct FloatInput {
    float_input: Entity<InputState>,
    float_value: f64,
    step: f64,
}

impl FloatInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let float_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("0.0")
                .pattern(Regex::new(r"^-?\d*\.?\d*$").unwrap())
        );

        Self {
            float_input,
            float_value: 0.0,
            step: 0.1,
        }
    }

    fn on_float_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                self.float_value += self.step;
                state.update(cx, |input, cx| {
                    input.set_value(format!("{:.1}", self.float_value), window, cx);
                });
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                self.float_value -= self.step;
                state.update(cx, |input, cx| {
                    input.set_value(format!("{:.1}", self.float_value), window, cx);
                });
            }
        }
    }
}
```

## 最佳实践

1. 客户端和服务端都应校验数值输入。
2. 对用户可操作范围设置明确上下限。
3. 根据业务场景选择合适的步进值。
4. 对非法输入提供清晰反馈。
5. 在整个应用中保持统一的数字格式。
6. 高频点击增减时，必要时做节流或去抖。
7. 始终为输入提供清晰的标签与描述。

---
title: ComboBox
description: 带有可搜索下拉列表的自动补全输入组件。
---

# ComboBox

ComboBox 允许用户从可搜索的列表中选择一个（或多个）值。

与 [Select](select) 相比，`ComboBox` 额外支持自定义触发器渲染和自定义列表项渲染，便于构建富交互的选择 UI。

`MultiComboBox` 是多选变体——点击列表项会切换其选中状态，下拉菜单保持展开直到用户主动关闭。

## 导入

```rust
use gpui_component::combo_box::{
    ComboBox, ComboBoxState, ComboBoxEvent,
    MultiComboBox, MultiComboBoxState, MultiComboBoxEvent,
    TriggerCtx, MultiTriggerCtx,
};
use gpui_component::searchable_list::{
    SearchableListItem, SearchableVec, SearchableGroup,
};
```

## 用法

### 基础单选

```rust
let state = cx.new(|cx| {
    ComboBoxState::new(
        SearchableVec::new(vec!["Next.js", "SvelteKit", "Nuxt.js"]),
        None, // 无初始选中
        window,
        cx,
    )
    .searchable(true)
});

ComboBox::new(&state)
    .placeholder("选择框架...")
    .search_placeholder("搜索...")
    .w_full()
```

### 预选项

通过索引路径指定预选的列表项：

```rust
let state = cx.new(|cx| {
    ComboBoxState::new(items, Some(IndexPath::default()), window, cx)
});
```

### 分组列表项

使用 `SearchableGroup` 对列表项进行分组：

```rust
let grouped = SearchableVec::new(vec![
    SearchableGroup::new("水果").items(vec![
        FoodItem::new("苹果"),
        FoodItem::new("香蕉"),
    ]),
    SearchableGroup::new("蔬菜").items(vec![
        FoodItem::new("胡萝卜"),
        FoodItem::new("菠菜"),
    ]),
]);

let state = cx.new(|cx| {
    ComboBoxState::new(grouped, None, window, cx).searchable(true)
});

ComboBox::new(&state)
```

### 实现 `SearchableListItem`

`String`、`SharedString` 和 `&'static str` 已内置实现了 `SearchableListItem`。自定义类型需手动实现该 trait：

```rust
#[derive(Clone)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl SearchableListItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &SharedString {
        &self.code
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(query)
            || self.code.to_lowercase().contains(query)
    }
}
```

### 禁用列表项

在列表项的 `disabled()` 方法中返回 `true` 即可将该项设为不可选：

```rust
impl SearchableListItem for MyItem {
    // ...
    fn disabled(&self) -> bool {
        self.is_unavailable
    }
}
```

### 自定义勾选图标

```rust
ComboBox::new(&state)
    .check_icon(Icon::new(IconName::CircleCheck))
```

### 底部操作按钮

在下拉菜单底部渲染一个固定操作项（如"新建"按钮）：

```rust
ComboBox::new(&state)
    .footer(|_, cx| {
        Button::new("add-new")
            .ghost()
            .label("新建项目")
            .icon(Icon::new(IconName::Plus))
            .w_full()
            .justify_start()
            .into_any_element()
    })
```

### 自定义触发器

完全覆盖触发器元素的渲染。`TriggerCtx` 包含当前选中状态、开关标志和尺寸信息：

```rust
ComboBox::new(&state)
    .render_trigger(|ctx, _, cx| {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .when_some(ctx.selected_item, |this, item| {
                this.child(
                    div()
                        .bg(cx.theme().accent)
                        .rounded_sm()
                        .px_1p5()
                        .py_0p5()
                        .text_sm()
                        .child(item.title()),
                )
            })
            .when(ctx.selected_item.is_none(), |this| {
                this.text_color(cx.theme().muted_foreground)
                    .child("请选择...")
            })
            .into_any_element()
    })
```

### 自定义列表项渲染

覆盖每行列表项的渲染方式。设置后自动隐藏默认的尾部勾选图标，由闭包完全控制行内容：

```rust
ComboBox::new(&state)
    .render_item(|item: &MyItem, is_selected, _, cx| {
        h_flex()
            .w_full()
            .gap_2()
            .items_center()
            .child(Icon::new(item.icon.clone()).small())
            .child(div().child(item.title()))
            .into_any_element()
    })
```

### 尺寸

```rust
ComboBox::new(&state).large()
ComboBox::new(&state)  // 默认（medium）
ComboBox::new(&state).small()
```

### 可清除

```rust
ComboBox::new(&state).cleanable(true) // 有选中值时显示清除按钮
```

### 禁用状态

```rust
ComboBox::new(&state).disabled(true)
```

### 事件监听

```rust
cx.subscribe_in(&state, window, |view, _, event, window, cx| {
    match event {
        ComboBoxEvent::Confirm(value) => {
            // value 为 Option<Value>
        }
    }
});
```

### 程序化操控

```rust
// 通过索引设置
state.update(cx, |s, cx| {
    s.set_selected_index(Some(IndexPath::default()), window, cx);
});

// 通过值设置（需要 Value: PartialEq）
state.update(cx, |s, cx| {
    s.set_selected_value(&"my-value".into(), window, cx);
});

// 读取当前值
let value = state.read(cx).selected_value(); // Option<&Value>
```

## 多选

### 基础多选

`MultiComboBoxState` 保存 `Vec<Value>` 的选中集合。点击列表项切换其选中状态，下拉菜单保持展开直到关闭。

```rust
let state = cx.new(|cx| {
    MultiComboBoxState::new(
        SearchableVec::new(vec!["React", "Vue", "Angular"]),
        vec!["React"], // 预选项
        window,
        cx,
    )
    .searchable(true)
});

MultiComboBox::new(&state)
    .placeholder("选择框架")
```

### 自定义多选触发器

`MultiTriggerCtx` 提供 `selected_values: &[Value]`：

```rust
MultiComboBox::new(&state)
    .render_trigger(|ctx, _, cx| {
        if ctx.selected_values.is_empty() {
            return div()
                .text_color(cx.theme().muted_foreground)
                .child("请选择...")
                .into_any_element();
        }

        h_flex()
            .flex_wrap()
            .gap_1()
            .children(ctx.selected_values.iter().map(|val| {
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(cx.theme().border)
                    .px_1p5()
                    .py_0p5()
                    .text_sm()
                    .child(*val)
            }))
            .into_any_element()
    })
```

### 多选事件

```rust
cx.subscribe_in(&state, window, |view, _, event, window, cx| {
    match event {
        MultiComboBoxEvent::Change(values) => {
            // 每次切换时触发
        }
        MultiComboBoxEvent::Confirm(values) => {
            // 下拉菜单关闭时触发
        }
    }
});
```

### 程序化操控多选

```rust
state.update(cx, |s, cx| {
    s.add_value("Vue", cx);
    s.remove_value(&"React", cx);
    s.clear_selection(cx);
    s.set_selected_values(vec!["Angular", "Svelte"], cx);
});

let values = state.read(cx).selected_values(); // &[Value]
```

## 键盘快捷键

| 按键       | 操作                             |
| ---------- | -------------------------------- |
| `Tab`      | 聚焦触发器                       |
| `Enter`    | 打开菜单或确认当前高亮项         |
| `↑ / ↓`   | 在选项间导航（未打开时自动打开） |
| `Escape`   | 关闭菜单                         |

## 主题样式

- `background` — 触发器背景
- `input` — 触发器边框颜色
- `foreground` — 文字颜色
- `muted_foreground` — 占位符和禁用文字颜色
- `border` — 菜单边框颜色
- `radius` — 圆角

---
title: Command
description: 命令面板 —— 经过过滤的命令与快捷操作列表。
---

# Command

命令面板是带有分组、快捷键提示和键盘导航的命令过滤列表。可以内嵌使用，也可以组合到现有对话框中，作为 `⌘K` 风格的菜单。列表使用虚拟滚动，因此大型面板只渲染可见行。

## 引入

```rust
use gpui_component::command::{
    Command, CommandEntry, CommandEvent, CommandGroup, CommandItem, CommandState,
};
```

## 组合方式

命令保存在 [`CommandState`] 实体中；由 [`Command`] 渲染该状态。

```text
CommandState
├── CommandItem                 // 未分组
├── CommandGroup
│   ├── CommandItem
│   └── CommandItem
├── CommandEntry::Separator
└── CommandGroup
    ├── CommandItem
    └── CommandItem
```

## 用法

### 内嵌

```rust
let state = cx.new(|cx| {
    CommandState::new(window, cx)
        .group(
            CommandGroup::new("Suggestions")
                .item(CommandItem::new("calendar").label("Calendar").icon(IconName::Calendar))
                .item(CommandItem::new("search").label("Search Emoji").icon(IconName::Search))
                .item(CommandItem::new("calc").label("Calculator").disabled(true)),
        )
        .separator()
        .group(
            CommandGroup::new("Settings")
                .item(CommandItem::new("profile").label("Profile").icon(IconName::User).shortcut("⌘P"))
                .item(CommandItem::new("billing").label("Billing").shortcut("⌘B")),
        )
});

Command::new(&state)
    .placeholder("Type a command or search...")
    .empty("No results found.")
    .w(px(380.))
```

### 无搜索的快捷操作

为紧凑的操作面板关闭搜索。它没有搜索框，不会过滤条目，并且 `state.focus(window, cx)` 会聚焦 Command 外框，因此仍可使用方向键、Enter 和 Escape 操作。

```rust
let actions = cx.new(|cx| {
    CommandState::new(window, cx)
        .searchable(false)
        .item(CommandItem::new("New File").icon(IconName::Plus))
        .item(CommandItem::new("Duplicate").icon(IconName::Copy))
        .item(CommandItem::new("Move to Trash").icon(IconName::Delete))
});

Command::new(&actions).w(px(380.))
```

默认的 `.searchable(true)` 下，`state.focus(window, cx)` 和 [`Focusable::focus_handle`] 会改为聚焦搜索输入框。

### 在对话框中

使用现有的 [`WindowExt::open_dialog`] API 组合命令面板。订阅 [`CommandEvent`]，只在收到 `Confirm` 时显式关闭对话框。`CommandState` 会传播 `Cancel`，因此宿主 Dialog 负责 Escape/Cancel 的关闭；不要再次关闭。Command 不提供对话框专用 API。`header` 渲染在可选搜索框和列表之上，`footer` 渲染在列表之下。

```rust
use gpui_component::WindowExt as _;

let state = self.command_state.clone();
window.open_dialog(cx, move |dialog, _, _| {
    let state = state.clone();
    dialog.close_button(false).p_0().content(move |content, _, _| {
        content.child(
            Command::new(&state)
                .bordered(false)
                .placeholder("Type a command or search...")
                .header(|state, _, cx| {
                    h_flex()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child("Commands")
                        .child(format!("{} matches", state.matched_count()))
                })
                .footer(|_, _, cx| {
                    h_flex()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child("↑↓ Navigate")
                        .child("Enter Select")
                        .child("Escape Close")
                }),
        )
    })
});
```

### 响应选择

可以给条目设置回调：

```rust
CommandItem::new("profile")
    .label("Profile")
    .on_select(|window, cx| {
        window.push_notification("Opening profile", cx);
    })
```

也可以订阅状态：

```rust
cx.subscribe(&state, |this, _, event: &CommandEvent, cx| {
    match event {
        CommandEvent::Select(value) => { /* highlight moved */ }
        CommandEvent::Confirm(value) => { /* clicked or Enter */ }
        CommandEvent::Query(query) => { /* the query changed */ }
        CommandEvent::Cancel => { /* Escape on an empty query */ }
    }
})
```

### 动态更新命令

```rust
state.update(cx, |state, cx| {
    state.set_entries(
        results
            .into_iter()
            .map(|name| CommandEntry::Item(CommandItem::new(name))),
        cx,
    );
});
```

## 搜索

默认情况下，`CommandItem::matches(&self, query: &str) -> bool` 会在条目的 label、value 和 keywords 中进行忽略大小写的子串匹配。空查询会匹配全部条目。分组中的条目全被过滤时，其标题会隐藏；过滤后位于首尾或相邻的分隔线不会显示。

```rust
CommandItem::new("profile")
    .label("Profile")
    .keywords(["account", "user"])
```

当应用需要不同的匹配策略时，可使用自定义过滤器。这个股票搜索先检查股票代码，再检查公司名称：

```rust
let stocks = cx.new(|cx| {
    CommandState::new(window, cx)
        .filter(|item, query| {
            let query = query.to_lowercase();
            item.value().to_lowercase().contains(&query)
                || item.title().to_lowercase().contains(&query)
        })
        .item(CommandItem::new("AAPL.US").label("Apple Inc."))
        .item(CommandItem::new("NVDA.US").label("NVIDIA Corporation"))
});
```

自定义谓词只会在搜索开启且查询非空时运行；否则全部条目保持可见。远程搜索时，监听 `CommandEvent::Query`，用返回结果替换条目；若仍需本地过滤，请把查询词保留在条目的 value 或 label 中。等待结果时使用 `set_loading`，以隐藏空状态文案。

## 自定义行与虚拟滚动

`CommandItem::element` 会替换条目的图标和 label 内容。Command 在重建 `v_virtual_list` 的尺寸时，会测量每一条扁平化的行，因此自定义行可以拥有各自的固有高度。应按列表可用宽度构建行，并在状态更新前保持渲染内容稳定，因为虚拟列表会复用已保存的尺寸。

```rust
CommandState::new(window, cx)
    .item(CommandItem::new("compact").element(|_, _| {
        h_flex().w_full().py_1().child("Compact custom row")
    }))
    .item(CommandItem::new("expanded").element(|_, cx| {
        v_flex()
            .w_full()
            .py_4()
            .child("Expanded custom row")
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Extra detail"))
    }))
```

## Command

| 方法 | 签名与说明 |
| --- | --- |
| `new` | `new(&Entity<CommandState>) -> Command` 渲染状态。 |
| `placeholder` | `placeholder(impl Into<SharedString>) -> Self` 设置搜索框占位文本。 |
| `empty` | `empty(impl Into<SharedString>) -> Self` 设置无匹配时的文案。 |
| `max_h` | `max_h(impl Into<DefiniteLength>) -> Self` 设置列表最大高度。默认：`18.75rem`（300px）。 |
| `bordered` | `bordered(bool) -> Self` 绘制外边框和圆角。默认：`true`。 |
| `header` | `header<F, E>(F) -> Self`，其中 `F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static`、`E: IntoElement`；渲染在搜索框和列表之上。 |
| `footer` | `footer<F, E>(F) -> Self`，使用相同的回调约束；渲染在列表之下。 |

`Command` 实现了 [`Styled`]，因此 `w`、`max_w`、`bg` 和其他样式可作用于面板外框。

## CommandItem

| 方法 | 说明 |
| --- | --- |
| `new(value)` | value 标识条目，在 `label` 设置前也作为其 label。 |
| `label` | 设置可见 label。 |
| `icon` | 设置前置图标。 |
| `shortcut` | 设置尾部快捷键提示；实际按键由应用绑定。 |
| `checked` | 绘制尾部勾选；`shortcut` 会占用该位置。 |
| `keywords` | 添加默认匹配词。 |
| `disabled` | 使条目不可交互，并在键盘导航时跳过。 |
| `element` | 用自定义元素替换行内容。 |
| `on_select` | 点击或 Enter 确认时运行。 |

## CommandState

| 方法 | 签名与说明 |
| --- | --- |
| `new` | `new(&mut Window, &mut Context<Self>) -> Self` 创建一个空的、可搜索的面板。 |
| `item` / `group` / `separator` | 添加未分组条目、分组或分隔线。 |
| `searchable` | `searchable(bool) -> Self` 开启本地过滤和搜索框。默认：`true`。 |
| `filter` | `filter<F>(F) -> Self`，其中 `F: Fn(&CommandItem, &str) -> bool + 'static`，替换默认匹配。 |
| `set_entries` | 替换全部条目。 |
| `query` / `set_query` | 读取或替换搜索词。 |
| `selected_index` / `selected_value` | 读取当前高亮的匹配条目。 |
| `matched_count` | 返回匹配条目数。 |
| `focus` | `focus(&self, &mut Window, &mut App)`：可搜索时聚焦输入框，否则聚焦 Command 外框。 |
| `set_loading` | 显示搜索加载动画，并在加载时隐藏空状态文案。 |

## 键盘快捷键

| 按键 | 行为 |
| --- | --- |
| `↑` / `↓` | 移动高亮，循环并跳过禁用项。 |
| `Enter` | 确认当前高亮项。 |
| `Escape` | 清空搜索词；若已为空则发出 `Cancel`。 |

## 最佳实践

1. 对相关命令分组，并为别名补充关键词。
2. 对紧凑、可用键盘导航的快捷操作使用 `searchable(false)`。
3. 将 `shortcut` 视为视觉提示，在应用中绑定实际按键。
4. 使用插槽承载应用自有的状态和提示，不要增加 Command 专用对话框层。
5. 每个正在渲染的面板使用独立的 [`CommandState`]。

[Command]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.Command.html
[CommandState]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandState.html
[CommandGroup]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandGroup.html
[WindowExt::open_dialog]: https://docs.rs/gpui-component/latest/gpui_component/trait.WindowExt.html#tymethod.open_dialog
[Focusable::focus_handle]: https://docs.rs/gpui/latest/gpui/trait.Focusable.html#tymethod.focus_handle
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html

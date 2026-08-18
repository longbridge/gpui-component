---
title: Command
description: 命令面板 —— 在搜索框下过滤命令与快捷操作的列表。
---

# Command

命令面板：在搜索框下过滤命令列表，支持分组、快捷键提示与键盘导航。可以内嵌使用，也可以放进对话框，作为 `⌘K` 风格的菜单。

列表使用了虚拟滚动，因此即使有上千条命令，也只渲染可见的行。

## 引入

```rust
use gpui_component::command::{
    Command, CommandEntry, CommandEvent, CommandGroup, CommandItem, CommandState,
};
```

## 组合方式

命令保存在 [`CommandState`] 实体中，由 [`Command`] 元素负责渲染。

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

### 在对话框中

使用现有的 [`WindowExt::open_dialog`] 组合命令面板。应用订阅
[`CommandEvent`]，并在收到 `Confirm` 或 `Cancel` 时关闭对话框。

```rust
use gpui_component::WindowExt as _;

let state = self.command_state.clone();
window.open_dialog(cx, move |dialog, _, _| {
    let state = state.clone();
    dialog.close_button(false).p_0().content(move |content, _, _| {
        content.child(
            Command::new(&state)
                .bordered(false)
                .placeholder("Type a command or search..."),
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
        CommandEvent::Select(value) => { /* 高亮项发生变化 */ }
        CommandEvent::Confirm(value) => { /* 点击或按下 Enter */ }
        CommandEvent::Query(query) => { /* 查询词发生变化 */ }
        CommandEvent::Cancel => { /* 查询为空时按下 Escape */ }
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

### 作为搜索面板

远程搜索时，监听 `CommandEvent::Query` 并用返回结果替换 entries。返回的
entries 仍会参与本地匹配，因此应在 value、label 或 keywords 中包含查询词。

```rust
Command::new(&self.search)
    .placeholder("Search stocks...")
    .empty("No stock found.")
```

```rust
fn on_search_event(
    &mut self,
    state: &Entity<CommandState>,
    event: &CommandEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    let CommandEvent::Query(query) = event else {
        return;
    };

    let query = query.trim().to_string();
    state.update(cx, |state, cx| state.set_loading(true, window, cx));

    let state = state.clone();
    // 保存在视图上，下一次查询会取消上一次。
    self._search_task = Some(cx.spawn_in(window, async move |_, cx| {
        let results = fetch(query).await;

        _ = state.update_in(cx, |state, window, cx| {
            state.set_loading(false, window, cx);
            state.set_entries(results.into_iter().map(CommandEntry::Item), cx);
        });
    }));
}
```

`set_loading` 会让搜索框转起加载动画，并在结果返回前不显示空状态文案，避免"请求中"被误读成"没有结果"。

行内容也可以完全自定义 —— 用 `CommandItem::element` 构建行，面板会测量第一行来确定所有行的高度，因此两行的结果行同样能保持虚拟滚动：

```rust
CommandItem::new(symbol).element(move |_, cx| {
    h_flex()
        .w_full()
        .justify_between()
        .child(v_flex().child(symbol).child(name))
        .child(v_flex().items_end().child(price).child(change))
})
```

## 搜索

当查询词（忽略大小写）是条目 label、value 或任意 keyword 的子串时，该命令即被匹配：

```rust
CommandItem::new("profile")
    .label("Profile")
    .keywords(["account", "user"])
```

分组中所有条目都被过滤掉时，分组标题会一并隐藏；过滤后位于首尾或与另一个分隔线相邻的分隔线不会绘制。

## Command

| 方法            | 说明                                              |
| --------------- | ------------------------------------------------- |
| `new(&state)`   | 渲染给定 [`CommandState`] 中的面板。               |
| `placeholder`   | 搜索框的占位文本。                                 |
| `empty`         | 没有命令匹配查询时显示的文案。                     |
| `max_h`         | 列表的最大高度，默认 `18.75rem`（300px）。         |
| `bordered`      | 是否绘制外边框与圆角，默认 `true`。                |

`Command` 实现了 [`Styled`]，因此 `w`、`max_w`、`bg` 等样式方法都可以作用于面板外框。

## CommandItem

| 方法         | 说明                                                             |
| ------------ | ---------------------------------------------------------------- |
| `new(value)` | value 是条目的标识，在未设置 `label` 时也作为显示文本。            |
| `label`      | 行内显示的文本。                                                  |
| `icon`       | 前置图标。                                                        |
| `shortcut`   | 尾部的快捷键提示，实际按键绑定由应用自行完成。                     |
| `checked`    | 在行尾绘制勾选标记；若设置了 `shortcut`，该位置由快捷键占用。       |
| `keywords`   | 参与搜索匹配的额外关键词。                                        |
| `disabled`   | 渲染为不可交互，并在键盘导航时跳过。                              |
| `element`    | 用自定义元素替换行内容。                                          |
| `on_select`  | 点击或按下 Enter 确认时执行。                                     |

面板会测量第一行并让所有条目使用该高度，这正是虚拟滚动得以成立的前提。用 `element` 自定义的行可以任意高，只要每一行高度一致即可。

## CommandState

| 方法              | 说明                                          |
| ----------------- | --------------------------------------------- |
| `new(window, cx)` | 创建一个空面板。                               |
| `item`            | 添加一个未分组的条目。                         |
| `group`           | 添加一个 [`CommandGroup`]。                    |
| `separator`       | 在前后两个分组之间添加分隔线。                 |
| `set_entries`     | 替换全部条目。                                 |
| `query`           | 当前的搜索关键词。                             |
| `set_query`       | 设置搜索关键词。                               |
| `selected_index`  | 高亮项在匹配结果中的下标。                     |
| `selected_value`  | 高亮项的 value。                               |
| `matched_count`   | 当前查询下匹配到的条目数量。                   |
| `focus`           | 把焦点移到搜索框。                             |
| `set_loading`     | 显示搜索框的加载动画，并在此期间隐藏空状态文案。 |

## 键盘快捷键

| 按键      | 行为                                       |
| --------- | ------------------------------------------ |
| `↑` / `↓` | 移动高亮，循环并跳过禁用项                  |
| `Enter`   | 确认当前高亮项                              |
| `Escape`  | 清空搜索词；若已为空则退出面板              |

## 最佳实践

1. **对命令分组**：为每个分组设置标题，并用分隔线区分。
2. **补充关键词**：用户可能用别的名字搜索的命令要设置 `keywords`。
3. **快捷键仅是提示**：`shortcut` 只渲染提示文本，按键绑定需要自己完成。
4. **保持行高一致**：`element` 自定义的行会决定所有行的高度，因此各行应使用同一套设计。
5. **一个面板一个状态**：每个正在渲染的命令面板使用独立的 [`CommandState`]。

[Command]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.Command.html
[CommandState]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandState.html
[CommandGroup]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandGroup.html
[WindowExt::open_dialog]: https://docs.rs/gpui-component/latest/gpui_component/trait.WindowExt.html#tymethod.open_dialog
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html

---
title: MessageScroller
description: 支持尾部跟随、未读定位和稳定 prepend 的虚拟消息列表。
---

# MessageScroller

`MessageScroller` 将 GPUI 的可变高度虚拟 `ListState` 与会话专属的尾部跟随行为组合起来。消息数据、稳定消息 ID 与未读状态仍由应用持有。

## 导入

```rust
use gpui_component::message_scroller::{MessageScroller, MessageScrollerState};
```

## 创建状态

把 `MessageScrollerState` 作为 Entity，与应用的消息集合放在一起。当父 view 渲染 scroller，或读取 `is_scrolled_up()` 时，应观察该 Entity：

```rust
let scroller = cx.new(|cx| MessageScrollerState::new(messages.len(), cx));
cx.observe(&scroller, |_, _, cx| cx.notify()).detach();
```

该状态只安装一次延迟 scroll handler。GPUI 调用 handler 时仍持有内部 `ListState` 的可变借用，因此必须延迟 Entity 更新。

## 渲染消息

传入按 index 渲染 row 的闭包。GPUI 会虚拟化这些 row，因此无需再提供 Provider、Viewport、Content 或 Item 组件。

```rust
MessageScroller::new(
    "conversation",
    scroller.clone(),
    move |index, window, cx| render_message(&messages[index], window, cx),
)
.w_full()
.h(px(480.))
```

渲染出的 row 应使用由应用消息 ID 派生的稳定 element ID。`MessageScroller` 有意不保存 index 到 ID 的映射。

## 同步更新数据与列表

消息结构变化时，在同一个操作中更新应用数据和 scroller 状态：

```rust
messages.push(message);
scroller.update(cx, |state, cx| {
    state.append(1, cx);
});
cx.notify();
```

只有列表正在跟随尾部时，`append` 才会跟随新 row。用户向上滚动后，新消息会保留当前位置，并显示内置的“跳到最新”按钮。

加载更早记录时，先把记录插到开头，再调用 `prepend`。GPUI list splice 会重新定位当前 item 锚点，使可见内容保持稳定：

```rust
messages.splice(0..0, earlier_messages);
scroller.update(cx, |state, cx| {
    state.prepend(earlier_count, cx);
});
cx.notify();
```

其他增量结构变更可用 `splice`。`reset` 会安装一组新 row，并重新启用尾部跟随。row 内容高度变化但身份和数量不变时调用 `remeasure_items`；全局文字或宽度变化后调用 `remeasure`。

## 定位到未读

未读身份由应用负责。先把第一条未读消息 ID 解析为当前 index，再传给状态：

```rust
if let Some(index) = messages.iter().position(|message| message.id == first_unread_id) {
    scroller.update(cx, |state, cx| {
        state.scroll_to_unread(index, cx);
    });
}
```

这会把未读 row 作为 viewport 锚点，并暂停尾部跟随。其下方有足够内容时，row 会出现在 viewport 起始位置；靠近末尾时，GPUI 会限制在可用滚动范围内。正常滚动到末尾会重新启用尾部跟随；`scroll_to_end` 和内置跳转按钮会显式重新启用。

## 滚动状态

- `item_count()` 返回当前虚拟 row 数量。
- `is_scrolled_up()` 表示 viewport 已离开尾部且当前不在末尾。
- `is_following_tail()` 表示是否会继续跟随新增的尾部内容。

这些 reader 会直接查询 `ListState`，不公开缓存的可见范围，因此滚轮滚动、scrollbar 拖动、结构更新和程序化定位都使用同一个事实来源。拖动内置 scrollbar 时，它会重绘所属 view；延迟状态通知负责 GPUI list scroll event，同时避免重复借用 `ListState`。

## 样式与控件

`MessageScroller` 在 root 上实现了 `Styled`。`with_content_style(...)` 用于调整内部 scrollbar viewport，`with_list_style(...)` 会在默认 padding 与 gap 之后调整 GPUI list。

使用 `.scrollbar(false)` 隐藏内置 scrollbar。当应用需要根据 `is_scrolled_up()` 与 `scroll_to_end()` 自行组合 Button 时，可使用 `.jump_button(false)`。内置按钮直接复用现有 `Button`，`with_jump_button_label(...)` 可设置应用本地化文本。

## 组件边界

GPUI 版本有意省略 React primitive 中的 Provider、Viewport、Content、Item 和 Button 导出：

- `Entity<MessageScrollerState>` 提供状态所有权与通知，无需 React Context。
- GPUI `list(...)` 已经负责 viewport、虚拟内容、item 测量和滚动锚点。
- index renderer 已经是 item 边界，再增加 `MessageScrollerItem` 只会包装任意内容。
- 跳转操作复用现有 `Button`；应用可以关闭默认按钮并自行组合。
- 消息 ID 与未读 ID 的类型、持久化规则属于业务域，因此继续由应用持有。

公开 API 由此只保留 GPUI 尚未直接提供的行为：尾部跟随协调、安全结构更新、滚动状态报告，以及可选的跳转入口。

## API 参考

- [MessageScroller]
- [MessageScrollerState]

[MessageScroller]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScroller.html
[MessageScrollerState]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScrollerState.html

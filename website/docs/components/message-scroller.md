---
title: MessageScroller
description: A virtualized message list with tail following, unread navigation, and stable prepend behavior.
---

# MessageScroller

`MessageScroller` combines GPUI's variable-height virtual `ListState` with conversation-specific tail-follow behavior. The application continues to own message data, stable message IDs, and unread state.

## Import

```rust
use gpui_component::message_scroller::{MessageScroller, MessageScrollerState};
```

## Create the state

Keep `MessageScrollerState` as an entity beside the application's message collection. Observe it when the parent view renders the scroller or reads `is_scrolled_up()`:

```rust
let scroller = cx.new(|cx| MessageScrollerState::new(messages.len(), cx));
cx.observe(&scroller, |_, _, cx| cx.notify()).detach();
```

The state installs one deferred scroll handler. Deferral is required because GPUI invokes the handler while its internal `ListState` is mutably borrowed.

## Render messages

Pass an indexed row renderer. GPUI virtualizes the rows, so a separate Provider, Viewport, Content, or Item component is unnecessary.

```rust
MessageScroller::new(
    "conversation",
    scroller.clone(),
    move |index, window, cx| render_message(&messages[index], window, cx),
)
.w_full()
.h(px(480.))
```

Give the rendered row a stable element ID derived from the application's message ID. `MessageScroller` deliberately does not keep an index-to-ID map.

## Update the data and list together

When message structure changes, update the application data and the scroller state in the same operation:

```rust
messages.push(message);
scroller.update(cx, |state, cx| {
    state.append(1, cx);
});
cx.notify();
```

`append` follows new rows only while the list is already following its tail. Once the user scrolls upward, new messages preserve the current position and the built-in jump button appears.

For earlier history, insert the records at the start and call `prepend`. GPUI's list splice rebases the current item anchor so the visible content remains stable:

```rust
messages.splice(0..0, earlier_messages);
scroller.update(cx, |state, cx| {
    state.prepend(earlier_count, cx);
});
cx.notify();
```

Use `splice` for other incremental structural changes. `reset` installs a new row set and re-engages tail following. Call `remeasure_items` when row content changes height without changing its identity or count; call `remeasure` after a global typography or width change.

## Scroll to unread

Unread identity belongs to the application. Resolve the first unread message ID to its current index, then pass that index to the state:

```rust
if let Some(index) = messages.iter().position(|message| message.id == first_unread_id) {
    scroller.update(cx, |state, cx| {
        state.scroll_to_unread(index, cx);
    });
}
```

This uses the unread row as the viewport anchor and pauses tail following. When enough content remains below it, the row appears at the viewport start; near the end, GPUI clamps to the available scroll extent. Reaching the end through normal scrolling re-engages tail following; `scroll_to_end` and the built-in jump button re-engage it explicitly.

## Scroll state

- `item_count()` returns the current virtual row count.
- `is_scrolled_up()` reports that the viewport has left the tail and is not at the end.
- `is_following_tail()` reports whether new tail content will be followed.

These readers query `ListState` directly instead of exposing a cached visible range. That keeps wheel scrolling, scrollbar dragging, structural updates, and programmatic navigation on one source of truth. The built-in scrollbar redraws its owning view while it is dragged; the deferred state notification covers GPUI list scroll events without re-borrowing `ListState`.

## Styling and controls

`MessageScroller` implements `Styled` for the root. `with_content_style(...)` refines the internal scrollbar viewport and `with_list_style(...)` refines the GPUI list after the default padding and gap.

Use `.scrollbar(false)` to hide the built-in scrollbar. Use `.jump_button(false)` when the application needs to compose its own Button from `is_scrolled_up()` and `scroll_to_end()`. The built-in button is an existing `Button`, and `with_jump_button_label(...)` allows application-localized text.

## Component boundaries

The GPUI version intentionally omits the React primitive's Provider, Viewport, Content, Item, and Button exports:

- `Entity<MessageScrollerState>` provides state ownership and notifications without React Context.
- GPUI `list(...)` already owns the viewport, virtual content, item measurement, and scroll anchor.
- The indexed renderer is the item boundary; another `MessageScrollerItem` would only wrap arbitrary content.
- Existing `Button` supplies the jump action. Applications can disable the default and compose their own.
- Message IDs and unread IDs remain application data because their type and persistence rules are domain-specific.

This keeps the public API focused on behavior that GPUI does not already provide: tail-follow coordination, safe structural updates, scroll-state reporting, and the optional jump affordance.

## API reference

- [MessageScroller]
- [MessageScrollerState]

[MessageScroller]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScroller.html
[MessageScrollerState]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScrollerState.html

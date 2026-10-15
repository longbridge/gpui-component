---
title: Textarea
description: Multi-line text input with fixed rows, soft wrapping, and auto-grow.
---

# Textarea

`Textarea` is the styled control for ordinary multi-line text. Use
[`Input`](./input.md) for a single line and [`Editor`](./editor.md) for source
code.

## Import

```rust
use gpui_kit::component::input::{Textarea, TextareaState};
```

## Basic usage

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(5)
        .placeholder("Notes")
});

Textarea::new(&notes)
```

## Auto-grow

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(2, 8)
        .placeholder("Write a message")
});

Textarea::new(&message)
```

The control grows until `max_rows`; overflowing content then scrolls.
Auto-grow sets the control's minimum height from its content (within those
limits). Use application-owned preferred height instead when a surrounding
flex layout must be allowed to shrink below that preferred height.

### Application-owned preferred height

`TextareaState::visual_row_count(&self) -> usize` reads the existing cached
content row count, including soft-wrapped continuations and explicit line
breaks. It is an O(1) query: it does not shape text, allocate, or update layout.
The count is independent of `rows`, auto-grow limits, and the visible viewport;
an empty laid-out textarea has one row.

The cache is populated during textarea layout. Before the first layout it can
be zero; after a width or font change it reflects the previous layout until
the textarea is laid out again. Text edits use the currently cached wrapping
parameters. Reading the getter does not force a layout or notify the parent.
A host that derives its height from the count must consume the refreshed cache
on a subsequent render; it must not assume the new width's count is available
while constructing that same frame.

For example, keep the state at `.rows(1)`, clamp the cached count to `3..=12`,
and translate it to a preferred height using the host's line height and the
control's vertical padding. Give the surrounding flex item that preferred
height with `.min_h(px(0.)).flex_shrink(1.)`, and render
`Textarea::new(&message).h(relative(1.))` inside it. The host can then shrink
below three rows when space is limited, while the textarea retains its own
internal scrolling. This is a host layout policy, not a second text-wrapping
implementation.

## Scrolling

A scroll over a textarea whose text can scroll stays in the textarea, even at
the end of the text: it does not scroll the page around it. A textarea whose
text fits leaves the scroll to the page.

## Scroll bounce

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(10)
        .scroll_bounce(true)
});
```

With `scroll_bounce(true)`, text taller than the textarea stretches past its
top or bottom when precise scrolling from a trackpad or Magic Mouse, or its
momentum, runs into the end, then springs back. Ordinary mouse wheels scroll
by lines and do not bounce. The frame stays put. It is on by default on iOS, Android and macOS, whose own scroll views bounce, and off elsewhere. When the system asks for reduced motion, the text
stops at its ends without stretching.

## Value and events

```rust
let value = notes.read(cx).value();

notes.update(cx, |state, cx| {
    state.set_value("Updated notes", window, cx);
});

cx.subscribe(&notes, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.notes = state.read(cx).value();
        cx.notify();
    }
});
```

`insert`, `replace`, `cursor_position`, `soft_wrap`, `searchable`, and
`submit_on_enter` are available on `TextareaState`.

## Sizes

```rust
Textarea::new(&notes).large()
Textarea::new(&notes) // medium (default)
Textarea::new(&notes).small()
```

The size changes the text size and the padding around the text together. It does
not set the height: use `h` for a fixed height, and let `rows` or `auto_grow`
decide the height of a growing textarea.

## Appearance

```rust
Textarea::new(&notes)
    .h(px(160.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("Notes")
```

Unlike `disabled`, a read-only textarea keeps the normal appearance and still
can be focused, selected and copied, it only rejects the changes made by the
user.

`Textarea` deliberately does not expose Input-only adornments such as `prefix`,
`suffix`, mask toggle, or the clear button. Compose related actions beside the
textarea.

## Atomic inline tokens

Use tokens to include mentions, file references or commands in a multi-line
message. Insert a token with `TextareaState::replace_with_token`, or restore a
saved draft with `set_value`. The default label is ready to use; add a renderer
when you want an icon or other custom content:

```rust
Textarea::new(&state)
    .token(|token, _, _| InputToken::new(token).icon(IconName::File))
```

Import `InputToken` from `gpui_kit::component::input` and `IconName` from
`gpui_kit::component`. A token wraps onto the next line as a whole, and auto-grow adjusts the textarea
height to fit. Put line breaks in the text between tokens. Use `on_token_hover`
to show a tooltip or preview without selecting or editing; disabled tokens never
report hover entry; disabling a hovered token sends its exit. See
[Input: atomic inline tokens](./input.md#atomic-inline-tokens) for editing,
activation, draft persistence, mode restrictions and JavaScript APIs.

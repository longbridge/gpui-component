---
title: Command
description: A command palette — a filtered list of commands and quick actions.
---

# Command

A command palette is a filtered list of commands with groups, shortcut hints,
and keyboard navigation. Use it inline or compose it into an existing dialog
for a `⌘K`-style menu. The list is virtualized, so large palettes render only
their visible rows.

## Import

```rust
use gpui_component::command::{
    Command, CommandEntry, CommandEvent, CommandGroup, CommandItem, CommandState,
};
```

## Composition

Commands live in a [`CommandState`] entity; [`Command`] renders that state.

```text
CommandState
├── CommandItem                 // ungrouped
├── CommandGroup
│   ├── CommandItem
│   └── CommandItem
├── CommandEntry::Separator
└── CommandGroup
    ├── CommandItem
    └── CommandItem
```

## Usage

### Inline

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

### Quick Actions Without Search

Disable search for a compact action palette. It has no search field, does not
filter entries, and `state.focus(window, cx)` focuses the Command frame so its
arrow, Enter, and Escape actions remain available.

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

With the default `.searchable(true)`, `state.focus(window, cx)` and
[`Focusable::focus_handle`] target the search input instead.

### In a Dialog

Compose the palette with the existing [`WindowExt::open_dialog`] API. Subscribe
to [`CommandEvent`] and close the dialog on `Confirm` or `Cancel`; Command does
not provide a dialog-specific API. `header` renders above the optional search
field and list, while `footer` renders below the list.

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

### Reacting to a Choice

Either give an item a handler:

```rust
CommandItem::new("profile")
    .label("Profile")
    .on_select(|window, cx| {
        window.push_notification("Opening profile", cx);
    })
```

or subscribe to the state:

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

### Changing Commands

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

## Searching

By default, `CommandItem::matches(&self, query: &str) -> bool` uses a
case-insensitive substring match against the item's label, value, and
keywords. Empty queries match every item. A group whose items all filter out
hides its heading; a separator left leading, trailing, or adjacent to another
separator is omitted.

```rust
CommandItem::new("profile")
    .label("Profile")
    .keywords(["account", "user"])
```

Use a custom filter when the application's match policy differs. This stock
search checks the symbol first, then the company name:

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

The custom predicate runs only for non-empty queries while search is enabled;
otherwise every item remains visible. For remote search, listen for
`CommandEvent::Query`, replace the entries with returned results, and keep the
query terms in the item value or label if local filtering should still apply.
Use `set_loading` while waiting so the empty message is suppressed.

## Custom Rows and Virtualization

`CommandItem::element` replaces an item's icon and label content. Command
measures each flattened row when rebuilding its `v_virtual_list` sizes, so
custom rows may have independent intrinsic heights. Build rows for their
available list width and keep their rendered content stable until the state is
updated, because the saved sizes are reused by the virtual list.

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

| Method | Signature and description |
| --- | --- |
| `new` | `new(&Entity<CommandState>) -> Command` renders a state. |
| `placeholder` | `placeholder(impl Into<SharedString>) -> Self` sets the search-field placeholder. |
| `empty` | `empty(impl Into<SharedString>) -> Self` sets the message for no matches. |
| `max_h` | `max_h(impl Into<DefiniteLength>) -> Self` sets the list maximum. Default: `18.75rem` (300px). |
| `bordered` | `bordered(bool) -> Self` draws the surrounding border and rounding. Default: `true`. |
| `header` | `header<F, E>(F) -> Self`, where `F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static` and `E: IntoElement`; renders above search and list. |
| `footer` | `footer<F, E>(F) -> Self`, with the same callback bounds; renders below the list. |

`Command` implements [`Styled`], so `w`, `max_w`, `bg`, and other styles apply
to the palette frame.

## CommandItem

| Method | Description |
| --- | --- |
| `new(value)` | The value identifies the item and is its label until `label` sets one. |
| `label` | Sets the visible label. |
| `icon` | Sets the leading icon. |
| `shortcut` | Sets a trailing shortcut hint; the application binds the keystroke. |
| `checked` | Draws a trailing check; `shortcut` uses that position instead. |
| `keywords` | Adds default-match terms. |
| `disabled` | Makes the item non-interactive and skips it during keyboard navigation. |
| `element` | Replaces the row content with a custom element. |
| `on_select` | Runs on click or Enter confirmation. |

## CommandState

| Method | Signature and description |
| --- | --- |
| `new` | `new(&mut Window, &mut Context<Self>) -> Self` creates an empty, searchable palette. |
| `item` / `group` / `separator` | Add an ungrouped item, a group, or a divider. |
| `searchable` | `searchable(bool) -> Self` enables local filtering and the search field. Default: `true`. |
| `filter` | `filter<F>(F) -> Self`, where `F: Fn(&CommandItem, &str) -> bool + 'static`, replaces default matching. |
| `set_entries` | Replaces every entry. |
| `query` / `set_query` | Read or replace the search query. |
| `selected_index` / `selected_value` | Read the highlighted matching item. |
| `matched_count` | Returns the number of matching items. |
| `focus` | `focus(&self, &mut Window, &mut App)` focuses the input when searchable, otherwise the Command frame. |
| `set_loading` | Shows the search spinner and suppresses the empty message while loading. |

## Keyboard Shortcuts

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move the highlight, wrapping around and skipping disabled items. |
| `Enter` | Confirm the highlighted item. |
| `Escape` | Clear the query; when it is already empty, emit `Cancel`. |

## Best Practices

1. Group related commands and add keywords for alternate names.
2. Use `searchable(false)` for compact, keyboard-navigable quick actions.
3. Treat `shortcut` as a visual hint and bind the keystroke in the application.
4. Use slots for application-owned status and hints, not a Command-specific dialog layer.
5. Give each rendered palette its own [`CommandState`].

[Command]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.Command.html
[CommandState]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandState.html
[CommandGroup]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandGroup.html
[WindowExt::open_dialog]: https://docs.rs/gpui-component/latest/gpui_component/trait.WindowExt.html#tymethod.open_dialog
[Focusable::focus_handle]: https://docs.rs/gpui/latest/gpui/trait.Focusable.html#tymethod.focus_handle
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html

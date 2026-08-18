---
title: Command
description: A command palette — a search field over a filtered list of commands and quick actions.
---

# Command

A command palette: a search field over a filtered list of commands, with groups, shortcut hints and keyboard navigation. Use it inline, or present it in a dialog as a `⌘K`-style menu.

The list is virtualized, so a palette with thousands of commands renders only the rows that are on screen.

## Import

```rust
use gpui_component::command::{
    Command, CommandEntry, CommandEvent, CommandGroup, CommandItem, CommandState,
};
```

## Composition

The commands live in a [`CommandState`] entity; the [`Command`] element renders it.

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

### In a Dialog

[`WindowExt::open_command_dialog`] presents the same palette in a dialog: it drops the border, hides the close button, moves focus to the search field, and closes the dialog when an item is confirmed.

```rust
use gpui_component::WindowExt as _;

window.open_command_dialog(&self.command_state, cx, |command, _, _| {
    command.placeholder("Type a command or search...")
});
```

### Reacting to a Choice

Either give the item a handler:

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

### Changing the Commands

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

### As a Search Panel

Turn `filterable` off when the entries already are the results — a remote search, for instance. The palette then shows everything it was given, and the query arrives as `CommandEvent::Query`:

```rust
Command::new(&self.search)
    .filterable(false)
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
    // Held on the view, so the next query cancels this one.
    self._search_task = Some(cx.spawn_in(window, async move |_, cx| {
        let results = fetch(query).await;

        _ = state.update_in(cx, |state, window, cx| {
            state.set_loading(false, window, cx);
            state.set_entries(results.into_iter().map(CommandEntry::Item), cx);
        });
    }));
}
```

`set_loading` spins the search field and holds back the empty message until the answer arrives, so an in-flight search never reads as "no results".

Rows can carry a result design of their own — `CommandItem::element` builds the row, and the palette measures the first one to size all of them, so a two-line result row stays virtualized:

```rust
CommandItem::new(symbol).element(move |_, cx| {
    h_flex()
        .w_full()
        .justify_between()
        .child(v_flex().child(symbol).child(name))
        .child(v_flex().items_end().child(price).child(change))
})
```

## Searching

A command matches when the query is a case-insensitive substring of its label, its value, or any of its keywords:

```rust
CommandItem::new("profile")
    .label("Profile")
    .keywords(["account", "user"])
```

A group whose items are all filtered out hides its heading, and a separator that ends up leading, trailing, or next to another separator is not drawn.

## Command

| Method          | Description                                                   |
| --------------- | ------------------------------------------------------------- |
| `new(&state)`   | Render the palette held by the given [`CommandState`].         |
| `placeholder`   | The placeholder of the search field.                           |
| `empty`         | The message shown when no command matches the query.           |
| `max_h`         | The max height of the list. Default: `18.75rem` (300px).       |
| `bordered`      | Draw the surrounding border and rounding. Default: `true`.     |
| `close_on_confirm` | Close the hosting dialog when an item is confirmed. Default: `false`, and `true` under `open_command_dialog`. |
| `filterable`    | Filter the entries by the query. Default: `true`; turn off for a remote search. |

`Command` implements [`Styled`], so `w`, `max_w`, `bg` and the rest apply to the palette's frame.

## CommandItem

| Method       | Description                                                                       |
| ------------ | --------------------------------------------------------------------------------- |
| `new(value)` | The value identifies the item and is the label until `label` sets one.             |
| `label`      | The text shown in the row.                                                         |
| `icon`       | The leading icon.                                                                  |
| `shortcut`   | The trailing shortcut hint. Binding the keystroke is the application's job.         |
| `checked`    | Draw a check at the right end of the row. A `shortcut` takes that slot instead.     |
| `keywords`   | Extra terms the search matches against.                                            |
| `disabled`   | Render the item as non-interactive and skip it during keyboard navigation.          |
| `element`    | Replace the row content with a custom element.                                      |
| `on_select`  | Run when the item is clicked or confirmed with Enter.                               |

The palette measures its first row and draws every item at that height, which is what keeps the list virtualized. A design built with `element` can be as tall as it likes, as long as every row is the same height.

## CommandState

| Method            | Description                                                    |
| ----------------- | -------------------------------------------------------------- |
| `new(window, cx)` | Create an empty palette.                                        |
| `item`            | Add an ungrouped item.                                          |
| `group`           | Add a [`CommandGroup`].                                         |
| `separator`       | Add a divider between the previous and the next group.          |
| `set_entries`     | Replace every entry.                                            |
| `query`           | The current search query.                                       |
| `set_query`       | Replace the search query.                                       |
| `selected_index`  | The index of the highlighted item, among the matching items.    |
| `selected_value`  | The value of the highlighted item.                              |
| `matched_count`   | The number of items matching the current query.                 |
| `focus`           | Move focus to the search field.                                 |
| `set_loading`     | Spin the search field and hold back the empty message.          |

## Keyboard Shortcuts

| Key       | Action                                                       |
| --------- | ------------------------------------------------------------ |
| `↑` / `↓` | Move the highlight, wrapping around and skipping disabled items |
| `Enter`   | Confirm the highlighted item                                  |
| `Escape`  | Clear the query; when it is already empty, leave the palette  |

## Best Practices

1. **Group Related Commands**: Give each group a heading and separate the groups.
2. **Add Keywords**: A command people search for by another name needs `keywords`.
3. **Shortcut Hints Only**: `shortcut` renders a hint — bind the keystroke yourself.
4. **Keep Rows Uniform**: A custom `element` row sets the height for every row, so give them all the same design.
5. **One State Per Palette**: Reuse the same [`CommandState`] for the inline and dialog forms of the same menu.

[Command]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.Command.html
[CommandState]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandState.html
[CommandGroup]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandGroup.html
[WindowExt::open_command_dialog]: https://docs.rs/gpui-component/latest/gpui_component/trait.WindowExt.html#tymethod.open_command_dialog
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html

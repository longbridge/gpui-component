# Command API Redesign

## Goal

Make `Command` follow GPUI Component ownership conventions without exposing the
full generic `ListDelegate` API. `CommandState` will contain interaction state;
`Command` will receive the entries, rendering policy, and callbacks. Keep
`v_virtual_list`, independently measured row heights, grouping, filtering,
Action dispatch, and dialog-friendly composition.

This is an intentional breaking redesign of the Command API currently on the
feature branch.

## Responsibilities

- `CommandState` owns query input, focus, selection, scrolling, flattened row
  indices, measurement caches, and loading state.
- `Command` owns the entries and construction-time behavior installed into the
  state for rendering.
- `CommandItem` describes one lazily renderable command row.
- `CommandGroup` groups `CommandItem`s under an optional heading.
- `gpui::Action` is the executable behavior and the source of displayed
  keybindings.
- Command callbacks notify the palette owner directly; `CommandEvent` and
  `EventEmitter<CommandEvent>` are removed.

## Public Construction API

Entries move from `CommandState` to `Command`:

```rust
let state = cx.new(|cx| CommandState::new(window, cx));

Command::new(&state)
    .searchable(true)
    .filter(|item, query| item.matches(query))
    .item(
        CommandItem::new("open-file")
            .label("Open File")
            .icon(IconName::Folder)
            .action(Box::new(OpenFile)),
    )
    .group(
        CommandGroup::new("Settings")
            .item(CommandItem::new("profile").label("Profile"))
            .item(CommandItem::new("billing").label("Billing")),
    )
    .separator()
    .header(|state, window, cx| render_header(state, window, cx))
    .footer(|state, window, cx| render_footer(state, window, cx))
    .on_query(|query, window, cx| update_search(query, window, cx))
    .on_select(|value, window, cx| preview(value, window, cx))
    .on_confirm(|value, window, cx| finish(value, window, cx))
    .on_cancel(|window, cx| cleanup(window, cx));
```

`CommandState::item`, `group`, `separator`, `searchable`, `filter`, and
`set_entries` are removed. Dynamic or asynchronous data belongs to the owner
view; the owner reconstructs the `Command` with its current entries when it
renders.

`CommandState` retains imperative interaction methods such as `query`,
`set_query`, `focus`, selection access, and loading state.

## Lazy Item Content

The default item renderer uses the item's label, icon, checked state, disabled
state, and Action binding. A custom item uses a repeatable lazy child factory:

```rust
CommandItem::new("stock:AAPL").child(|_, cx| {
    h_flex()
        .justify_between()
        .child("AAPL")
        .child("$245.18")
})
```

This `child` is an inherent CommandItem builder accepting
`Fn(&mut Window, &mut App) -> E`, not GPUI's eager
`ParentElement::child(AnyElement)`. It replaces `CommandItem::element`.
The factory must be side-effect-free because measurement, viewport entry, and
typography or width invalidation may call it more than once.

## Actions and Keybindings

`CommandItem::shortcut(SharedString)` and `CommandItem::on_select` are removed.
An optional Action is configured with:

```rust
CommandItem::new("open-file").action(Box::new(OpenFile))
```

The default row resolves its visible keybinding from the real GPUI binding,
first in the Command focus scope and then at application scope, using the same
`Kbd::binding_for_action_in` / `Kbd::binding_for_action` pattern as PopupMenu.
If no binding exists, no key hint is rendered.

Clicking or confirming an item dispatches a boxed clone of its Action and then
calls the palette-level `on_confirm` callback. Items without Actions still call
`on_confirm`, identified by their value. Highlight changes never dispatch the
Action.

## Callback API

Callbacks are optional and stored as private, type-erased `Rc<dyn Fn...>`
values in the Command render model:

```rust
on_query(Fn(&str, &mut Window, &mut App))
on_select(Fn(&SharedString, &mut Window, &mut App))
on_confirm(Fn(&SharedString, &mut Window, &mut App))
on_cancel(Fn(&mut Window, &mut App))
```

Semantics:

- `on_query` runs only when a searchable Command's query actually changes.
- `on_select` runs when keyboard or pointer highlighting changes.
- Confirm dispatches the item Action first, then invokes `on_confirm`.
- Cancel invokes `on_cancel`, then propagates the Cancel action. A dialog owner
  should let the hosting Dialog perform dismissal instead of closing again in
  `on_cancel`.

## Internal Adapter and Virtualization

The public API does not expose a generic delegate. Internally, `Command` builds
a type-erased render model containing entries, filtering, callbacks, and visual
options, and installs it into `CommandState` during `RenderOnce`.

`CommandState` filters the model and flattens it into lightweight indices:

```rust
enum CommandRow {
    GroupHeading { entry_ix: usize },
    Item {
        entry_ix: usize,
        item_ix: usize,
        matched_ix: usize,
    },
    Separator,
}
```

`v_virtual_list` remains the rendering backend. Its visible-range callback asks
`CommandState::render_row(row_ix, ...)` to resolve an indexed item and invoke
the lazy content factory. The same render path is used by the row measurement
pass. Existing width, rem, typography, and outer-style invalidation behavior is
preserved.

The render model persists inside the state so query, selection, and scrolling
updates can redraw the virtual list without requiring the owner view to render
again. When a subsequent `Command` render installs a new model, state preserves
the selected value when it still exists, otherwise selects the first enabled
match. Row measurement is invalidated when the model changes.

## Groups, Separators, and Filtering

`CommandGroup::new(...).item(...).items(...)` remains the direct grouping API.
`Command::item`, `group`, and `separator` preserve intuitive composition while
moving structure out of `CommandState`.

The default filter continues to match value, label, and keywords without case
sensitivity. `Command::filter` overrides it. `searchable(false)` hides the
input, retains all entries, focuses the Command frame, and never invokes
`on_query`.

Filtering continues to remove empty groups and redundant leading, trailing, or
adjacent separators.

## Migration

All branch call sites migrate in the same change:

- Static Command stories construct items on `Command`.
- Stock search results live in `CommandStory` rather than `CommandState`.
- Story component and theme palettes store their entry collections in
  `StoryRoot` and pass them to `Command` when rendering the dialog.
- Event subscriptions are replaced by `Command::on_*` callbacks.
- Manually formatted shortcuts are replaced by Actions and resolved `Kbd`
  bindings.
- English and Chinese documentation use identical revised examples.

No compatibility shim is required because the Command component has not yet
landed on the base branch.

## Testing

Tests will cover:

1. Items, groups, and separators are configured on `Command`, not state.
2. Default and custom filtering operate on the installed render model.
3. Lazy child factories support repeated measurement and visible rendering.
4. Different-height rows retain independent `v_virtual_list` sizes.
5. Actions dispatch on click and Enter, and their actual binding renders with
   `Kbd`.
6. `on_query`, `on_select`, `on_confirm`, and `on_cancel` fire with the defined
   ordering and propagation behavior.
7. Reinstalling a dynamic model preserves selection by value when possible and
   invalidates measurements.
8. `searchable(false)`, disabled rows, grouping, dialog ownership, and virtual
   scrolling retain their existing regressions.
9. Story component/theme palettes and stock search continue to work with the
   simplified API.

## Performance

Filtering remains linear. Visible rows are still created lazily by
`v_virtual_list`; only measurement invalidation eagerly invokes row factories.
Installing an unchanged model should avoid unnecessary selection resets and
remeasurement. The implementation may use a private structural revision or
entry signature, but this mechanism is not part of the public API.

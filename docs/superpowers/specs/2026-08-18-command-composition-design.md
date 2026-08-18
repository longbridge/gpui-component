# Command Composition Design

## Goal

Extend the Command component with custom filtering, an optional search input,
header and footer content, and independently sized rows. Preserve the existing
GPUI Component split: `CommandState` owns interaction state and `Command`
renders and styles that state.

## Public API

`CommandState` gains two builder methods:

```rust
CommandState::new(window, cx)
    .searchable(true)
    .filter(|item, query| fuzzy_match(item.title(), query))
```

- `searchable(bool)` defaults to `true`. When false, Command hides its search
  input, does not filter entries, and does not emit query changes caused by a
  hidden input.
- `filter(F)` accepts `F: Fn(&CommandItem, &str) -> bool + 'static`. It replaces
  the default case-insensitive value, label, and keyword matching. The custom
  filter is ignored when `searchable` is false.

`Command` gains two presentation builders:

```rust
Command::new(&state)
    .header(|state, window, cx| render_header(state, window, cx))
    .footer(|state, window, cx| render_footer(state, window, cx))
```

Each callback returns an `IntoElement`. Header and footer are optional and sit
outside the scrolling list. They receive the current `CommandState`, allowing
status text to reflect the query, match count, selection, or loading state.
Their surrounding layout and decoration remain application-owned; Command does
not impose a special footer component or keybinding model.

## Internal Structure

Replace `v_virtual_list` and `VirtualListScrollHandle` with `gpui::list` and
`gpui::ListState`. The rendered hierarchy becomes:

```text
Command frame
├── header (optional)
├── search input (when searchable)
├── variable-height gpui::list
└── footer (optional)
```

The flattened `CommandRow` model remains: headings, items, and separators are
individual list rows. `gpui::list` measures each rendered row and caches its
height, so custom item elements may differ in height. Command no longer stores
`RowHeights`, `row_sizes`, or a measurement sample.

`CommandState` owns a `gpui::ListState`. When filtering or replacing entries
changes the flattened rows, it resets or splices the list state to the new row
count. Selection stores matched-item indices as today; the corresponding
`row_ix` is used to scroll the GPUI list when keyboard navigation moves beyond
the viewport.

## Filtering and State Changes

The default filter remains `CommandItem::matches`. A custom filter receives the
same trimmed query and each candidate item. Filtering still removes empty
groups and redundant separators.

Changing `searchable` or the custom filter after entity creation is not part of
this change. They are construction-time policies, consistent with the existing
builder API. Dynamic data continues through `set_entries`; query and loading
continue through `set_query` and `set_loading`.

When search is hidden, focus belongs to the Command frame rather than the
hidden input, so arrow, Enter, and Escape actions continue to work. The public
`focus` method selects the appropriate focus target.

## Rendering and Performance

`gpui::list` renders and measures visible variable-height rows, with overdraw
for smooth scrolling. It avoids the current extra root-layout measurement on
every invalidation and removes the requirement that all custom rows share the
first row's height.

Header and footer callbacks run when Command renders. They must follow normal
GPUI render-callback expectations and should not perform external side effects.
Filtering remains linear in the number of items; this design intentionally
does not change string normalization or matching allocation behavior.

## Compatibility

Existing Command construction remains valid. Defaults preserve the current
search input and matching behavior. Existing uniform rows render unchanged,
while custom rows gain independent heights.

No `CommandDialog`, `WindowExt::open_command_dialog`, `CommandFooter`, or
shadcn-style child primitive API is introduced.

## Testing

Add focused tests that verify:

1. A custom filter controls which items and groups remain.
2. `searchable(false)` renders no input, retains all entries, and supports
   keyboard selection through the Command focus handle.
3. Header and footer callbacks render outside the scrolling list and can read
   current state.
4. Two custom rows with different intrinsic heights receive different measured
   heights under `gpui::list`.
5. Filtering and `set_entries` reset the GPUI list row count and keyboard
   navigation scrolls variable-height rows into view.
6. Existing grouping, disabled-item, confirmation, empty-state, and async
   loading behavior continues to pass.

Update the Command story and both English and Chinese component documentation
with examples for all four capabilities.

---
title: Text Selection
description: Add native window-level text selection to plain text and custom GPUI renderers.
order: 3
---

# Text Selection

`gpui-base` provides window-level text selection for rendered text. It coordinates pointer gestures, Shift-click extension, cross-region selection, copying, scrolling, modal scopes, and multi-window lifetime without prescribing how text is laid out or highlighted.

Use it when you render text yourself with `StyledText`, `TextLayout`, a virtualized document, or another custom renderer. If you use `gpui-component::TextView`, call `.selectable(true)` instead; its Markdown and HTML adapter is already connected to the same base selection engine.

## How it fits together

A selectable window has three parts:

1. One `TextSelection` element owns the selection state and window pointer handlers.
2. Each independently selectable document or label owns a stable `TextSelectionRegion`.
3. During rendering, the region reports its current geometry and projects the selection onto its laid-out text runs.

The selection state belongs to the retained `TextSelection` element. Regions and callbacks never receive or own that internal state.

## Install the window element

Add one `TextSelection` as the first child of the window root:

```rust
use gpui::prelude::*;
use gpui::{Context, Render, Window};
use gpui_base::TextSelection;

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(TextSelection)
            .child(self.content.clone())
    }
}
```

The unit value is sufficient when regions register later during prepaint or paint. If the root sets a selection scope or registers a region during the same render, prebind the element first:

```rust
use gpui_base::{SelectionScopeId, TextSelection, WindowTextSelection as _};

let text_selection = TextSelection::new(window, cx);
window.set_text_selection_scope(SelectionScopeId::new(7), cx);

div()
    .child(text_selection)
    .child(content)
```

Keep the element first and render only one logical selection owner per window. Calls through `WindowTextSelection` are safe no-ops before the element is retained.

`gpui_component::Root` already installs this element. Applications using that root do not add another one.

## Create a stable region

Create one region for the semantic lifetime of the text. Do not create a new region every frame.

```rust
use gpui::Context;
use gpui_base::TextSelectionRegion;

struct DocumentView {
    selection: TextSelectionRegion,
}

impl DocumentView {
    fn new(cx: &mut Context<Self>) -> Self {
        let selection = TextSelectionRegion::new("", cx);

        // Repaint when the window selection projected onto this region changes.
        selection.on_selection(|_, cx| cx.refresh_windows(), cx);

        Self { selection }
    }
}
```

The initial string is the region's fallback copy value. A plain renderer normally lets `project_selection_runs` maintain the copied substring automatically. Use `set_selected_text` when the fallback is produced elsewhere.

## Register geometry during prepaint

Register the region once per rendered frame, after its bounds and hitbox are known:

```rust
use gpui::{Bounds, Hitbox, Pixels, Window};
use gpui_base::{SelectionRegionFrame, WindowTextSelection as _};

fn register_selection(
    selection: &TextSelectionRegion,
    hitbox: Hitbox,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    window.register_text_selection_region(
        selection.clone(),
        SelectionRegionFrame::new(hitbox, bounds)
            .with_document_order(0)
            .with_text_bounds(vec![bounds]),
        cx,
    );
}
```

- `bounds` is the region's content viewport in window coordinates.
- `text_bounds` contains the visible glyph-bearing areas. Blank-only drags do not start a text selection.
- `document_order` provides stable ordering between regions for cross-region selection and copy. Do not derive semantic order from a `HashMap` or accidental paint order.
- `with_scroll_offset` maps window points into scrolled content coordinates.
- `with_scope` assigns an explicit opaque scope. A surrounding `TextSelection::scope` marker overrides it while that subtree renders.

Regions not registered in the current frame are pruned automatically.

## Project selection onto text runs

In paint, describe each laid-out run with the exact text used to create its `TextLayout`. The returned state contains a UTF-8-safe byte range:

```rust
use gpui::{Bounds, Pixels, SharedString, TextLayout};
use gpui_base::SelectionRunFrame;

fn selected_range(
    selection: &TextSelectionRegion,
    text: SharedString,
    layout: TextLayout,
    bounds: Bounds<Pixels>,
    cx: &mut gpui::App,
) -> Option<std::ops::Range<usize>> {
    selection
        .project_selection_runs(
            &[SelectionRunFrame::new(0, text, layout, bounds)],
            cx,
        )
        .into_iter()
        .next()
        .and_then(|state| state.byte_range().cloned())
}
```

Paint the returned range behind the glyphs, then paint the text normally. Wrapped selections need three kinds of highlight geometry: the remainder of the first line, full-width middle lines, and the prefix of the last line.

For multiple runs, give each run a stable logical `order`. Input order is preserved in the returned states so each state can be paired with its original layout; logical order is used when composing copied text.

See the complete runnable [`selectable_text` example](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/selectable_text.rs) for an `Element` that registers a `StyledText`, projects its run, and paints selection quads.

## Query and control the window selection

Import `WindowTextSelection` to read or mutate the window selection:

```rust
use gpui_base::WindowTextSelection as _;

let has_selection = window.has_text_selection(cx);
let text = window.selected_text(cx);

window.end_text_selection(cx);   // End a drag, preserving its range.
window.clear_text_selection(cx); // Clear window and renderer-local ranges.
```

`selected_text` invokes renderer copy adapters only after the window and region state leases have been released, so an adapter may safely read or update selection state.

## Advanced renderer adapters

Plain text usually needs only `on_selection` and `project_selection_runs`. Rich or virtualized renderers can configure additional behavior directly on the region handle:

| Method | Use |
| --- | --- |
| `copy_with` | Export source text or include virtualized content that is not currently painted. |
| `on_virtual_key` | Attach a stable renderer-defined block key to an endpoint. |
| `on_focus` | Focus the renderer when a drag begins inside it. |
| `on_auto_scroll` | Receive edge-scroll deltas while dragging. |
| `on_clear` | Clear renderer-local word, paragraph, or select-all state. |
| `set_local_selection` | Report renderer-local selection such as select-all or double-click selection. |

Callbacks are invoked outside selection-state leases. They may update the renderer or query `WindowTextSelection` without causing a reentrant entity borrow.

For a virtualized document, use the snapshot passed to `on_selection` together with `SelectionSnapshot::region_coverage()` and endpoint `virtual_key()` values. Coverage distinguishes a bounded region from a region selected from its start, to its end, or in full, allowing copy adapters to include unpainted blocks.

## Isolate modal content with scopes

Only regions in the active `SelectionScopeId` participate. Set the active window scope, then mark the corresponding rendered subtree:

```rust
use gpui_base::{SelectionScopeId, TextSelection, WindowTextSelection as _};

let dialog_scope = SelectionScopeId::new(42);
window.set_text_selection_scope(dialog_scope, cx);

let dialog = TextSelection::scope(dialog_scope, dialog_content);
```

Scope stacks are isolated per window and are cleaned up even if a scoped subtree panics while rendering. Changing the active scope clears the previous selection atomically.

## Use TextView from gpui-component

`TextView` remains a rich-text renderer in `gpui-component`; Markdown and HTML parsing do not move into base. Its adapter registers regions and runs, exports plain text or source Markdown, resolves virtual blocks, focuses, and auto-scrolls through the base interface.

```rust
use gpui_component::text::{SelectionFormat, TextView};

TextView::markdown("preview", markdown_source)
    .selectable(true)
    .selection_format(SelectionFormat::Source)
    .scrollable(true)
```

When using `gpui_component::Root`, selection across a plain base region and a `TextView` shares the same window state and copy order.

## Migrating existing component code

The selection methods on `gpui_component::WindowExt` and `Root::clear_text_selection` remain as deprecated forwarding shims. New code should import `gpui_base::WindowTextSelection` and call the same window methods directly:

```rust
use gpui_base::WindowTextSelection as _;

window.clear_text_selection(cx);
```

The compatibility methods do not own a second selection state, so migration can be incremental.

## Integration checklist

- Retain one `TextSelection` element as the first child of each custom window root.
- Keep every `TextSelectionRegion` stable across renders.
- Register current geometry every rendered frame.
- Use explicit document order and window-local scopes.
- Pass the exact UTF-8 text used by each `TextLayout`.
- Paint highlights before glyphs.
- Keep parser, source export, and virtual-document knowledge in the renderer adapter.

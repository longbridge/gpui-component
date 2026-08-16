# Text Selection Base Architecture Assessment

Date: 2026-08-15

## Conclusion

The text-selection mechanism belongs in `gpui-base`, but the existing
`window_selection.rs` should not simply be moved there. It should become an
independent, window-level deep module. `TextView`, `Label`, and custom Markdown
renderers then connect through adapters.

`TextView`, `TextViewState`, Markdown and HTML parsing, source reconstruction,
virtual document blocks, links, code blocks, and presentation styles remain in
`gpui-component`.

## Options considered

| Option | Shape | Assessment |
| --- | --- | --- |
| Move only anchor/cursor state | Base owns drag state; every renderer implements hit testing, ranges, highlighting, and copying | Too shallow. The hard behavior is duplicated by Label and custom renderers. |
| Generic participant coordinator | Base owns events, scopes, cross-participant selection, and lifecycle; renderers map geometry to text | Viable and compact, but a plain Label still implements substantial selection machinery. |
| Handle and text-run engine | Base also maps ordinary laid-out text runs to byte ranges and plain-text copy; TextView adds an advanced adapter | Recommended. It gives the module depth while keeping the common path simple. |

## Recommended module

Add `gpui_base::text_selection` with two layers.

### Window-level coordination

The first layer owns the selection session and window interaction:

```rust
TextSelection
TextSelectionScopeId
TextSelectionHandle
TextSelectionSnapshot
```

It hides:

- ordinary click, Shift+click, and drag lifecycle;
- stable anchor and moving cursor semantics;
- reversed selection when the cursor crosses the anchor;
- cross-participant selection;
- capture/bubble ordering and selection suppression;
- active selection scope;
- content-coordinate endpoints and proxy anchors;
- focus and auto-scroll orchestration;
- clearing, invalidation, and repainting;
- dead-participant pruning;
- ordering and merging copied text.

The retained `TextSelection` element should own the per-window state. Any
application-global lookup must remain a weak locator and must not require
`window.root::<gpui_component::Root>()`.

### Plain text runs

The second layer supports normal laid-out text:

```rust
TextSelectionHandle
TextSelectionRegistration
TextSelectionRun
TextSelectionRunState
```

A custom renderer registers its selection handle and laid-out runs:

```rust
selection.register(
    TextSelectionRegistration::new(hitbox, bounds),
    window,
    cx,
);

let states = selection.ranges_for(
    &[TextSelectionRun::new(order, text, layout, bounds)],
    cx,
);

paint_selection(states[0].byte_range());
```

The base module can then centralize:

- `TextLayout` position to UTF-8 byte-range mapping;
- geometric selection across text runs;
- plain-text copying;
- logical ordering of runs and participants;
- reusable selection-quad helpers.

This allows the common Label interface to stay small:

```rust
Label::new("Status: Ready").selectable(true)
```

## The adapter seam

The external seam is between the window selection engine and selectable participants
and runs. It should not be a large `TextViewSelectionDelegate` trait.

Ordinary text needs only participant and run registration. Advanced renderers may add
optional adapters for:

- formatted or source-text export;
- virtual document blocks;
- focus ownership;
- scrolling and auto-scroll;
- layout revision compatibility.

The adapter presented to the coordinator should be type-erased internally so a
single registry can contain Label, TextView, and custom renderer state without
making `gpui-base` generic over their entity types.

## Interface invariants

- `TextSelectionScopeId` is opaque. Base does not know Dialog or Sheet semantics.
- A selection belongs to exactly one active scope.
- Participant handles stay stable for the semantic lifetime of their content.
- Endpoints use participant-relative content coordinates, never durable window
  coordinates.
- Runs use explicit logical order. HashMap iteration and screen `y/x` are not a
  document-order interface.
- Run ranges must end on UTF-8 boundaries.
- Registration is frame-based; dead or unpainted participants are pruned safely.
- Copy reads an immutable snapshot so adapters cannot re-enter a leased window state.
- Missing window state, dead participant, or unsupported advanced export is a safe no-op or
  explicit fallback, not a paint-time panic.
- The interface documents that selection and copy reflect the last painted
  frame.

## Backward compatibility

Behavioral compatibility is a release requirement for this migration. Existing
applications must not lose selection behavior or be forced to retain a selection element
manually. The old Root-bound selection entry points may, however, be deprecated
and removed through the normal breaking-change process once replacements exist
in `gpui-base`.

### Public interface compatibility

The following TextView-facing interfaces retain their current signatures and
semantics:

- `TextView::selectable(true)` and `TextViewState::set_selectable`;
- `TextView::selection_format` and `SelectionFormat::{Plain, Source}`;
- existing copy and select-all actions.

The existing selection methods on `gpui_component::WindowExt` may be deprecated:

- `selected_text`;
- `has_text_selection`;
- `clear_text_selection`;
- `end_text_selection`.

`Root::clear_text_selection`, which is currently public while the other Root
selection operations are crate-private, may be deprecated at the same time.
Their replacements are associated functions on `gpui_base::TextSelection`
rather than methods on the broad component-layer `WindowExt`.

During the compatibility window, the old methods delegate to the same base state
and carry `#[deprecated]` guidance pointing to the base functions. They do not retain
their own state or implementation. A later semver-breaking release can remove
them from `gpui-component`.

```rust
let text = gpui_base::TextSelection::selected_text(window, cx);
gpui_base::TextSelection::clear(window, cx);
```

No additional window extension trait is required, so callers can keep using
unrelated `gpui_component::WindowExt` methods during the deprecation window.

Applications already using `gpui_component::Root` continue to get text selection
automatically. Root retains the base `TextSelection` element internally, so no
new setup call or wrapper is required from existing callers even after its
direct selection methods are deprecated.

### Behavioral compatibility

The base adapter must preserve the current observable behavior before the old
implementation is deleted:

- ordinary click, Shift+click, repeated Shift+click, and Shift+drag;
- forward and reversed drag selection;
- selection across multiple TextViews;
- blank-space proxy endpoints and right-gutter exclusion;
- double-click word and triple-click paragraph selection;
- select-all and plain/source copying;
- copy ordering and existing newline joining behavior;
- scroll-following endpoints and auto-scroll;
- virtualized block export, including blocks that were not painted;
- selection confinement behind Dialog and Sheet scopes;
- clearing when modal state or relevant layout changes;
- suppression by Input, Button, links, drag-and-drop, and resize interactions;
- existing focus and link-activation behavior.

Compatibility is determined by tests at the public interface, not by retaining
the current private field layout. Internal types such as `TextSelectionState`,
`SelectionEndpoint`, and Root's registries may change freely.

### Single-state compatibility adapter

Compatibility forwarding must use the new base state as the only source of
truth. During the deprecation window, existing `gpui-component` methods are thin
adapters over the base interface. It is not acceptable to keep the old Root
selection state synchronized with a new base state, even temporarily across a
release, because that creates ambiguous ownership and event-order races.

During implementation, temporary compile-time shims may exist within one branch,
but every runnable checkpoint must have one authoritative selection session.

### Compatibility test gate

Before removing the old implementation:

1. Run the existing TextView selection suite unchanged against the base-backed
   adapter.
2. Add contract tests in `gpui-base` with at least two adapters: a plain-text
   adapter and the TextView integration adapter or an equivalent fake.
3. Add public-interface tests proving TextView builder code behaves unchanged,
   base window methods work directly, and deprecated component methods forward
   to the same state during the compatibility window.
4. Compare selected text, selection presence, and clear/end behavior for all
   existing regression fixtures.
5. Keep the old implementation available only as a test oracle during the
   migration; do not ship both engines.

Any intentional behavior change discovered during migration must be reviewed and
landed separately. It must not be hidden inside the architectural move.

## What stays in gpui-component

`gpui-component` continues to own:

- `TextView` and `TextViewState`;
- Markdown and HTML parsers;
- `ParsedDocument` and document nodes;
- Markdown source reconstruction;
- source/plain `SelectionFormat` presentation;
- virtual block semantics;
- TextView focus and `ListState` scrolling;
- links, code blocks, images, and styling.

TextView connects through a thin advanced adapter, tentatively named
`TextViewSelectionAdapter`.

## Current migration seam

Move or generalize from `crates/ui/src/text/window_selection.rs`:

- `TextSelectionState`;
- `SelectionEndpoint`;
- `SelectionStart`;
- `TextSelection` lifecycle element;
- pointer gesture handling;
- endpoint resolution and proxy anchors;
- scope filtering;
- selection-band invalidation;
- suppression integration;
- cross-participant ordering and merging.

Remove from `gpui_component::Root`:

- `text_selection`;
- `selectable_text_views`;
- `selectable_text_inlines`.

Retain behind the TextView adapter:

- `TextView::paint` participant registration;
- `Inline::paint` text-run registration;
- Markdown source selection;
- virtual block selection;
- TextView-specific focus and scrolling.

Move or forward selection methods from `gpui_component::WindowExt` to
`gpui_base::TextSelection`, eliminating the concrete Root downcast.

## Migration plan

1. Freeze the existing public and behavioral compatibility suite before moving
   production code.
2. Build the retained per-window `TextSelection` element and contract tests
   with a fake adapter.
3. Move the ordinary text-run geometry algorithm into base.
4. Add a plain `TextSelectionHandle` or Label adapter, proving the seam serves more
   than TextView.
5. Connect TextView as the advanced adapter while running the unchanged
   compatibility suite against the new single state source.
6. Add the focused `gpui_base::TextSelection` associated functions; make existing
   `gpui-component::WindowExt` methods and `Root::clear_text_selection` delegate
   to it with deprecation guidance.
7. Move opaque scope handling.
8. Delete the old Root fields and UI-only window selection module only after all
   compatibility gates pass.
9. Remove the deprecated component-layer entry points only in a later
   semver-breaking release.

Do not begin with a compatibility facade around the existing Root. That would
create two state sources and leave the new module shallow.

## Primary risks

### Document ordering

Current copying sorts views by screen position. A reusable architecture needs an
explicit participant and run order so multi-column or transformed layouts do not
silently produce incorrect text.

### Virtualized content

The base module cannot infer unpainted Markdown blocks. TextView must export them
through its adapter using stable block identifiers rather than exposing parsed
nodes to base.

### Reentrancy

The window state must compute immutable command or copy snapshots before invoking
adapter callbacks. Calling adapters while the window-state entity is leased risks GPUI
borrow failures.

### Interface growth

Focus, auto-scroll, source export, and virtualization must remain optional
capabilities. Requiring every Label adapter to implement the complete TextView
feature set would make the module shallow despite moving the code.

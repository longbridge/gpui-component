# gpui-base Text Selection Architecture

Date: 2026-08-15

## Goal

Move reusable window-level text-selection behavior into `gpui-base` without
moving `TextView`, Markdown, HTML, or document parsing out of
`gpui-component`. Prove the new seam with both TextView and a plain-text
adapter, while preserving existing TextView behavior.

## Module boundary

`gpui_base::text_selection` is a per-window deep module. It owns pointer-event
coordination, stable endpoints, Shift extension, cross-region selection,
selection scopes, invalidation, focus and auto-scroll commands, and copy
ordering.

The module does not know `gpui_component::Root`, `TextViewState`, Markdown,
HTML, parsed nodes, Dialog, or Sheet. Those concepts connect through adapters.

## Public interface

The base layer exposes:

```rust
pub struct TextSelection;
pub struct TextSelectionRegion;
pub struct SelectionScopeId;
pub struct SelectionRegionFrame;
pub struct SelectionRunFrame;
pub struct SelectionRunState;
pub trait WindowTextSelection;
```

`TextSelection` is a zero-sized element that enables one window selection
session and its event lifecycle. `TextSelectionRegion` is a base-owned,
renderer-neutral entity handle; advanced adapters attach renderer callbacks to
that handle without accessing the implementation-private state.

`SelectionRegionFrame` registers current bounds, scroll offset, hitbox, scope,
document order, visible text-hit bounds, and an optional stable virtual block
key. `SelectionRunFrame` registers laid-out plain text and returns the selected
UTF-8 byte range through `SelectionRunState`.

The window extension exposes `selected_text`, `has_text_selection`,
`clear_text_selection`, and `end_text_selection` against the window state.

## Element lifecycle

The state is window-local and implementation-private. `gpui_component::Root`
renders one `TextSelection` child; a custom root does the same. Roots and
renderers never create, retrieve, or retain the state entity. Frame registration
and scope mutation may create state lazily before the element paints, while
queries without an element remain safe empty/no-op operations.

Registration is frame-based. A region identifier remains stable for the
semantic lifetime of its content. Dead and unpainted regions are pruned safely.
Endpoints store region-relative content coordinates and resolve using the
region's current frame.

## Adapter responsibilities

Base owns ordinary `TextLayout` position-to-byte-range selection and plain-text
copy for registered runs.

The TextView adapter remains in `gpui-component` and provides:

- TextView region registration;
- Inline run registration and highlight painting;
- Markdown source reconstruction;
- virtual block export;
- focus and ListState auto-scroll callbacks;
- layout-revision invalidation.

The plain adapter requires no advanced callbacks and demonstrates that the seam
is not TextView-specific. Label can later expose this as `.selectable(true)`.

## Scope

`SelectionScopeId` is opaque. Base enforces that one selection belongs to one
active scope. `gpui-component::Root` maps its base, active Sheet, and top Dialog
states to opaque IDs. Dialog and Sheet meanings never enter base.

## Ordering and copying

Regions and runs carry explicit logical order. Copying must not depend on
HashMap iteration or screen coordinates alone. Plain adapters use selected run
substrings. Advanced adapters receive an immutable selection snapshot and may
return source text or include virtualized content.

Callbacks are invoked only after the private state releases its entity borrow.

## Backward compatibility and deprecation

Existing TextView builders, `SelectionFormat`, copy/select-all actions, and all
selection behavior remain compatible. Applications using
`gpui_component::Root` do not add anything new.

The selection methods on `gpui_component::WindowExt` and public
`Root::clear_text_selection` may be deprecated in favor of
`gpui_base::WindowTextSelection`. During the deprecation window they forward
to the same window state. The migration never ships two authoritative selection
states. Removal happens only in a later semver-breaking release.

## Behavioral requirements

The unchanged compatibility suite must cover ordinary and Shift click,
repeated/reversed extension, Shift drag, cross-view selection, blank proxy
endpoints, right-gutter exclusion, word/paragraph selection, select-all,
plain/source copy, copy ordering, scrolling, virtual blocks, Dialog/Sheet
confinement, layout invalidation, suppression, focus, links, and drag-and-drop.

Base contract tests use at least two adapters and exercise the public interface.
TextView integration tests remain in `gpui-component`.

## Migration rule

Move behavior behind the new interface rather than layering a facade over the
old Root-owned engine. Every runnable checkpoint has one authoritative
selection state. Old private Root fields and `ui::text::window_selection` are
deleted only after the base-backed TextView passes the complete compatibility
suite.

The detailed assessment and rejected alternatives are in
[`docs/research/2026-08-15-text-selection-base-architecture-assessment.md`](../../research/2026-08-15-text-selection-base-architecture-assessment.md).

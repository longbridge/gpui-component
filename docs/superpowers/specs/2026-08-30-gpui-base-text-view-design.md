# gpui-base TextView and Selectable Text Design

## Goal

Move the complete reusable `TextView` capability, including HTML and Markdown,
from `gpui-component` into `gpui-base`. A project that depends only on
`gpui-base` must be able to render useful rich text, select it with the pointer,
and copy the selected text without first building its own text component or
style sheet.

The existing `gpui_base::Selectable` trait describes controlled selected state
for controls. It will keep that meaning. The new text-selection primitive will
be named `SelectableText` so the two public APIs are not ambiguous.

## Layering

`gpui-base` will own:

- `TextView`, `TextViewState`, document nodes, layout, and rendering;
- Markdown and HTML parsing, including the existing extension/plugin API;
- links, images, lists, tables, code blocks, and selection projections;
- selection gestures, selected-text production, and copy handling;
- `TextViewStyle` and a complete neutral `TextViewStyle::default()`;
- a small `SelectableText` element for ordinary plain text.

`gpui-component` will no longer contain an independent TextView
implementation. It will re-export the Base API at its current public paths and
may add a theme adapter for applications initialized through
`gpui_component::init`. Compatibility code must not make `gpui-base` depend on
`gpui-component`.

## Default styling

`TextViewStyle::default()` is a usable style, not an empty customization bag.
It supplies neutral defaults for body and heading typography, paragraph and
list spacing, links, block quotes, inline and block code, tables, horizontal
rules, selection highlights, and readable unhighlighted code blocks. Base-only
applications can render HTML or Markdown without constructing a style.

Defaults use GPUI primitives and Base semantic theme tokens. They must not read
`gpui-component::Theme`, use component icons, or depend on component-specific
tooltip and scrolling implementations. Every field remains overridable, and
the component facade can derive an adapted style from its active theme to
preserve the current appearance where practical.

The default is a stable neutral light style and cannot require an `App`.
`TextViewStyle::from_theme(&gpui_base::Theme)` derives an appearance-aware
style when a caller wants current Base tokens; callers may also pass an
explicit style for fixed rendering or brand-specific choices.
Syntax highlighting is deliberately not enabled by default and Base does not
own language grammars or a highlighter registry. Callers may inject precomputed
highlight runs or provide a code-block renderer when they want highlighting.

## SelectableText

`SelectableText` is a value-like `Element` backed by a
`TextSelectionHandle`. It accepts an `ElementId`, text, and optional text style
or highlight runs. It registers its hitbox and shaped line bounds with the
window-scoped `TextSelection` service and paints the current selection.

Its baseline behavior includes:

- press-drag selection and selection extension;
- double-click word and triple-click line selection where supported by the
  shared selection service;
- composition with other registered selectable runs in document order;
- `Copy` action handling and plain-text clipboard output;
- neutral selection colors from Base theme tokens;
- opt-in construction with an existing `TextSelectionHandle` for retained or
  multi-element documents.

The convenience constructor owns element-local retained selection state keyed
by the caller-provided `ElementId`. The explicit-handle constructor is the
escape hatch for views that need to observe selection events, control document
ordering, or share selection across several rendered runs.

`SelectableText` is also the maintained reference implementation for projects
building specialized selectable text elements. Its module documentation will
show the minimal Base initialization, `TextSelectionLayer`, focus/copy action,
and shared-handle forms.

## TextView integration

`TextView` continues using the window-scoped `TextSelection` service rather
than creating a second selection model. Its structured document adapter maps
block and inline layout back to source or plain-text ranges. The public
`SelectionFormat` behavior remains available.

Component-only conveniences are replaced by injected behavior or Base
primitives:

- link activation remains a callback and does not require a themed `Link`;
- link titles use an optional renderer instead of a component Tooltip;
- code-block actions use optional render hooks rather than component Icon or
  Button types;
- scrolling uses GPUI/Base scrolling behavior;
- code blocks use a neutral monospace default; optional syntax highlighting is
  injected by the caller and does not add language dependencies to Base.

No feature listed in the current TextView documentation is intentionally
removed by the move.

## Compatibility and migration

The canonical implementation and types live under `gpui_base::text`.
`TextView`, `TextViewState`, `TextViewStyle`, `Text`, `SelectionFormat`, the
Markdown extension types, `markdown`, and `html` are also re-exported from the
`gpui-base` root. `gpui-component::text`
keeps re-exporting the same types and the `markdown(...)` and `html(...)`
helpers, so existing imports continue to compile unless they relied on a
previously private implementation detail.

The move will proceed in dependency order:

1. Add and test `SelectableText` in Base using `TextSelection`.
2. Move parsing, document, state, and selection-adapter modules into Base.
3. Remove component dependencies from rendering through Base defaults and
   render hooks.
4. Move `TextView` and expose the Base public API.
5. Replace the component implementation with compatibility re-exports and its
   theme adapter.
6. Move or duplicate tests at the owning layer, then verify Base-only and
   component compatibility builds.

## Verification

Tests must prove the requested capabilities rather than only compilation:

- Base-only plain text can be dragged, selected across runs, and copied.
- Base-only Markdown and HTML render with `TextViewStyle::default()`.
- headings, links, lists, tables, code blocks, and images still parse and lay
  out through representative tests.
- source and plain selection formats produce the expected clipboard text.
- selection composes across plain `SelectableText` and rich `TextView`
  registrations in one window.
- current `gpui_component::text` constructors and principal builder APIs still
  compile and behave through re-exports.
- `cargo test -p gpui-base` and the relevant `gpui-component` text tests pass;
  workspace checks confirm that no Base source imports `gpui-component`.

## Non-goals

- Renaming or changing the meaning of the existing controlled-state
  `Selectable` trait.
- Moving the full gpui-component theme, icon set, or overlay system into Base.
- Introducing a second window-level selection coordinator.
- Requiring every Base consumer to define a TextView style before rendering.
- Enabling syntax highlighting or bundling language grammars by default.

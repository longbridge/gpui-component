# gpui-base Architecture

## Status and Scope

This document describes the architecture implemented by `crates/base`. It is a
source-derived reference, not a migration plan. Public exports in
`crates/base/src/lib.rs` and the Rust API documentation remain authoritative for
individual methods.

`gpui-base` is the reusable foundation below the styled `gpui-component` crate.
It is designed for both of these callers:

- `crates/ui`, which adapts base behavior into GPUI Component's complete visual
  system;
- applications that build and own a different visual system directly on top of
  base behavior.

## Architectural Thesis

The durable rule is:

> Base owns reusable behavior and the geometry required to implement it. The
> presentation layer owns the product's visual language.

“Headless” does not mean that every base module is a single empty `Div`.
Keyboard navigation, text editing, popup collision handling, virtualization,
calendar grids, resizable panels, and toast stacking require internal structure,
measurement, and retained state. Base owns that complexity when moving it to
callers would duplicate difficult behavior.

Base does not own product-level choices such as brand colors, typography,
control density, borders, radii, icons, variants, or final composition.

## Dependency Direction

```text
                       application
                     /             \
                    ▼               ▼
       application-owned UI     gpui-component
                    \               /
                     └──────┬──────┘
                            ▼
                        gpui-base
                            ▼
                           GPUI
```

Dependencies point downward. `gpui-base` must not import `gpui-component`
themes, assets, or façade types. `gpui-component::init` may initialize and theme
the base layer, but the base layer must also work when initialized directly.

There is no Registry or CLI crate in the current workspace. Source distribution
can be added above this seam without changing the ownership model, but it is not
part of the implemented architecture documented here.

## Module Families

The public surface contains four distinct module families. Treating all of them
as identical “primitives” hides important interface differences.

### 1. Semantic elements

Examples include Button, Checkbox, Radio, Switch, Toggle, Link, Tabs, Progress,
Avatar, and the semantic Table parts.

These modules typically implement GPUI interfaces directly:

```text
IntoElement
+ Styled
+ ParentElement where composition is meaningful
+ InteractiveElement for interactive roots
```

They provide a stable element identity, event normalization, focus behavior,
keyboard activation, accessibility semantics, controlled values, and optional
semantic-state styles. The caller supplies visible children and presentation.

`Button::new("save")`, for example, receives an `ElementId`, not a label. It has
no default height, padding, background, border, or radius.

### 2. Compound behavior roots

Examples include Accordion, Dialog, AlertDialog, Sheet, Popover, HoverCard,
Select, Combobox, DatePicker, and Popup.

These modules coordinate multiple parts or application-owned children. Their
interfaces encode behavior that would be fragile if every caller rebuilt it:

- open-state requests and change reasons;
- trigger and content focus transfer;
- Escape, Confirm, and directional key actions;
- dismissal ordering;
- backdrop hit testing;
- focus trapping;
- trigger measurement and popup placement.

Parts such as `DialogTitle`, `DialogDescription`, and `DialogClose` are explicit
semantic seams. Base does not walk arbitrary descendant trees to discover them.

### 3. Stateful systems

Examples include InputState, CalendarState, TreeState, SliderState,
ResizableState, OtpState, ColorPickerState, ToastManager, and ToastStackState.

These modules retain data because their behavior spans frames or requires
measurement, subscriptions, history, focus, or incremental updates. State is
usually stored in a GPUI `Entity`, a keyed element state, or an application-owned
model passed back to the element.

Stateful systems expose application rendering seams rather than leaking their
implementation. Calendar provides a pre-wired `CalendarItem` and semantic
`CalendarItemState` to an item renderer. Tree owns flattening, expansion,
selection, keyboard movement, and virtualization while the caller renders each
visible `TreeEntry`.

### 4. Infrastructure and utility modules

Examples include Positioner, Scrollbar, VirtualList, FocusTrapElement,
AutoScroll, motion, History, geometry helpers, measurement, theme tokens, and
global initialization.

These are deep modules: a small interface hides layout, lifecycle, or data
structure complexity used by many controls. Their interface is also their test
seam; callers should not need to reproduce the hidden algorithm to verify it.

## State Ownership

State ownership follows behavioral needs rather than one universal pattern.

### Controlled values

Checkbox, Radio, Switch, Toggle, Select, and similar roots accept the current
value and report requested changes. They do not silently mutate application
state.

```text
application value
      │
      ▼
base element ── activation ──▶ on_change(next_value)
      ▲                            │
      └──── next render ───────────┘
```

Callbacks describe intent. Pointer-originated value changes include the
`ClickEvent` when modifier keys are useful. Model-driven changes, such as
pagination requests, do not invent a pointer event.

### Handles and explicitly shared state

Dialog supports `DialogHandle` for imperative open and close requests while
still reporting `DialogChangeReason`. Scrollbar adapters share an underlying
scroll handle. Toast stack geometry is shared through `ToastStackState`.

Handles coordinate one logical behavior. They must not be reused across
unrelated viewports or component instances.

### Entity-backed state

Complex modules use `Entity<State>` when they need GPUI observation,
subscriptions, focus handles, or incremental notification. The state entity owns
behavioral data; the presentation layer still owns how that data looks.

### Keyed element state

Small pieces of ephemeral state tied to an element identity use
`window.use_keyed_state`. Stable `ElementId` values are therefore part of the
interface, not an implementation detail. Changing an ID can reset focus,
measurement, animation, or open-state bookkeeping.

## Styling Model

Base separates three styling mechanisms:

1. ordinary GPUI `Styled` calls for instance presentation;
2. typed semantic-state style builders such as checked, selected, or disabled;
3. GPUI native runtime modifiers such as hover, active, and focus-visible.

The shared `state_style::resolve_style` function makes semantic precedence
consistent across controls. See [Styling and Motion](STYLING-AND-MOTION.md) for
the complete contract.

Compound and stateful modules expose explicit presentation seams:

- GPUI `Styled` on the root or part;
- application-provided children;
- typed part elements;
- item renderer callbacks;
- style refinements for internal virtualized containers;
- presentation snapshots such as `InputPresentation`.

## Input and Editor Architecture

Input is intentionally deeper than the semantic elements.

`InputState` owns the text model and editing behavior:

- Rope-backed text and edit history;
- cursor, selection, IME, masking, validation, and number stepping;
- single-line, multi-line, auto-grow, and code-editor modes;
- wrapping, indentation, folding, decorations, diagnostics, and search;
- LSP provider interfaces and overlay state;
- auto-scroll, editor scrolling, and cursor visibility;
- native text-content integration where supported.

The custom input element owns shaping, layout, hit testing, selection and caret
painting, line-number and gutter painting, and editor scrollbar integration.

Presentation is injected through `InputEditorStyle`, highlighter interfaces,
fold-icon renderers, context-menu adapters, and the higher-level UI input frame.
This keeps the hard editor mechanics local while allowing `crates/ui` or an
application to supply theme colors and surrounding controls.

Platform-specific behavior is isolated behind adapters. For example, folding is
disabled on WebAssembly, time uses `web_time` where needed, and native text
content support is conditionally compiled.

## Overlay and Positioning Architecture

Overlay modules separate lifecycle from placement and presentation.

### Positioner

`Positioner` is the shared placement implementation. It supports:

- side placement with preferred-side selection, flipping, alignment, offset,
  and viewport clamping;
- corner placement compatible with anchored trigger geometry, with viewport
  clamping but no side flip.

Popup measures its trigger during prepaint, stores the captured bounds in keyed
state, and renders positioned content through a deferred element on the next
frame. Tooltip reuses Positioner rather than maintaining another collision
algorithm.

### Modal hosts

Dialog and Sheet create viewport-sized hosts. They own focus trapping, keyboard
actions, backdrop or overlay dismissal, and callback ordering. They do not own
the final popup or panel placement: the application styles the supplied popup or
surface as centered, right-aligned, bottom-aligned, or another product-specific
layout.

Overlay visuals and overlay hit targets are separate layers. The surface is
painted above the dismissal target so a click inside content does not dismiss
the modal.

## Scrolling and Virtualization

`ScrollbarHandle` is the seam between scrolling implementations and the shared
Scrollbar element. An adapter reports:

- viewport bounds;
- current offset;
- content size;
- offset updates;
- optional drag lifecycle notifications.

Adapters exist for GPUI `ScrollHandle`, `UniformListScrollHandle`, and
`ListState`, plus base `VirtualListScrollHandle`.

Scrollbar normally overlays the viewport reported by its handle. Two explicit
alternatives support deeper widgets:

- `viewport_bounds` supplies custom-painted bounds, as the editor does;
- `viewport_from_layout` uses same-frame layout bounds, as DataTable does when
  excluding fixed headers and columns.

VirtualList owns variable-size item layout, visible-range calculation, content
masking, deferred scroll-to-item requests, and clamped offsets. The caller owns
item sizes and renders only the requested range. Tree builds on GPUI's uniform
list because its visible entries share a row height.

One handle represents one logical viewport. Sharing a handle between nested or
unrelated scroll areas causes offsets, hitboxes, and scrollbar geometry to
interfere.

## Theme Projection

The base `Theme` contains:

- `SemanticThemeTokens` for colors, radius, spacing, typography, and shadows;
- global Scrollbar defaults;
- the minimal Resizable handle colors required by its infrastructure.

Semantic tokens describe roles and scales, never component names. They do not
automatically style base controls. A presentation layer reads and applies them.

`gpui-component` projects its active theme into the base theme during
initialization and theme changes. Direct base users can mutate
`gpui_base::Theme::global_mut(cx)` themselves.

## Motion and Lifecycle

Ordinary semantic controls do not install product motion. The generic
`motion::transition` function manages keyed interpolation, timing, reversal,
animation frames, and reduced-motion behavior while the caller chooses the
animated property.

Some deep behavior modules own configurable motion that is inseparable from
their layout lifecycle. `ToastStack`, for example, owns stack expansion,
collapse, measurement, and overlap motion through `ToastMotion`. Its child
`Toast` remains an unstyled semantic root, and the application owns toast
content and visual styling.

This distinction is intentional:

- product decoration and choreography belong to presentation;
- motion required to keep a deep behavioral layout coherent may live in that
  module and must be configurable.

## Initialization

Call `gpui_base::init(cx)` before constructing base controls. Initialization:

- installs the base global theme if absent;
- initializes shared global state;
- registers key bindings and infrastructure for dialog, focus traps, popover,
  sheet, combobox, color picker, select, number input, input, and tree.

Applications that call `gpui_component::init(cx)` must not initialize base a
second time; the styled crate includes base initialization.

## Relationship to `gpui-component`

`gpui-component` is a presentation adapter and compatibility layer above base.
It may:

- map its Theme into base tokens and infrastructure defaults;
- wrap base elements with labels, icons, sizes, variants, and product layout;
- provide application-specific popup menus, LSP views, and native integrations;
- preserve historical public interfaces while delegating behavior to base.

The migration of a UI control to a base module is complete only when behavior,
focus, keyboard interaction, accessibility, overlay geometry, and visual output
remain correct. Sharing a type name or compiling an adapter is not sufficient.

## Design Invariants

Changes to `gpui-base` should preserve these invariants:

1. Base remains independent of `gpui-component` presentation and assets.
2. Reusable behavior is implemented once behind the lowest useful interface.
3. Applications can replace the visual language without reimplementing the
   behavior module.
4. Behavioral geometry stays with the module that must keep it correct.
5. Controlled elements report intent and do not own application values.
6. Stable element identity is documented wherever keyed state is used.
7. Pointer and keyboard paths converge on the same semantic action.
8. Accessibility is behavior, not optional decoration.
9. Overlay coordinates, paint order, and hitboxes use one viewport model.
10. One scroll handle represents one logical viewport.
11. Platform differences are isolated behind explicit adapters or conditional
    implementations.
12. Long-lived architectural facts belong here; progress logs and temporary
    reviews belong in issues and pull requests.

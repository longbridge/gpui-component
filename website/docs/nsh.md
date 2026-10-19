---
title: NSH
description: Experimental GPUI Base and Component bindings for NSH applications
---

# NSH

The experimental `gpui-kit-nsh` crate lets an NSH application render GPUI
Component controls. NSH owns TSX composition, signals and its JavaScript
runtime; the adapter renders the original Rust components with GPUI Fast.
The crate is currently available from this repository, and is not published.
It has its own workspace and lockfile, so its required GPUI Fast feature does
not change the backend selected by the parent workspace's existing crates.

The catalog has partial bindings across all seventy Component UI modules,
including inputs, collections, layout, charts, dock and window overlays.
Complete public API parity is still under review. A `partial` module has usable
bindings; it means every Rust builder, callback and composition path has not
been proven equivalent, rather than that the whole module is unavailable. Live children and named-slot composition, host theme
synchronization and standard NSH run/package runners have source
implementations and are undergoing integration verification. Their presence
does not establish that all behavior checks have passed. This adapter targets
the GPUI desktop backend on macOS, Windows and Linux; other NSH backends cannot
render these controls.

NSH dependencies come from the real NSH source checkout; this repository does
not vendor NSH. Engine and runner changes belong in the NSH repository. The
local setup requires sibling `gpui-kit/` and `nsh/` checkouts. Use the NSH revision pinned in the NSH job of
`.github/workflows/ci.yml` for a matching checkout. CI checks out that exact
revision alongside GPUI Kit and verifies both adapters; no NSH source snapshot
is stored in this repository.

Bindings use Rust-owned native specs. Rust `Props` and `Event` declarations
feed NSH's existing `Registry`; the same registration supplies native factories,
runtime exports and TypeScript declarations in `nsh/ui/gpui`. Shared composition,
controlled-value reconciliation and batch validation live in NSH's Rust renderer.
Ordinary leaf controls use a small shared macro and their existing Rust builder.
Stateful controls retain the entities, subscriptions and lifecycle hooks they
actually need. The adapter reuses Component's rendering and styling.

There are no handwritten per-component TS wrappers. The compatibility entry
`crates/nsh/js/index.ts` forwards generated exports and derives historical helper
types from generated props. It does not contain a second component schema.
This reduces duplicate maintenance; it does not imply that complex controls need
no Rust adaptation or that the total Rust line count necessarily decreases.

The seven-component baseline, shared composition and controlled-value rejection
have passed targeted runtime checks. The generated CLI has also passed type
generation, valid and invalid type checks, HMR and macOS dev packaging.
The complete migrated catalog has passed compilation and generated-type checks.
Interaction checks are recorded separately; source registration does not establish
complete component API parity.

## Compare the Base approach

Both implementations are retained independently. `crates/nsh` uses the styled
Component library through `nsh/ui/gpui`. `crates/nsh-base` exposes unstyled Base
behavior through `nsh/ui/base`; its dependency graph does not enable Component
or Kit assets. NSH owns the Base example's colors, typography and composition.

The Base approach currently starts with Button, Checkbox and Input, with a
separate real-engine UI verifier. It does not claim to expose every Base API.
The matching task-creation examples are `crates/nsh/examples/compare` and
`crates/nsh-base/examples/app`. They use the same task name, priority, validation,
submission, reset and focus flow so their UI can be compared.

Run the Base UI verifier from the repository root:

```sh
script/test-nsh-base-ui
```

It builds the Base CLI and host, compiles the original task example and runs
real NSH/GPUI pointer, keyboard, controlled-state and lifecycle checks. Screenshots
are written to `target/nsh-base-verify`. This currently requires macOS or Linux
with a working GPUI headless renderer and the matching sibling NSH checkout.

After that script finishes, open the Base example with its own host:

```sh
cargo run --manifest-path crates/nsh-base/Cargo.toml --features dev,cli,test-support --bin nsh-gpui-host -- --bundle target/nsh-base-task-app/bundle.nshc
```

After installing the Component runtime below, open the matching Component example:

```sh
target/nsh-runtime/gpui-kit-nsh build crates/nsh/examples/compare -o target/nsh-component-compare
target/nsh-runtime/gpui/nsh-gpui-host --bundle target/nsh-component-compare/bundle.nshc
```

## Run the example

Install the Kit-aware NSH CLI and matching runners from the repository root:

```sh
cargo run --manifest-path crates/nsh/Cargo.toml --bin gpui-kit-nsh-runtime -- --manifest-path crates/nsh/Cargo.toml --profile dev target/nsh-runtime
target/nsh-runtime/gpui-kit-nsh check crates/nsh/examples/app
target/nsh-runtime/gpui-kit-nsh build crates/nsh/examples/app -o target/nsh-example
cargo run --manifest-path crates/nsh/Cargo.toml --example nsh -- target/nsh-example/bundle.nshc
```

The example host loads a trusted, locally built bundle. It initializes
GPUI Component and opens the window through `gpui_kit::open_window`.
`gpui-kit-nsh` calls NSH's existing CLI with the Kit UI set registered and
finds the matching runners beside its executable, unless `NSH_RUNTIME` is set.
An unrelated global `nsh` executable does not contain this UI set; selecting
the GPUI target alone does not register it.

The generated TSX bindings can be imported directly:

```tsx
import { signal } from "nsh";
import { Button, Input } from "nsh/ui/gpui";

const name = signal("");

<Input
  value={name()}
  accessibilityLabel="Name"
  onChange={(value) => name.set(value)}
  style={{ width: 280, height: 36 }}
/>;

<Button
  label="Clear"
  disabled={name() === ""}
  onPress={() => name.set("")}
  style={{ width: 120, height: 36 }}
/>;
```

The CLI generates `nsh/ui/gpui` and its editor declarations from Rust; no
separate JavaScript component package is required. The repository's
`crates/nsh/js/index.ts` remains a compatibility entry during migration.
Give each native control
an explicit NSH layout size. Intrinsic size negotiation is not implemented.

Input retains its editing state across NSH signal updates. Echoing the same
value back skips `set_value`, preserving the current selection and undo
history. Replacing the value with a different external value intentionally
uses the input's reset behavior. Input also exposes `onFocus`, `onBlur` and
`onSubmit`, and its NSH node ref accepts `command("focus")`.

Checkbox and Switch are controlled: pass `checked` and write the boolean
received by `onChange` back into the owning signal. `disabled` is available
on Button, Checkbox, Switch, Input, Select and Slider; Input additionally supports `readonly` and `placeholder`.

## Use the standard NSH commands

Install the Kit development and package runners into a separate runtime
directory, then point the NSH CLI at it:

```sh
cargo run --manifest-path crates/nsh/Cargo.toml --bin gpui-kit-nsh-runtime -- --manifest-path crates/nsh/Cargo.toml --profile dev target/nsh-runtime
target/nsh-runtime/gpui-kit-nsh run crates/nsh/examples/app --target gpui
target/nsh-runtime/gpui-kit-nsh package crates/nsh/examples/app --target gpui -o target/nsh-package
```

The installer creates `gpui-kit-nsh`, `gpui/nsh-gpui-host` and `gpui/nsh-gpui-app`, with `.exe`
on Windows. The `dev` profile is for local verification; omitting
`--profile dev` builds release runners by default. The development host uses
NSH's dev server and hot reload; the package runner loads NSH's standard
resource layout. Package formats remain NSH's macOS `.app`, Linux tar.gz and
Windows zip. The macOS dev profile has been exercised through initial module loading,
module execution after a source edit via HMR, .app packaging, and launching
the packaged runner from /tmp to load its own resources. Logs are in
`target/nsh-typed-rust-runtime/verification.log`. This does not verify signal
preservation or full GUI interaction. Release optimization and Windows/Linux
runtime execution have not been verified.

This workflow applies to ordinary NSH TSX applications. When `nsh.json` names
a custom `rust` crate, the current NSH CLI prioritizes its generated Rust
runtime and bypasses `NSH_RUNTIME`; that generated path does not yet install
Kit Root/catalog. A custom Rust host can call `gpui_kit_nsh::host::run` explicitly.

The Kit host calls `gpui_kit::init`, installs Root through
`gpui_kit::open_window`, registers the Component catalog, and synchronizes NSH
appearance with the Component theme. It retains NSH's JavaScript runtime and
does not introduce gpui-shell.

## Select and Slider

Select takes `items: { value: string; label: string; disabled?: boolean }[]`
and a controlled `value: string | null`. Values must be unique and remain
stable across label changes, filtering and reordering. `onChange` receives
the selected value or `null` when cleared. `searchable` enables built-in
search; `cleanable` enables clearing. The adapter retains the list and focus
handles when these props or the item data change. Its node ref accepts
`command("focus")`.

Slider accepts either a number or an ordered `[start, end]` pair. `onChange`
reports continuous user changes; `onRelease` reports the value when the user
releases the control. `min`, `max`, `step`, `scale` (`linear` or `logarithmic`),
`vertical`, and `reverse` map to the Rust component. Values clamp to the active
bounds. Bounds must be finite with `min < max` and `step > 0`; logarithmic
scale additionally requires a positive minimum and a finite maximum/minimum
ratio. Changing props updates the retained state rather than creating a new
entity or subscription. Programmatic value updates do not emit user events.

Select and Slider expose `onError` for invalid items, values or configuration.
An invalid update leaves the previous usable native state intact. Select accepts
`item:<value>` and `value:<value>` named slots for live NSH JSX rendering of
options and the selected value. Slot content keeps its original NSH owner.

## Component inventory

The source adapters now cover the following families. Every component module remains
`partial`: declarations, registration, runtime behavior and full API parity
are separate acceptance checks. This table describes implementation scope;
per-component interaction and cross-platform checks that have not run are
not recorded as passing.

| Family | Bindings |
| --- | --- |
| Controls | Button, ButtonGroup, Toggle, ToggleGroup, Checkbox, Switch, Radio, RadioGroup, Rating, Slider |
| Input | Input, Textarea, Editor, NumberInput, OtpInput |
| Display | Icon, Avatar, AvatarGroup, Badge, Tag, Alert, Marker, Spinner, Skeleton, ShimmerText, Progress, ProgressCircle, Separator, Kbd, Label, Clipboard, Link |
| Collections | List, Table, DataTable, Tree, Combobox, Command, Pagination |
| Date and time | Calendar, DatePicker, TimeField |
| Navigation and layout | Accordion, Collapsible, TabBar, Tabs, Sidebar, SidebarHeader, SidebarFooter, Breadcrumb, Form, Field, GroupBox, DescriptionList, Settings, Toolbar, ToolbarGroup, StatusBar, Empty, Carousel |
| Dock and resize | DockArea, Resizable |
| Scrolling | ScrollArea, VirtualList, HorizontalVirtualList |
| Content | TextView, Markdown, Html, CodeBlock, Diff, Image |
| Chart and plot | LineChart, AreaChart, BarChart, PieChart, RadarChart, CandlestickChart, SankeyChart, Plot |
| Messaging | Message, MessageGroup, Bubble, BubbleGroup, Attachment, AttachmentGroup, MessageScroller, Questionnaire |
| Window | Dialog, Sheet, Notification, TitleBar, WindowBorder |
| Popups | Popover, HoverCard, Tooltip, ContextMenu, DropdownMenu, NativeMenu, DropdownButton |
| Color and progress | ColorPicker, ColorSelect, Stepper |
| Speech | SpeechButton, SpeechWaveform |

`Image` uses GPUI's `img` and is classified separately from Component modules.
Speech requires the corresponding system backend and permissions; an API
being present does not imply that speech recognition is available. Editor implements an asynchronous JavaScript language-service request/response
channel, with runtime verification still in progress. The token:<id> slot renders
live NSH JSX, and showDocument is handed back to NSH through an event.
The existing Base resolve_completions hook has no usable call path; the binding
does not invent that capability. Full LSP CompletionItem fields can be returned
in the completion response. Plot uses native scales, axes, grids and line/area/bar/pie/arc primitives;
arbitrary Rust geometry callbacks are not serialized into JavaScript. Refer to the generated `nsh/ui/gpui` declarations and the Rust adapter
implementations for the current props, events and ref commands.

Use `bun script/check-nsh-catalog.ts --report target/nsh-api-audit.json` to
produce source evidence for public Rust declarations, renderable controls and
parts, and each binding's props/events/commands. Unreviewed entries retain the
`unreviewed` marker; matching names do not establish behavior parity.
`--require-complete` rejects every `partial` or `pending` module so registration
counts cannot be used as the completion criterion.

## Window overlays

Dialog and Sheet use a controlled `open` prop. `onOpenChange(false)` reports
user dismissal; replacing a Sheet also reports its closure. Setting `open={false}`
from the owner does not emit a user event. Dialog also provides `onConfirm` and
`onCancel`; `closeOnConfirm` and `closeOnCancel` control whether those actions
close it. `size` is in GPUI logical pixels: Dialog width or Sheet thickness.
Titles and descriptions use text props; Dialog/Sheet children and the `footer`
slot accept live NSH JSX subtrees.

Notification accepts `open`, `title`, `message`, `variant` and `autohide`, and
reports dismissal through `onOpenChange(false)`. Each instance has independent
identity; unmount removes only its notification. Updating content replaces the
notification and restarts its timeout. This binding currently uses in-app
notifications; system delivery is not exposed. The `content`/`action` slots and
`actionLabel`/`onAction` expose real custom actions, retaining the native rule
that notifications with actions do not auto-hide.

PopupMenu retains its native Entity and supports `item:<id>` JSX slots,
keyboard selection, controlled dismissal and open/close/focus ref commands.
AppMenuBar explicitly leases application-global menus: the latest mounted
active owner wins, and unmount restores the previous live owner or host menus.
An explicit host takeover is preserved. Action ids must be nonempty and unique.

Dialog, Sheet and Notification use Component's window-level Root and can extend beyond the
native node's layout rectangle. Dialog and Sheet close through scoped handles
on unmount, preserving unrelated NSH instances and host overlays. Verification records individual passing lifecycle paths below; the complete
overlay fixture still awaits confirmation of its last path.

## Verify the integration

The dedicated verifier runs a real NSH engine in a GPUI offscreen window on
macOS or Linux (a headless GPU renderer is required):

```sh
cargo run --manifest-path crates/nsh/Cargo.toml --features test-support --example verify
```

It checks signal round trips, Input cursor preservation on controlled echoes,
shared Checkbox/Switch state, Select identity after item reordering, Slider
change/release events, programmatic synchronization and unmount. It saves
offscreen images in `target/nsh-verify`, with reduced motion enabled to inspect
settled surfaces. This is a targeted integration check,
not proof of full component coverage or all-platform behavior.

A separate `NSH_VERIFY_FAMILY=datetime` run passed on macOS, checking real date
selection, disabled dates, keyboard time-segment editing, controlled reset
without feedback, navigation snapshots, range/presets, and precision/hour-cycle
changes. The real-checkout rerun also passed Calendar, DatePicker and TimeField
owner veto checks. A separate Collections run also verified real List, DataTable JSX cells, Tree,
Command, grouped multi-select Combobox and Pagination. Its latest run also
passed List/Tree Enter confirmation, live compound Table cells and independent
ListItem clicks. These checks do not
establish complete part coverage or all-platform behavior.

The real-checkout Inputs run passed multiline editing, controlled caret/echo,
Editor language-switch undo, number stepping, OTP completion, rich-token
deletion, InputGroup mode switching, and asynchronous completion responses
accepted through the native menu. Collections also passed real Tab entry into
Tree and reverse traversal into Command after disabling its search field.

The Display run passed native Tab/Shift-Tab traversal, disabled-item skipping,
controlled Rating rejection/acceptance, stable group focus after reordering,
Clipboard and split Dropdown interactions, and cleanup while a menu is open.

The focused Dock run verified real tab/splitter dragging, zoom, layout save/
restore, standalone ResizeHandle dragging, and Input editing/cursor state
across restoration. Nineteen content scenarios exercised all sixteen registered
names, real Diff clicks, TextView block/inline JSX slots, and Plot's native
bar/pie/scales/grid/axis. Speech was checked only in its idle rendering state;
no microphone or real recognition was started. After switching to the real NSH checkout, the complete menu, layout, window
overlay and AlertDialog fixtures passed. They cover scoped ownership and focus
chains, Notification dismissal, owner veto/reset, live nested content, active-tab
focus, and application-menu ownership restoration. The combined run is checked
separately for interference between families.

`bun script/check-nsh-catalog.ts` checks the module inventory against
Component's public module seams and the Rust/TSX registration names. The
inventory marks partial bindings separately from pending modules and shared
infrastructure; it does not count a registration name as full API support.

The adapter registers the native catalog through
`gpui_kit_nsh::components()`, assigned to NSH's `AppOptions::components`.
It selects Kit's `gpui-fast` feature; ordinary Kit applications keep their
existing dependency and initialization paths.

Button, Checkbox, Switch, Radio, Toggle and Clipboard expose
`track_focus(&FocusHandle)` in their Rust builders; DropdownButton also exposes
`track_menu_focus(&FocusHandle)`. This lets NSH use the actual control handles.
Compound groups retain handles by item value and expose enabled tab stops in
visual order. Ordinary Rust callers can omit these methods and keep the
existing retained-handle behavior.

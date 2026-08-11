# gpui-base Migration Checklist

This checklist tracks the migration to `gpui-base`. Check an item only
after its implementation and compatibility requirements have been verified.

## Current Status

- **Updated:** 2026-08-11
- **Overall:** In progress
- **Completed workstream:** Base Button interaction verification
- **Completed workstream:** Semantic Theme Tokens compatibility bridge
- **Paused workstream:** Hand-authored component Registry templates
- **Active:** M1/M2 `crates/ui` composition and legacy compatibility gates
- **Active:** Base Toggle behavior
- **Next integration gate:** Verify the complete M2 vertical slices before checking
  individual component milestones.

## Working Rules

- Mark `[x]` only when implementation and proportional verification both exist.
- Keep legacy styled components unchanged unless a compatibility adapter requires
  a narrowly scoped edit.
- A legacy component may be checked only when its Base-backed wrapper preserves
  100% of the original behavior, interaction, design, functionality, and public
  API. Compilation or approximate visual similarity is not sufficient evidence.
  A deliberate behavior change requires explicit user approval and must be
  recorded as a named compatibility exception with replacement acceptance tests.
- Base must never depend on `gpui-component`.
- Base owns behavior and infrastructure, but controls and parts remain no-style:
  they do not install layout, positioning, color, sizing, gap, radius, border,
  shadow, variant, or animation defaults. `crates/ui` remains the canonical
  complete presentation source during migration.
- Do not hand-maintain a second simplified component implementation under
  `registry/`. A future Registry pipeline must derive complete editable sources
  from `crates/ui`.
- Do not run `cargo fmt` during intermediate iterations; format once at the final
  integration gate.
- Append material decisions, validation results, and blockers to the log below.

## Decisions and Constraints

- The real legacy Button path is `gpui_component::button::Button`; it must remain
  distinct from the foundation `gpui_base::Button`.
- Base Button requires a stable `ElementId`, so its constructor is
  `Button::new(id)`.
- Base Button relies on GPUI's native click synthesis for pointer, Enter, and
  Space activation; a second keybinding/action path would double-fire.
- Theme migration cannot move `crates/ui/src/theme` alone. Its current types are
  coupled to list, scrollbar, sheet, notification, and highlight-theme models.
- New semantic tokens must not copy component-specific fields such as
  `button_*`, `list_*`, `tab_*`, or `sidebar_*`.
- New `ColorTokens` uses shadcn's semantic vocabulary, including `surface` and
  `destructive`; legacy `popover` and `danger` fields remain available through
  the compatibility projection.
- `gpui_component::init` calls Base initialization at the former focus-trap
  initialization point to preserve ordering.
- `crates/ui` is the single canonical source for complete default components.
  Registry generation is deferred until it can emit those complete components;
  toy or manually duplicated Registry implementations are not acceptable.

## Local Reference Implementations

- `../shadcn` — primary reference for CLI lifecycle, project preflight,
  `components.json`, Registry dependency resolution, diff/update behavior, and
  non-destructive source installation. Start with
  `packages/shadcn/src/commands/` and `packages/shadcn/src/preflights/`.
- `../reui` — reference for a large real-world Registry taxonomy, alternative
  component bases, item metadata, blocks, and dependency composition. Start with
  `registry-reui/_meta/components/bases/`, `registry/`, and `components.json`.
- `../base-ui` — reference for primitive Root/Item boundaries, controlled state,
  axis/orientation naming, disabled semantics, and accessibility. Do not copy its
  React context/provider plumbing into GPUI; prefer normal elements, builders,
  `ParentElement`, and GPUI-owned state/focus mechanisms.
- These are design references, not source-level dependencies. Adapt their
  ownership and workflow ideas to Rust/GPUI; do not copy Web, React, or CSS-specific
  architecture into Base.
- `./target/release/gpui-component-story` — authoritative pre-migration Story
  binary for interaction, behavior, layout, styling, animation, focus, and visual
  detail comparisons. Preserve this binary; compare it side by side with a new
  build instead of rebuilding over it.

## Known Blockers and Risks

- Publishing `gpui-base` is intentionally deferred; this phase validates it only
  as an internal workspace dependency.
- GPUI currently has no obvious public `aria_disabled` helper. Disabled Button
  accessibility and propagation need runtime evidence or an upstream seam before
  that checklist item can be completed.
- Semantic theme-file support uses a standalone `SemanticThemeConfigFile`; the
  public field shape of legacy `ThemeConfig` remains unchanged for source compatibility.
- Root, Overlay, Dock, and the current Theme are not file-level moves; each needs
  a dependency seam before migration. VirtualList has moved after reversing its
  only scrollbar dependency through a UI-local adapter.

## Base Control Review Checklist

Apply this list to every Base control and its `crates/ui` composition before
checking the component milestone:

- [ ] The UI component constructs the Base primitive directly in `RenderOnce::render`;
  there is no `compose`, `into_stateful`, `take_children`, or façade-only escape
  hatch that exposes an intermediate render representation.
- [ ] Element construction follows GPUI fluent-builder style: one chain using
  `when`/`when_some`/`when_none`/`map` for conditional composition, without
  mutable temporary elements and imperative reassignment when a chain suffices.
- [ ] Base controls and parts are completely unstyled: they install no default
  layout, positioning, color, size, gap, radius, border, shadow, variant, or
  animation. All presentation, including structural layout, belongs to `crates/ui`
  or the application; caller `Styled` refinements remain the closest layer.
- [ ] Base receives the complete application-owned content through normal
  `ParentElement` slots. Verify label, icon, loading indicator, custom children,
  indicator/thumb parts, and trailing affordances in their exact render order.
- [ ] A semantic state with existing visual presentation is actually expressed
  through the component's typed `.styles(...)` context. Do not add empty state
  styles or invent a new disabled/checked appearance merely to exercise the API.
- [ ] GPUI-native interaction states use the native modifiers: `.hover(...)`,
  `.active(...)`, `.focus(...)`, and `.focus_visible(...)`. They are not duplicated
  inside semantic `.styles(...)` and are installed at most once on an element.
- [ ] Semantic state names match the control contract: Toggle uses `pressed`,
  Checkbox/Switch/Radio use `checked`, and selectable collection items use
  `selected`. This project's legacy Button also has a controlled `selected`
  presentation contract used by DropdownMenu/Popover triggers to retain their
  trigger appearance while the menu is open. It remains distinct from GPUI's
  native momentary `active` state and Toggle's persistent `pressed` contract,
  and it never implies `aria_toggled`; accessibility toggle metadata remains an
  explicit, independent UI façade choice. The legacy `Selectable` trait remains
  UI-owned.
- [ ] Style precedence is explicit and tested. Base resolves root static style,
  then active semantic states, then GPUI runtime interaction states. If a legacy
  UI contract requires caller instance style to beat semantic presentation, its
  semantic closure explicitly refines that caller style last; Base does not add a
  second public instance-style namespace. Different properties still compose.
- [ ] Motion is application/UI-owned and uses `transition(...)` only where replacing
  the existing animation preserves the approved timing and reversal contract.
  Base controls contain no built-in component animation.
- [ ] Pointer, keyboard, focus, disabled propagation, controlled callback, role,
  accessible state/name, and tab behavior have runtime tests. Compile-only API
  tests are not completion evidence.
- [ ] A compound control has a real Base root/item contract, controlled selection,
  orientation, roving focus, Arrow/Home/End navigation, disabled propagation, and
  group/item accessibility. A vector calculation helper is not a group primitive.
- [ ] Public façade builders and visual output are compared with the preserved old
  Story binary. Any deliberate incompatibility is named, approved, and locked by
  replacement acceptance tests.

## Validation Log

- 2026-08-11: `cargo check -p gpui-base -p gpui-component` passed after
  the initial foundation extraction.
- 2026-08-11: `cargo test -p gpui-component --test base_compat` passed 3 tests,
  covering legacy type identity and application-owned Button state styling.
- 2026-08-11: Semantic token projection and configuration tests passed (6 tests).
- 2026-08-11: CLI initialization, path safety, dependency resolution,
  non-destructive installation, and editable-source tests passed (6 tests).
- 2026-08-11: Base Button runtime suite passed (5 tests): pointer activation,
  Enter/Space activation, disabled propagation, state styling, and current
  accessibility surface.
- 2026-08-11: Base Button, Checkbox, Radio, and Switch runtime suite passed
  together (20 tests). Their component milestones remain open pending Registry
  and legacy compatibility evidence.
- 2026-08-11: Base Toggle runtime suite passed (5 tests).
- 2026-08-11: Legacy Checkbox, Radio, RadioGroup, and Switch API compatibility
  suite passed (4 tests); their adapters now delegate behavior to Base while
  preserving the original public and presentation contracts.
- 2026-08-11: CLI suite passed with installed-binary fallback templates for
  Button, Checkbox, Switch, and Radio (7 tests).
- 2026-08-11: Legacy Button compatibility suite passed (5 tests), existing Button
  unit suite passed (11 tests), and its adapter preserves the existing visual tree.
- 2026-08-11: M1/M2 Base runtime suite passed 43 tests; facade compatibility
  suites passed 12 tests; legacy Link and Button-Link child prepaint regressions
  proved visible children have non-zero layout bounds.
- 2026-08-11: Link destination data remains separate from presentation content:
  `.href(...)` does not render text, matching the old API; visible content must
  be supplied through `.child(...)`.
- 2026-08-11: CLI embedded Registry sources moved to external, format-friendly
  files and the canonical-resource parity test passed as part of 9 CLI tests.
- 2026-08-11: User manually exercised Button, Switch, Checkbox, and Slider in
  the running gallery and found their behavior and presentation correct. Radio,
  Toggle, and Link still require the same manual old/new comparison.
- 2026-08-11: M1/M2 Base controls gained typed semantic root style contexts.
  Registry usage was deliberately deferred: `crates/ui` must first become the
  canonical complete composition source, and future Registry output will be
  produced from it rather than maintained as duplicate simplified code.
- 2026-08-11: Generic CSS-like `transition(...)` landed with ElementId-like
  scalar/tuple identity, delay/easing, smooth target reversal, reduce-motion,
  and per-window/view keyed lifecycle. No Base component installs motion.
- 2026-08-11: `ui::Button` now directly stores and renders the standard Base
  Button element. The legacy icon/loading icon, label, application children, and
  dropdown caret remain UI-owned presentation but are attached as one Base child
  in their original order. The temporary `from_stateful`/`into_stateful` escape
  hatch was removed. Base Button tests passed 6, UI Button tests passed 14, and
  the legacy Button API suite passed 5.
- 2026-08-11: The Checkbox façade adopted disabled root text styling where exact
  legacy ordering could be retained. Button disabled styling remains in UI after
  strict review rejected compatibility-only behavior flags. Radio and Switch
  require typed part slots; Toggle requires a legacy-compatible state vs. instance
  priority seam. Slider's Base element is not feature-equivalent to the legacy
  range/drag implementation, and Link's historical disabled flag is inert.
- 2026-08-11: Existing Checkbox and Switch animations were not rewritten with
  `transition(...)`: their old direction-keyed animation/reversal behavior differs
  from the new smooth current-sample reversal contract. Compatibility takes
  precedence over superficial API adoption.
- 2026-08-11: A proposed loading activation flag and disabled pointer-policy flag
  were rejected and removed after API review. Legacy loading/disabled event guards
  remain in UI until Base can preserve their exact focus and accessibility action
  surface without compatibility-only public switches.
- 2026-08-11: A proposed `SplitButtonState` was rejected and removed because it
  only wrapped `disabled || loading` and selected getters. DropdownButton remains
  open until a real popup-trigger/overlay seam can own open/dismiss/anchor/focus
  behavior without depending on UI PopupMenu or Theme.
- 2026-08-11: Story migration was explicitly deferred; this phase does not edit
  `crates/story`.
- 2026-08-11: Strict Standards/Spec review rejected public ButtonGroupState and
  ToggleGroupState as shallow wrappers around vector calculations; both types and
  their UI wiring were removed. Group behavior remains an open M1 seam.
- 2026-08-11: Strict M2 review reopened every component milestone. Checkbox was
  rebuilt without façade-only behavior flags or child/focus escape hatches;
  complete indicator/label/children presentation is attached once to the natural
  Base root. Exact façade behavior remains under review before completion.
- 2026-08-11: Radio, Switch, and Toggle compatibility-only Base flags and the
  shallow RadioGroupState were removed. Their legacy UI implementations were
  restored because disabling canonical focus/keyboard/a11y behavior through
  public Base switches is not an acceptable migration seam.
- 2026-08-11: The disconnected single-value Base Slider was removed; the actually
  migrated range/log/drag `slider_state` remains. Slider stays open until one
  unified primitive owns state, keyboard, a11y, pointer, drag, and release.
- 2026-08-11: Base Link now always keeps natural Link role/focus semantics and no
  longer exposes façade-only role/focus switches. Legacy UI Link was restored and
  Link remains open; `href` plus injected navigation policy stays a valid Base API.
- 2026-08-11: A no-flags Toggle adapter was tested and reverted. `tab_stop(false)`
  keeps it out of keyboard traversal but pointer activation still focuses it;
  native mouse-down `prevent_default` preserves legacy no-focus behavior but also
  prevents GPUI's native click synthesis. Without changing the legacy contract or
  adding a compatibility switch, the current GPUI seam cannot preserve pointer
  once, pointer no-focus, and keyboard inert behavior simultaneously.
- 2026-08-11: **Approved compatibility exception — Toggle interaction.** The user
  explicitly authorized replacing the legacy pointer-only Toggle contract with
  the canonical Base/shadcn contract: pointer focus, Tab traversal, Enter/Space
  activation, and disabled activation/propagation blocking. Visual design,
  controlled value semantics, callbacks, builders, and public legacy API remain
  compatibility requirements.
- 2026-08-11: Toggle now composes the natural Base primitive without compatibility
  flags. Base owns pressed/disabled state, activation, focus, keyboard behavior,
  accessibility, and content attachment; `crates/ui` retains the complete visual
  presentation, tooltip, variants, sizing, and instance-style precedence. Runtime
  coverage locks pointer, keyboard, disabled propagation, accessibility, and style
  behavior. ToggleGroup remains a separate open M1 boundary.
- 2026-08-11: A ToggleGroup item/binding/context and roving-keyboard experiment
  was removed after API review. Base ToggleGroup is intentionally a simple
  no-style `ParentElement` root with `axis(Axis)` accessibility metadata; it does
  not own keydown navigation. UI keeps its existing controlled `Vec<bool>` and
  bubbling composition without exposing React-style context or binding APIs.
- 2026-08-11: Checkbox façade no longer installs a Base activation handler when
  the legacy `on_click` callback is absent. Façade tests cover pointer once/no
  pointer focus, Tab plus Enter/Space, disabled bubbling/inert behavior, and
  label/custom-child layout. `CheckboxIndicator` adds a no-wrapper typed part;
  checked/disabled indicator border and fill now use `.styles(...)`, while the
  existing UI-owned 250ms checkmark animation remains unchanged. The user's
  manual Checkbox comparison and these runtime gates complete the Checkbox M2
  slice.
- 2026-08-11: `crates/ui/src/styled.rs` is now re-export only. Generic
  `StyledExt`, `FocusableExt`, flex/shadow helpers, and inspector reflection moved
  to Base. The temporary ad-hoc `StyledTheme` field projection was removed;
  themed helpers now consume the semantic tokens in `gpui_base::Theme`.
  UI-specific `Size`, `Sizable`, `StyleSized`, `Selectable`, `Disableable`, and
  `Collapsible` remain in UI modules. Theme initialization and supported mode/
  registry changes synchronize the Base Theme projection.
- 2026-08-11: Base Button gained controlled `selected` state and
  `ButtonStyles::selected`. UI Button now expresses both selected and disabled
  presentation only through typed styles. Selected is the existing
  DropdownMenu/Popover trigger contract; it is independent from native `active`,
  Toggle `pressed`, and explicit toggle accessibility metadata.
- 2026-08-11: Switch track presentation now uses a no-style, no-wrapper
  `SwitchTrack` typed part. Checked and checked+disabled backgrounds flow through
  typed styles; unchecked background, geometry, tooltip, Thumb, label, and the
  existing 150ms animation remain UI-owned. The stateful track requires an
  explicit ElementId, and UI uses the structured `(switch_id, "track")` identity
  rather than a call-site-derived key.
- 2026-08-11: Pure GPUI `ElementExt::on_prepaint` moved to Base with its only
  blanket implementation. UI keeps `ChildElement`/`AnyChildElement` because they
  depend on UI-specific sizing, and re-exports the Base extension trait.
- 2026-08-11: `IndexPath` moved intact to Base, including its `ElementId`
  conversion and behavior tests. UI now re-exports the same type identity.
- 2026-08-11: ButtonGroup/ToggleGroup characterization tests lock the migration
  baseline without adding features: callback override rules, rendered snapshot
  selection results, ButtonGroup disabled builder-order behavior, and the fact
  that GPUI keyboard Click does not reach either group's bubbling callback.
- 2026-08-11: Generic `History`/`HistoryItem` moved intact to Base with all undo,
  redo, grouping, unique, and version tests. UI Input and Dock use a minimal
  `is_ignoring`/`set_ignoring` seam instead of the former crate-private field;
  UI re-exports the same type identity.
- 2026-08-11: VirtualList's complete vertical/horizontal virtualization,
  measurement, visible-range, deferred scroll, and handle implementation moved
  to Base. Runtime tests cover both axes, scroll-to-item, and empty lists.
- 2026-08-11: Generic drag `AutoScroll` timing, edge-speed calculation, task
  lifecycle, and stop state moved to Base. Input and Text Selection continue to
  consume the same type through the legacy `scroll::AutoScroll` re-export.
- 2026-08-11: Collapsible was audited but not migrated. The current UI type is a
  styled conditional `v_flex` container; it owns no trigger, activation, focus,
  accessibility, or open-change event contract. Moving it intact would violate
  Base's no-style boundary, while inventing those missing behaviors would exceed
  the current migration scope.
- 2026-08-11: The complete generic Scrollbar module moved to Base: handle
  adapters, both axes, track click, thumb drag lifecycle, automatic visibility,
  fade timing, and custom painting. A deliberately small `ScrollbarStyles`
  interface follows GPUI fluent composition and exposes track width/background/
  border plus normal/hover/active thumb width, inset, radius, minimum length,
  and background. `ScrollbarMode` controls `Scrolling`, `Hover`, and `Always`;
  the former `ScrollbarShow` API was removed rather than retained as an alias.
  The Theme setter synchronizes the Base projection so Story mode changes take effect.
  Project-specific `Scrollable<E>` and `ScrollableMask` remain in UI with all
  existing tests.
- 2026-08-11: The Base package was renamed from `gpui-component-base` to the
  concise `gpui-base` (`gpui_base` in Rust paths). Base now has one coherent
  `Theme` Global containing semantic tokens and per-module defaults; Scrollbar
  contributes `ScrollbarTheme { mode, styles }` rather than registering
  scattered globals or adding component fields to a generic styled projection.

## Crate Foundation

- [x] Add `gpui-base` as an internal workspace crate; publishing is deferred.
- [x] Keep the dependency direction one-way: `gpui-component` depends on Base.
- [x] Preserve the original focus-trap initialization order in `gpui_component::init`.
- [x] Move geometry primitives and extension traits into Base.
- [x] Move interaction event extensions into Base.
- [x] Move pure `ElementExt::on_prepaint` into Base while retaining UI-sized
  child composition types in `crates/ui`.
- [x] Move `IndexPath` into Base and preserve its legacy type identity.
- [x] Move generic `History`/`HistoryItem` into Base and preserve Input/Dock
  behavior and the legacy module path.
- [x] Move VirtualList and its scroll handle into Base while retaining the UI
  legacy type identity.
- [x] Move generic drag AutoScroll behavior into Base and preserve its legacy
  scroll module type identity.
- [x] Move animation and transition behavior into Base.
- [x] Move focus-trap behavior and state into Base.
- [x] Move generic styled extensions into Base while keeping UI-specific sizing
  and component traits in `crates/ui`; make `ui/styled.rs` re-export only.
- [x] Re-export migrated APIs from their existing `gpui-component` public paths.
- [x] Add compile-time compatibility coverage for migrated public types.
- [ ] Verify Base and facade packaging after the Base version is visible on crates.io.
- [ ] Document and verify the release order: dependencies, Base, then facade.

## Base Component Milestones

A component is checked only when its Base behavior/foundation exists and its
`crates/ui` composition preserves 100% of the legacy behavior, interaction,
design, functionality, and API. Registry production is a later milestone and is
not an M1/M2 completion gate.

### M1 — Pilot Vertical Slice

Completion definition: prove the Base → `crates/ui` composition seam on one
component, including interaction tests and unchanged legacy UI.

- [ ] Button module

M1 covers the complete public `crates/ui/src/button` module, not only the
`button::Button` struct:

- [x] Button uses a standard Base `RenderOnce` root element with no
  `Stateful<Div>` escape hatch.
- [x] Button activation, disabled state, focus, and semantic root-style delegation
  from the legacy façade without compatibility-only Base flags.
- [x] Button content slot composition: icon/loading icon, label, application
  children, and dropdown caret retain their exact UI order and render through
  the Base `ParentElement` seam.
- [x] Button loading inert behavior boundary.
- [ ] ButtonGroup behavior and child composition boundary.
- [ ] DropdownButton trigger/menu behavior boundary.
- [ ] Toggle and ToggleGroup behavior/state boundary.
- [ ] Verify every public type and builder exported by `button/mod.rs` remains
  100% compatible.

Current evidence covers the standard Button root, complete content composition,
activation, disabled/focus behavior, loading inert behavior, and selected/disabled
typed presentation. Both group behavior models remain open. DropdownButton
additionally requires the generic popup-trigger/
overlay seam planned for the overlay phase; `disabled || loading` and
selected-value passthrough alone would be a shallow Base module.

### M2 — Simple Controls

Completion definition: migrate independent controls that do not require overlay or
compound state. Variants, sizes, icons, layout, and colors remain outside Base.

Composition audit rule: compare each slice with `../shadcn`'s primitive/component
split. Base owns state contracts, events, accessibility, and composable content
slots; `crates/ui` owns the complete indicator, label layout, icons, variants,
visual tokens, and motion policy. A future Registry exporter uses this complete UI
source; it must not introduce a parallel reduced implementation.

- [x] Checkbox
- [ ] Radio
- [ ] Switch
- [ ] Toggle — Base behavior and pressed presentation are connected; final old/new
  Story visual comparison remains.
- [ ] Slider
- [ ] Link

### M1/M2 — Stateful Presentation

- [x] Typed `StateStyle` preserving GPUI `Styled` and `FluentBuilder`.
- [x] Button semantic root style: disabled.
- [x] Button semantic root style: selected.
- [x] Checkbox semantic root styles: checked, indeterminate, disabled.
- [x] Radio semantic root styles: checked, disabled.
- [x] Switch semantic root styles: checked, disabled.
- [x] Toggle exposes typed semantic root styles for pressed and disabled.
- [x] `crates/ui::Toggle` delegates its existing pressed presentation through
  `.styles(...)`; it intentionally does not invent a disabled appearance that the
  legacy component never had.
- [ ] Slider semantic root style: disabled (deferred until the Slider primitive is
  unified with the migrated range/drag state).
- [x] Link semantic root style: disabled.
- [x] Generic application-owned value transitions with no component defaults.
- [x] Checkbox Indicator typed state projection with no wrapper or built-in motion.
- [x] Switch Thumb and Track typed state projection with no wrapper or built-in motion.
- [ ] Typed state projection for remaining parts.
- [ ] Interaction transition spike for GPUI hover/active edges.

### M3 — Input Controls

Completion definition: centralize editing and selection behavior while Registry
templates own field structure and appearance.

- [ ] Input
- [ ] Number Input
- [ ] OTP Input
- [ ] Form

### M4 — Disclosure and Navigation

Completion definition: provide reusable state, keyboard navigation, focus, and
accessibility without prescribing presentation.

- [ ] Collapsible
- [ ] Accordion
- [ ] Tabs
- [ ] Pagination
- [ ] Stepper
- [ ] List
- [ ] Tree

### M5 — Overlay and Compound Components

Completion definition: Base owns overlay lifecycle, positioning, dismissal, focus
trap, and keyboard behavior; Registry owns trigger/content structure and style.

- [ ] Popover
- [ ] Tooltip
- [ ] Hover Card
- [ ] Menu
- [ ] Select
- [ ] Combobox
- [ ] Dialog
- [ ] Alert Dialog
- [ ] Sheet
- [ ] Date Picker
- [ ] Color Picker

### M6 — Data and Application Infrastructure

Completion definition: preserve complex crate-owned behavior and expose presentation
seams suitable for application-owned wrappers.

- [x] Virtual List
- [ ] Table
- [ ] Data Table
- [ ] Editor
- [ ] Text View
- [ ] Resizable
- [ ] Scrollable
- [ ] Notification
- [ ] Dock

### Registry-only Components and Blocks

These do not get standalone Base components. Check each item when its Registry
template is delivered and its required behavior is composed from existing Base APIs.

- [ ] Alert
- [ ] Avatar
- [ ] Badge
- [ ] Breadcrumb
- [ ] Description List
- [ ] Group Box
- [ ] Kbd
- [ ] Label
- [ ] Progress
- [ ] Rating
- [ ] Separator
- [ ] Sidebar
- [ ] Skeleton
- [ ] Spinner
- [ ] Status Bar
- [ ] Tag
- [ ] Title Bar
- [ ] Settings

## Infrastructure

- [x] Keep UI-specific sizing APIs outside Base while moving generic styled
  helpers out of the legacy styled module.
- [x] Split pure GPUI element extensions from UI-sized child composition.
- [x] Move the generic Scrollbar behavior, handle interface, axis/show modes,
  painting, dragging, and fade lifecycle into Base while keeping UI-specific
  Scrollable wrappers and masks in `crates/ui`.
- [x] Move VirtualList into Base without depending on styled scrollbar components.
- [ ] Extract an unstyled overlay host and entry model.
- [ ] Extract popup positioning and focus-restoration behavior.
- [ ] Extract Dock's serializable layout model from its styled runtime views.
- [ ] Preserve Editor and Input State as crate-owned infrastructure.
- [ ] Define accessibility contracts for all Base behaviors.

## Semantic Theme Tokens

- [x] Add component-independent `ColorTokens`.
- [x] Add `RadiusTokens`.
- [x] Add `SpacingTokens`.
- [x] Add `TypographyTokens`.
- [x] Add `ShadowTokens`.
- [x] Add a `SemanticThemeTokens` aggregate.
- [x] Project semantic tokens from the current public `Theme` without stale duplicate state.
- [x] Apply semantic tokens without changing legacy component-specific token fields.
- [x] Support semantic tokens in standalone theme configuration files without
  changing the public shape of legacy `ThemeConfig`.
- [ ] Verify legacy themes render exactly as before.
- [ ] Mark the existing component-specific `ThemeTokens` surface as compatibility-only.

## Legacy Compatibility

- [x] Keep `gpui_component::button::Button` distinct from the new Base Button.
- [x] Preserve the existing geometry, event, animation, and focus-trap type identity.
- [ ] Add compile tests covering all migrated legacy module and root paths.
- [ ] Capture baseline screenshots from `./target/release/gpui-component-story`
  without overwriting that binary.
- [ ] Compare the migrated gallery against the baseline on supported platforms.
- [ ] Verify existing component behavior and interaction are unchanged.
- [ ] Verify all existing examples compile without source changes.
- [ ] Verify macOS, Linux, Windows, and WASM build paths.

## Registry and CLI

- [x] Add the `registry/ui`, `registry/blocks`, and `registry/themes` structure.
- [x] Define and validate the Registry Item JSON format.
- [x] Implement registry dependency graph resolution.
- [x] Add an application-owned Button registry template.
- [x] Add `gpui-components.json` project configuration.
- [x] Add the `gpui-component` CLI binary crate.
- [x] Implement `gpui-component init`.
- [x] Implement `gpui-component add button`.
- [x] Support adding multiple components without duplicates.
- [ ] Add a representative block with registry dependencies.
- [ ] Record installed item version and content hash metadata.
- [ ] Implement non-destructive `diff`, `update`, and `status` workflows.
- [ ] Support configured third-party registries.
- [x] Verify generated source is owned and freely editable by the application.

## Completion Gates

- [ ] `cargo fmt --all --check`
- [ ] `cargo check -p gpui-base`
- [ ] `cargo test -p gpui-base`
- [ ] `cargo check -p gpui-component --no-default-features`
- [ ] `cargo test -p gpui-component`
- [ ] Story Web / WASM check
- [ ] `cargo clippy --workspace --all-targets -- --deny warnings`
- [ ] `cargo package -p gpui-component` after Base is available from crates.io
- [ ] Requirement-by-requirement review against `RFC.md` acceptance criteria

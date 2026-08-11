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
- Base control and part taxonomy follows `../base-ui` unless an existing GPUI or
  gpui-component API must be preserved. Use shadcn as a presentation and
  composition reference, not as authority for Radix-specific primitive names.
  When Base UI and Radix differ, prefer the Base UI name; for example, expose
  `SliderIndicator`, not Radix's `SliderRange`.
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
  migrated range/log/drag state in `slider` remains. Slider stays open until one
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
- 2026-08-11: Public `GlobalState` application-menu storage and pointer
  text-selection suppression moved to Base with legacy type identity. UI-only
  TextView selection stacks and modal selection scopes remain in a private UI
  global. Base tests, the 20-test compatibility suite,
  21 window-selection tests, 13 TextView tests, and the full 448-test UI suite pass.
- 2026-08-11: Deferred-popup registration moved from the UI sidecar into Base
  `GlobalState` and is shared by Popover, Select, Combobox, and Input. The full
  existing `PopoverState` lifecycle then moved to Base with legacy type identity:
  open/dismiss, Escape and DismissEvent handling, focus capture/restore, tracked
  focus, and callbacks remain one implementation. Base `Popup` subsequently
  centralized trigger measurement, anchor positioning, first-frame capture,
  deferred rendering, and edge snapping for both Popover and HoverCard; UI
  retains outside-click policy, appearance, and content.
- 2026-08-11: A state-only `HoverCardState` extraction was rejected as shallow.
  Base now exposes the complete unstyled `HoverCard`, owning trigger/content
  hover handoff, delayed open/close, stale-task cancellation, callbacks, and the
  generic `Popup` positioning host. UI owns only Popover appearance and content.
- 2026-08-11: Base `Popup` replaced the duplicated Popover/HoverCard anchor
  helpers and trigger-bounds fields. Runtime coverage proves first-frame
  measurement followed by deferred content, and the UI Popover runtime opens
  and dismisses through that host. The test also characterizes the existing
  duplicate `false` open-change notification on outside click; it remains an
  explicit compatibility issue rather than being silently changed during the
  structural migration.
- 2026-08-11: Tooltip's custom positioner moved intact to Base. It measures
  application-owned children, chooses or flips among Top/Bottom/Left/Right,
  clamps to viewport plus client inset, and applies the prepaint offset. Base
  owns no tooltip style or animation; five Base geometry tests and the retained
  UI lifecycle test pass.
- 2026-08-11: Resizable moved as one coherent Base boundary: `ResizableState`,
  panel/group layout, programmatic resizing, dynamic panel lifecycle,
  window-level pointer drag, resize completion events, and the resize-handle
  interaction element now have one implementation. The UI module only
  re-exports the same public types and projects its existing border/drag colors
  through the application-wide Base `Theme`; Base defaults remain transparent.
  Dock no longer imports the private panel controller. Runtime tests cover
  lifecycle redistribution, measured layout/programmatic resize, and real
  handle drag with one resize callback.
- 2026-08-11: Button module source-compatibility coverage now exercises all
  exported types and builder families, including icon/loading icon, action
  tooltip, DropdownButton menu/anchor, group callbacks, Toggle content, and
  Base/legacy type separation. ButtonGroup is intentionally UI-only; adding a
  Base wrapper would not remove any caller complexity.
- 2026-08-11: Automated old/new Story capture was attempted without modifying
  either binary, but bare GPUI executables are not exposed as individually
  addressable macOS applications to the available accessibility automation.
  The existing release binary remains preserved; visual comparison stays open
  rather than treating source inspection or tests as screenshot evidence.
- 2026-08-11: Removed the shallow Base `Alert`. Its implementation only
  forwarded controlled visibility, `Role::Alert`, style, and children; deleting
  it returns no behavior complexity to callers. The UI Alert again applies the
  role directly to its existing styled root and keeps the exact legacy tree.
- 2026-08-11: Accordion now exposes the five Base UI structural parts
  (`Accordion`, `AccordionItem`, `AccordionHeader`, `AccordionTrigger`, and
  `AccordionPanel`) without item contexts, bindings, render adapters, or
  constructor-only synthetic IDs. Header/Panel use optional GPUI-style `.id()`
  only when emitting Heading/Region accessibility nodes. The façade keeps its
  existing single/multiple aggregation and exact presentation, while its 200ms
  measured-height interpolation uses the application-owned Base `transition`
  API. A façade layout test locks expanded content between its own header and
  the next item; final Story comparison remains open.
- 2026-08-11: Accordion Story comparison caught a real layout regression: the
  migration had merged the legacy `outer flex_1 -> inner v_flex/overflow_hidden`
  nodes, placing clipping on the flex-constrained item and truncating expanded
  content under the next row. A fixed-height two-item regression test reproduced
  the overlap before the fix. Restoring the original two-node layout turns that
  test green and preserves the Base Item as the inner semantic root.
- 2026-08-11: Removed the shallow Base `StepperTrigger` after comparison with
  the full UI Stepper and `../base-ui` (which has no matching Stepper
  primitive). The wrapper only duplicated one `div().on_click` and owned none
  of selected-index projection, item state, separator geometry, or axis layout;
  the original UI trigger path is restored unchanged.

## Crate Foundation

- [x] Add `gpui-base` as an internal workspace crate; publishing is deferred.
- [x] Keep the dependency direction one-way: `gpui-component` depends on Base.
- [x] Preserve the original focus-trap initialization order in `gpui_component::init`.
- [x] Move geometry primitives and extension traits into Base.
- [x] Move interaction event extensions into Base.
- [x] Move the shared `ui` action namespace (`Confirm`, `Cancel`, and directional/
  paging selection actions) into Base without changing existing action types.
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
- [x] ButtonGroup remains UI composition rather than gaining a shallow Base
  wrapper. Individual Base Buttons already own activation and selected state;
  joined borders, axis layout, variants, sizing, legacy `Vec<usize>` aggregation,
  callback override, and builder-order behavior remain façade contracts.
- [x] DropdownButton trigger/menu behavior boundary: its two styled Buttons and
  PopupMenu remain UI composition, while DropdownMenu uses the Base-backed
  PopoverState and Popup host for open/dismiss/focus/anchor lifecycle. No
  Button-specific split-state helper is introduced.
- [x] Toggle and ToggleGroup boundary: Base Toggle owns controlled pressed,
  disabled, activation, focus, and accessibility behavior. Base ToggleGroup is
  limited to the existing semantic group root, `Axis` orientation, style, and
  normal children; the façade retains its legacy `Vec<bool>` aggregation and
  segmented presentation without adding roving focus or keydown behavior.
- [x] Verify every public type and builder exported by `button/mod.rs` remains
  source compatible through compile coverage for Button, variants/rounding,
  ButtonGroup, DropdownButton, Toggle, ToggleGroup, traits, callbacks, normal
  children, icon/loading/tooltip/menu builders, and Base/legacy type separation.

Current evidence covers the standard Button root, complete content composition,
activation, disabled/focus behavior, loading inert behavior, selected/disabled
typed presentation, and the established UI-only group composition boundaries.
DropdownButton now composes the generic Base-backed Popover lifecycle and Popup
positioning host through DropdownMenu; `disabled || loading`, selected-value
passthrough, split layout, menu construction, and appearance remain UI concerns.

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
- [ ] Slider — Base `Slider`, `SliderTrack`, `SliderIndicator`, and `SliderThumb`
  now own the existing a11y, pointer/range selection, drag, bounds, disabled,
  Change, and Release behavior over the single migrated `SliderState`. The UI
  façade retains the exact track/range/thumb presentation. Final post-migration
  Story visual comparison remains.
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
- [x] Slider disabled behavior is projected through the Base root and parts. The
  legacy Slider has no separate disabled root appearance, so no empty semantic
  style is invented.
- [x] Link semantic root style: disabled.
- [x] Avatar exposes unstyled root, image, and fallback slots in Base. The UI
  façade retains sizing, initials, hash-derived colors, placeholder icons,
  borders, and theme presentation; AvatarGroup remains UI-owned overlap layout.
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
- [x] Form — current implementation is UI-only grid/label/description layout;
  it has no validation, submission, field registration, or accessibility
  behavior contract to migrate into Base.

### M4 — Disclosure and Navigation

Completion definition: provide reusable state, keyboard navigation, focus, and
accessibility without prescribing presentation.

- [x] Collapsible — Base owns controlled content visibility and normal
  child/content slots; the façade preserves the legacy vertical layout.
- [ ] Accordion
  - [x] Base exports the complete structural primitive set: `Accordion`,
    `AccordionItem`, `AccordionHeader`, `AccordionTrigger`, and
    `AccordionPanel`.
  - [x] `Accordion` is the unstyled semantic group root with normal GPUI
    children. Existing single/multiple open-set projection, group-disabled
    propagation, and root-bubbling callback timing remain in the façade; moving
    them would require context/binding machinery that is larger than the
    existing behavior.
  - [x] `AccordionItem` connects one controlled trigger/header/panel pair;
    `AccordionHeader` projects heading level, `AccordionTrigger` projects
    button/expanded state, and `AccordionPanel` owns region mounting.
  - [x] The façade uses Base `transition` for its existing 200ms measured-height
    disclosure motion. Base primitives do not install default animation.
  - [ ] Final façade visual/interaction comparison against the old Story.
  - GPUI does not currently expose `aria-controls` / `aria-labelledby`; the
    trigger-panel accessibility relationship remains an explicit upstream gap.
    Title/icon/content layout, sizing, borders, colors, and measurement/clipping
    presentation remain façade-owned.
- [ ] Tabs
  - [x] Base `Tab` owns the existing pointer activation, disabled gating,
    `Role::Tab`, accessible label, selected state, children, and typed
    selected/disabled root styles.
  - [x] Legacy `Tab::new()` remains parameterless. `TabBar` assigns the existing
    index identity through Base Tab's GPUI-style `.id(...)` before render; Base
    does not synthesize an identity or add keyboard/focus behavior.
  - [x] Base `Tabs` owns `Role::TabList`, normal GPUI `child`/`children`, style,
    and interaction forwarding. Controlled selection and callbacks are expressed
    directly on each Base `Tab`; the façade keeps its existing index projection
    and callback override without exposing a binding/context interface.
  - [ ] Final façade behavior/visual comparison remains required before marking
    the compound control complete.
  - `TabBar` must continue to own indicator bounds and animation, overflow menu,
    variants, sizing, and layout because those are façade presentation. Do not
    introduce item bindings, contexts, render helpers, or keyboard behavior to
    force the remaining selection seam.
- [ ] Pagination — keep in UI until page items and ellipsis navigation can form
  one real behavior seam; do not migrate only `calculate_page_range` or wrap
  styled `Button`/`PopupMenu` composition in a shallow Base type.
- [ ] Stepper
  - Current Stepper is UI-only item numbering, separator geometry, axis layout,
    icon/text presentation, and a small pointer callback. The former Base
    `StepperTrigger` merely wrapped the same `div().on_click` and was removed as
    a shallow middle man. Revisit only when Stepper gains a real process/state
    contract that can move as one coherent boundary.
- [ ] List
  - [x] Move the public pure `ListSettings` configuration into Base with the
    legacy module path re-exporting the same type.
  - [ ] Separate selection, navigation, scrolling, loading, and section models
    from the styled `ListItem` and rendering delegate before moving List state.
- [ ] Tree
  - [x] Move the existing public `TreeItem`, `TreeEntry`, and `TreeEvent`
    hierarchy/state model into Base with shared clone state and type identity.
  - [x] Move selection, expansion navigation, focus, virtual-list scrolling, and
    entry interaction into Base. The legacy UI façade adapts Base's application-
    owned item slot to styled `ListItem` and keeps `PopupMenu` composition in UI.

### M5 — Overlay and Compound Components

Completion definition: Base owns overlay lifecycle, positioning, dismissal, focus
trap, and keyboard behavior; Registry owns trigger/content structure and style.

- [x] Popover
  - [x] Base exposes the complete unstyled `Popover`; it owns trigger
    activation, controlled/uncontrolled open state, focus, Escape and outside
    dismissal, callbacks, and `Popup` composition. The UI façade now delegates
    those behaviors and contributes only appearance and application content.
  - [x] `PopoverState` is a single Base type owning the existing controlled
    open state, deferred-context registration, DismissEvent subscription,
    Escape dismissal, tracked focus, focus capture/restore, and open callback.
    The legacy `popover::PopoverState` path re-exports the same type.
  - [x] Base `Popup` owns trigger measurement, anchor-point calculation,
    first-frame synchronization, deferred rendering, and window-edge snapping;
    both Popover and HoverCard use it without presentation defaults.
  - [x] Move outside-click policy into the unstyled Base Popover host; appearance
    and application content remain UI-owned.
  - [x] Existing outside-click characterization emits `on_open_change(false)`
    twice because the trigger wrapper and content mouse-down-out paths both run.
    Preserve it during structural migration; fix only as an explicit behavior
    change with replacement acceptance tests.
- [ ] Tooltip
  - [x] Base `TooltipPositioner` owns the existing child measurement,
    preferred-side selection, four-direction flipping, viewport/client-inset
    clamping, and prepaint offset without appearance or motion.
  - [x] Base `TooltipOverlay` is the per-window provider and owns show delay,
    hide grace, active request switching, dismissal, and positioning. UI trigger
    adapters submit `TooltipRequest` values instead of driving duplicate state.
  - [x] UI injects the existing enter/switch animation renderer through
    `render_with`; appearance, key-binding content, and motion remain UI-owned.
- [ ] Hover Card
  - [x] Base `HoverCard` owns trigger/content hover handoff, delayed open/close,
    stale-task cancellation, open callbacks, and popup positioning. Do not split
    its state back out into a façade-driven timer holder.
  - [x] Popover and HoverCard consume the same Base `Popup` positioning host.
  - [ ] Appearance and content remain UI/application-owned.
- [ ] Menu
- [ ] Select
- [ ] Combobox
- [ ] Dialog
  - [x] Base owns the shared Cancel/Confirm actions, key bindings, focus trap,
    keyboard dismissal/confirmation, callback ordering, and the unstyled
    DialogTitle, DialogDescription, and DialogClose parts.
  - [x] Base exposes outside-dismiss policy and application-provided
    overlay/surface slots.
  - [ ] The UI façade still owns its legacy outside-dismiss/title-bar filter;
    delegate that final policy to Base without changing layered animation.
- [ ] Alert Dialog
  - [x] Base AlertDialog specializes the modal host with `Role::AlertDialog`
    and non-dismissible overlay defaults.
  - [ ] The UI façade still owns legacy button variants and declarative header
    presentation until those slots are projected onto the Base host.
- [x] Sheet — Base owns the focus trap, Escape action, overlay dismissal policy,
  close callback ordering, and application-provided overlay/surface slots. The
  UI façade retains window/title-bar geometry, placement, size, borders,
  title/footer/body composition, scrolling, animation, and selection scope.
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
- [x] Resizable
- [ ] Scrollable
- [ ] Notification
- [ ] Dock

### Registry-only Components and Blocks

These do not get standalone Base components. Check each item when its Registry
template is delivered and its required behavior is composed from existing Base APIs.

- [x] Alert — UI-only presentation. Controlled visibility is ordinary
  conditional rendering, `Role::Alert` belongs directly on the styled root,
  and the close affordance has no reusable lifecycle. A Base wrapper would only
  forward role, children, and style without removing caller complexity.
- [x] Badge — UI-only presentation; count formatting, dot/icon placement, color,
  and visibility do not form a separate Base interaction primitive.
- [x] Breadcrumb — UI-only navigation presentation. Base UI has no Breadcrumb
  primitive, and the existing item behavior is only a styled `div` with an
  optional click handler and link/list-item role. Separators, ordering, last-item
  semantics, labels, and navigation presentation remain one cohesive UI module.
- [x] Description List — UI-only structured presentation; column grouping,
  spans, separators, label widths, borders, and layout do not define an
  interaction primitive.
- [x] Group Box — UI-only presentation container; title/content slots remain
  normal application composition and variants are visual.
- [x] Kbd — UI-only keybinding presentation and platform formatting helper;
  it does not own input behavior or key dispatch.
- [x] Label — UI-only text presentation; masking, secondary text, and highlight
  rendering do not define a Base interaction primitive.
- [ ] Progress
  - [x] Base linear `Progress` owns the existing clamped controlled value,
    progress role, numeric accessibility metadata, normal children, style, and
    interaction forwarding without presentation or motion.
  - [x] `ProgressCircle` composes the same Base `Progress` root, so linear and
    circular façades share one controlled value and accessibility contract while
    retaining their existing application-owned drawing and animation.
  - [x] Base exposes unstyled `ProgressTrack` and `ProgressIndicator`; the
    linear façade uses both instead of a raw indicator div. Indeterminate
    loading is projected by the Base root without an incorrect numeric value.
  - [ ] Final linear façade visual/animation comparison and ProgressCircle
    accessibility boundary review.
- [ ] Rating — requires one Base-owned group/item state path for controlled
  value, hover preview, and activation. Do not expose the current keyed state or
  add item bindings solely to move the existing icon loop.
- [x] Separator — UI-only presentation; solid/dashed painting, label layout,
  axis geometry, and color remain application-owned.
- [ ] Sidebar
- [x] Skeleton — UI-only presentation and animation; do not add built-in motion
  to Base.
- [x] Spinner — UI-only icon presentation and animation; do not add built-in
  motion to Base.
- [x] Status Bar — UI-only three-region layout and theme presentation; its
  contents retain their own behavior primitives.
- [x] Tag — UI-only presentation; variants, sizing, borders, colors, and child
  layout do not define a Base behavior primitive.
- [x] Title Bar — UI- and platform-specific window chrome. Native window drag,
  double-click behavior, drag regions, context menus, and window controls remain
  together in `crates/ui`; do not add a shallow Base primitive.
- [x] Settings — application-level composition of Sidebar, Input, Resizable,
  List, Button, GroupBox, and concrete setting-field renderers. Its search,
  reset, page, group, and field protocols remain in UI; it does not define a
  standalone unstyled Base primitive.

## Infrastructure

- [x] Keep UI-specific sizing APIs outside Base while moving generic styled
  helpers out of the legacy styled module.
- [x] Split pure GPUI element extensions from UI-sized child composition.
- [x] Move the generic Scrollbar behavior, handle interface, axis/show modes,
  painting, dragging, and fade lifecycle into Base while keeping UI-specific
  Scrollable wrappers and masks in `crates/ui`.
- [x] Move VirtualList into Base without depending on styled scrollbar components.
- [x] Move the native/WASM async channel compatibility layer into Base; the UI
  crate keeps its internal path as a direct re-export.
- [x] Move the generic `Selectable` and `Disableable` behavior contracts into
  Base while preserving the legacy root exports. Keep the legacy `Collapsible`
  trait in UI because its public name conflicts with the Base element.
- [x] Move generic measurement helpers into Base while preserving the legacy
  root function and `Measure` type identity.
- [x] Move the macOS accessibility hit-test forwarding shim into Base; the UI
  Root only invokes the platform foundation hook.
- [x] Move the public application-menu and pointer text-selection suppression
  `GlobalState` into Base with legacy type identity. Keep TextView selection
  stacks and deferred-popover bookkeeping in a private UI sidecar so Base does
  not depend on UI text or overlay types.
- [x] Migrate Resizable state, panel layout, window-level drag lifecycle, and
  resize-handle interaction as one coherent boundary. The private panel
  controller remains private; `ResizableState` owns dynamic
  insert/remove/reset/clear and container-size operations directly. Base has
  transparent handle defaults, while the UI Theme projects the exact existing
  border and active-drag colors.
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
- [x] Mark the existing component-specific `ThemeTokens` surface as compatibility-only.
  Its public fields remain unchanged for legacy themes and façade components;
  new application-owned presentation uses `SemanticThemeTokens` instead.

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

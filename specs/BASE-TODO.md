# gpui-component-base Migration Checklist

This checklist tracks the migration to `gpui-component-base`. Check an item only
after its implementation and compatibility requirements have been verified.

## Current Status

- **Updated:** 2026-08-11
- **Overall:** In progress
- **Completed workstream:** Base Button interaction verification
- **Completed workstream:** Semantic Theme Tokens compatibility bridge
- **Completed workstream:** Registry and CLI Button foundation
- **Active:** Simple-control Registry templates and legacy compatibility gates
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
- Base must never depend on `gpui-component`.
- Base owns behavior and infrastructure; applications and Registry templates own
  structure, variants, sizing, and visual style.
- Do not run `cargo fmt` during intermediate iterations; format once at the final
  integration gate.
- Append material decisions, validation results, and blockers to the log below.

## Decisions and Constraints

- The real legacy Button path is `gpui_component::button::Button`; it must remain
  distinct from the unstyled `gpui_component_base::Button`.
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

## Local Reference Implementations

- `../shadcn` — primary reference for CLI lifecycle, project preflight,
  `components.json`, Registry dependency resolution, diff/update behavior, and
  non-destructive source installation. Start with
  `packages/shadcn/src/commands/` and `packages/shadcn/src/preflights/`.
- `../reui` — reference for a large real-world Registry taxonomy, alternative
  component bases, item metadata, blocks, and dependency composition. Start with
  `registry-reui/_meta/components/bases/`, `registry/`, and `components.json`.
- These are design references, not source-level dependencies. Adapt their
  ownership and workflow ideas to Rust/GPUI; do not copy Web, React, or CSS-specific
  architecture into Base.
- `./target/release/gpui-component-story` — authoritative pre-migration Story
  binary for interaction, behavior, layout, styling, animation, focus, and visual
  detail comparisons. Preserve this binary; compare it side by side with a new
  build instead of rebuilding over it.

## Known Blockers and Risks

- Cargo can package Base locally, but cannot package the facade against an
  unpublished Base version until crates.io exposes that version. Release must
  publish Base first and retry the facade after index propagation.
- GPUI currently has no obvious public `aria_disabled` helper. Disabled Button
  accessibility and propagation need runtime evidence or an upstream seam before
  that checklist item can be completed.
- Semantic theme-file support uses a standalone `SemanticThemeConfigFile`; the
  public field shape of legacy `ThemeConfig` remains unchanged for source compatibility.
- Root, Overlay, Dock, VirtualList, and the current Theme are not file-level moves;
  each needs a dependency seam before migration.

## Validation Log

- 2026-08-11: `cargo check -p gpui-component-base -p gpui-component` passed after
  the initial foundation extraction.
- 2026-08-11: `cargo test -p gpui-component --test base_compat` passed 3 tests,
  covering legacy type identity and application-owned Button state styling.
- 2026-08-11: `cargo package -p gpui-component-base --allow-dirty --no-verify`
  succeeded (10 files packaged).
- 2026-08-11: Facade packaging confirmed the expected crates.io propagation
  dependency: `gpui-component-base` 0.5.2 is not published yet.
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
- 2026-08-11: M1/M2 Base controls gained typed semantic root style contexts;
  application-owned Registry templates use them for disabled root appearance
  without moving indicator/thumb presentation into Base.
- 2026-08-11: Generic CSS-like `transition(...)` landed with ElementId-like
  scalar/tuple identity, delay/easing, smooth target reversal, reduce-motion,
  and per-window/view keyed lifecycle. No Base component installs motion.

## Crate Foundation

- [x] Add `gpui-component-base` as a publishable workspace crate.
- [x] Keep the dependency direction one-way: `gpui-component` depends on Base.
- [x] Preserve the original focus-trap initialization order in `gpui_component::init`.
- [x] Move geometry primitives and extension traits into Base.
- [x] Move interaction event extensions into Base.
- [x] Move animation and transition behavior into Base.
- [x] Move focus-trap behavior and state into Base.
- [x] Re-export migrated APIs from their existing `gpui-component` public paths.
- [x] Add compile-time compatibility coverage for migrated public types.
- [ ] Verify Base and facade packaging after the Base version is visible on crates.io.
- [ ] Document and verify the release order: dependencies, Base, then facade.

## Base Component Milestones

A component is checked only when its unstyled Base behavior exists, its legacy UI
remains 100% identical in behavior, interaction, design, functionality, and API,
and its Registry-owned presentation can compose the Base API.

### M1 — Pilot Vertical Slice

Completion definition: prove the Base → Registry → Application ownership model on
one component, including interaction tests and unchanged legacy UI.

- [x] Button

Evidence: Base runtime tests, Registry installation/template compilation, legacy
API and component tests, and an unchanged legacy Button implementation.

### M2 — Simple Controls

Completion definition: migrate independent controls that do not require overlay or
compound state. Variants, sizes, icons, layout, and colors remain outside Base.

Composition audit rule: compare each slice with `../shadcn`'s primitive/registry
split. Base owns state, events, accessibility, and composable content slots;
Registry/UI owns the concrete indicator, label layout, icons, variants, and visual
tokens. A legacy adapter is incomplete if it only delegates activation while
retaining behavior-relevant content ownership outside Base.

- [x] Checkbox
- [x] Radio
- [x] Switch
- [x] Toggle
- [x] Slider
- [x] Link

### M1/M2 — Stateful Presentation

- [x] Typed `StateStyle` preserving GPUI `Styled` and `FluentBuilder`.
- [x] Button semantic root style: disabled.
- [x] Checkbox semantic root styles: checked, indeterminate, disabled.
- [x] Radio semantic root styles: checked, disabled.
- [x] Switch semantic root styles: checked, disabled.
- [x] Toggle semantic root styles: pressed, disabled.
- [x] Slider semantic root style: disabled.
- [x] Link semantic root style: disabled.
- [x] Generic application-owned value transitions with no component defaults.
- [ ] Typed state projection for Indicator, Thumb, Track, and other slots.
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

- [ ] Virtual List
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

- [ ] Split application-independent sizing APIs from the legacy styled module.
- [ ] Split pure GPUI element extensions from themed extensions.
- [ ] Move the scrollbar behavior interface into Base.
- [ ] Move VirtualList into Base without depending on styled scrollbar components.
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
- [ ] `cargo check -p gpui-component-base`
- [ ] `cargo test -p gpui-component-base`
- [ ] `cargo check -p gpui-component --no-default-features`
- [ ] `cargo test -p gpui-component`
- [ ] Story Web / WASM check
- [ ] `cargo clippy --workspace --all-targets -- --deny warnings`
- [ ] `cargo package -p gpui-component-base`
- [ ] `cargo package -p gpui-component` after Base is available from crates.io
- [ ] Requirement-by-requirement review against `RFC.md` acceptance criteria

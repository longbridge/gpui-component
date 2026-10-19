# NSH component coverage: composition and public render types

This internal review records how public Component render types reach NSH applications. A registration count is not a component acceptance criterion. A binding must construct the real component, preserve its required state, and expose useful data, events or composition. This document supplements `crates/nsh/catalog.json`; it does not claim complete Rust API parity.

The review compared the public structs/enums implementing `Render` or `RenderOnce` under `crates/component/src` with exports in `crates/nsh/js`, then inspected candidates without a same-name JavaScript export. Test harnesses, private modules and implementation-only renderers were checked separately. Generic implementations, macro-generated types and Base re-exports require manual review; this is a source review, not a formal proof of exhaustive API coverage.

## Direct display bindings

`crates/nsh/src/families/display.rs`, its `display/typed.rs` module and the compatibility exports in `crates/nsh/js/display.ts` provide these real bindings:

| Component module | NSH bindings |
| --- | --- |
| button | Button, ButtonGroup, Toggle, ToggleGroup, DropdownButton |
| checkbox, switch, radio | Checkbox, Switch, Radio, RadioGroup |
| icon | Icon |
| avatar | Avatar, AvatarGroup |
| badge, tag, alert | Badge, Tag, Alert |
| marker | Marker, MarkerIcon, MarkerContent |
| spinner, skeleton, shimmer | Spinner, Skeleton, ShimmerText |
| progress | Progress, ProgressCircle |
| separator, kbd, label, link | Separator, Kbd, Label, Link |
| clipboard, rating | Clipboard, Rating |

These are native Component renderers, not same-name placeholder aliases. Group options use stable values, with native per-item focus handles. Group item icons and tooltips map to the real child controls. Native children and named slots are exposed where the underlying control accepts them; Switch remains a leaf.

## Public types reached through parent components

A missing same-name JavaScript export does not by itself mean the component is unavailable. The following mappings were inspected in source. They describe composition coverage, not a promise that every builder is exposed.

| Public render type | NSH access and real implementation |
| --- | --- |
| ButtonIcon, ButtonIconVariant | `Button.icon` accepts an asset path, spinner descriptor or progress descriptor. `Display::button_icon` passes an actual Icon, Spinner or ProgressCircle into Button. These Rust types are also marked `doc(hidden)`; no dummy independent alias is counted. |
| IconName | Icon accepts an SVG asset path. The icon renderer is available; the Rust enum itself is not serialized or reproduced as a second JavaScript icon registry. |
| AccordionItem | Accordion `items` and item-id content slots construct real items through `Accordion::item` in `families/layout.rs`. |
| Tab | TabBar/Tabs `items` construct native Tab controls in `families/layout.rs`. Tabs additionally selects the matching live content slot. |
| BreadcrumbItem | Breadcrumb `items` construct native BreadcrumbItem children, including disabled state and activation events. |
| DescriptionText | DescriptionList creates native DescriptionItem label/value content; values may come from live slots. DescriptionText is the text/element sum type consumed by that parent, not a separately stateful control. |
| FieldBuilder | Form typed fields and Field composition use native field construction. FieldBuilder is the Rust construction helper for the same field presentation. |
| InputToken | Input `content.tokens` constructs real atomic input tokens; `token:<id>` slots customize their renderer. Token click/hover events are exposed. See `js/index.ts` and the retained Input adapter. |
| Scrollable | ScrollArea uses Component's `ScrollableElement` extension and real scrollbar behavior in `families/scrolling.rs`. The generic Rust wrapper is not exposed as a fake second control. |
| Text | TextView/Markdown/Html expose the rich-text renderer and retained state through `families/content.rs`. The legacy Text compatibility enum is not independently serialized. This does not promise all legacy Text builder options. |
| StepperItem | Stepper typed items and `step:<id>` slots construct native StepperItem in `extra_controls.rs`; selected/disabled state and activation reach the parent. |
| CarouselContent, CarouselItem | Carousel constructs the native content and item wrappers around item-id slots in `families/carousel.rs`. |
| CarouselPrevious, CarouselNext | Carousel `controls` enables the native previous/next controls; parent commands also operate its retained state. |
| CarouselPagination, CarouselPaginationItem | Carousel `pagination` enables native pagination and per-item controls. |
| QuestionnaireItem, QuestionnaireChoices, QuestionnaireChoice | Questionnaire typed definitions construct these real state-connected components; question-id and `id:value` slots accept live content. |
| QuestionnaireChoiceDescription | The default native QuestionnaireChoice renderer renders the choice definition's description. Custom choice children replace default content; no independent description binding is claimed. |
| QuestionnaireProgress, QuestionnaireInput, QuestionnaireError | Constructed with the retained QuestionnaireState by `families/questionnaire.rs`; input definitions and validation/error commands feed the native state. |
| QuestionnaireActions and macro-generated title/description/navigation parts | The retained adapter constructs QuestionnaireTitle, QuestionnaireDescription, QuestionnaireActions, Previous, Skip, Next and Submit. These are parent composition, not additional independent bindings. |
| SelectState, ComboboxState | Owned by the real retained Select/Combobox adapters (`src/select.rs`, `families/collections/picker.rs`). Applications provide serializable items/selection and receive events rather than construct Rust Entity values. |
| ListState, TableState, CommandState, DatePickerState | Retained by the corresponding real List/DataTable/Command/DatePicker adapters. State entities are implementation ownership, not separate visible components to count. |
| NotificationList | The Component window state constructs the actual notification host (`component/src/root.rs`); NSH Notification posts into that host. A second application-owned notification host is not required. |
| DropdownMenuPopover | Native DropdownMenu/DropdownButton use the real popup/menu machinery. The generic Rust trigger wrapper is not independently serialized. |
| plot::tooltip::CrossLine, Dot, Tooltip | Original native chart renderers use these overlays for hover/crosshair/point emphasis. NSH charts expose their native hover and optional tooltip content slots. There is no independent NSH binding for composing these three overlays into an arbitrary Plot; that lower-level composition remains outside the exposed API. |

The chart tooltip type above is distinct from the ordinary Component Tooltip binding.

## Exclusions and explicit limitations

| Type or family | Reason |
| --- | --- |
| DivInspector | Publicly re-exported only with the inspector feature or debug assertions. It is a developer tool, with no NSH inspector entry point. The catalog classifies the module as infrastructure; it must not be counted as a bound application component. |
| DragPanelPreview | Public re-export, but its field is private and it has no public constructor. The Component Dock skin constructs it during actual panel dragging (`dock/tab_panel.rs`). NSH Dock uses that native skin; no independent preview wrapper is invented. |
| BreadcrumbSeparator, StepperSeparator | Private Rust implementation types, automatically rendered by their parent controls. They are not public missing components. |
| Input CompletionMenu, CodeActionMenu, DiagnosticPopover, HoverPopover | Their containing `input::popovers` module is `pub(crate)`. Public-looking structs inside it are not public consumer entry points. This exclusion does not claim arbitrary NSH LSP-provider integration. |
| WindowState, WindowStateLayers, internal dock/table/list loading views | Window hosting, skins and internal composition are not counted as independently bound user components. Where public loading/row parts are exposed by collections, their actual bindings are listed in its JS/catalog entries. |
| Rust closures, trait implementations and arbitrary Entity values | JavaScript uses typed data, events, commands and live slots. The review does not claim a one-to-one wrapper for every Rust builder or callback. |

No ordinary end-user component that was completely absent from NSH was identified by this public-render candidate review. That conclusion is narrower than complete API parity and excludes the developer utility and independent low-level plot-overlay composition explicitly listed above.

## Verification evidence and boundaries

The display implementation was exercised through the actual bundled NSH runtime and native GPUI Component controls, not only through registration scanning.

- `NSH_VERIFY_FAMILY=display crates/nsh/target/debug/examples/verify` completed with exit code 0 after the shared verifier was built with `--features test-support`.
- The fixture covers eleven sequential Tab targets, Shift-Tab, disabled-item skipping, Radio and Clipboard keyboard activation, ToggleGroup pointer/Space activation, retained focus and selection after item reorder, ButtonGroup multiple selection, Rating controlled-value rejection/acceptance, Clipboard callbacks, Dropdown action/menu selection, disabled-event suppression and owner-driven updates.
- It opens the DropdownButton menu and unmounts its owner, then checks the menu is gone. It also renders display components with real nested content and checks native Progress semantics.
- `target/nsh-verify/display-controls.png` and `display-components.png` were generated and visually inspected for complete rendering without clipping. They are local generated artifacts, not committed reference images.
- `cargo test -p gpui-component rating::tests --lib --locked --offline -- --test-threads=1`: 4 passed.
- `cargo test -p gpui-component button::toggle::tests --lib --locked --offline -- --test-threads=1`: 9 passed, including native keyboard group callbacks.
- The display TypeScript entry bundled with Bun; the JavaScript fixture passed `node --check`.

These results validate the display family and the behaviors enumerated above. The cross-family mappings in this document were reviewed in source; this document does not convert them into claims that every listed component, platform or interaction has independently passed runtime testing. Consult each family's verifier and the maintained catalog/coverage notes for its own evidence. No additional Cargo run was performed for this documentation review.

## Initial typed registry prototype verification

The initial prototype migrated Button, Avatar and Skeleton to declare their props in Rust using NSH's existing `Props` derive; Button uses its existing `Event` derive. The platform's shared typed native wrapper decodes values and registers their specs in the real NSH Registry, which generates `nsh/ui/gpui` exports and declarations. Their old handwritten JS prop lists/interfaces are removed; the compatibility module imports/re-exports the generated types. At that verification point, other display controls still used the existing adapter.

Avatar and Skeleton use a small static macro that emits both the `Props` struct and the real `TypedNativeComponent` builder implementation from the same field declaration. It does not create a second schema or JS generator. Size and Button radius/icon unions remain precise; Button's focus, callbacks, children and native builder chain remain explicit. For the first two leaves this macro saves only one formatted source line after its shared implementation and documentation are counted. This prototype has not yet reduced the total display implementation size relative to the earlier dynamic adapter.

The generated-export fixture was run against the new shared build (`verify` artifact timestamp 1792339538): `NSH_VERIFY_FAMILY=display crates/nsh/target/debug/examples/verify` exited 0. The fixture imports Button, Avatar and Skeleton directly from `nsh/ui/gpui`, so this run exercises generated module exports and typed decoding rather than an independently handwritten `nativeComponent` spec for those three controls. The full existing display interaction assertions passed. Both screenshots were refreshed and visually inspected again without clipping. This is an end-to-end regression of the fixture's inputs and interactions, not exhaustive runtime testing of every optional Button prop.

A final regression also passed after the pure-Rust JS generator and metadata-only registration split (`verify` artifact timestamp 1792339922, display process exit 0). The fixture continued importing the same generated exports; no handwritten JS helper was needed for the three typed controls. This run preserves the earlier interaction assertions and does not expand the optional-prop coverage claim.

## Full display typed migration

All 29 display bindings now declare their props/events through the real NSH typed native registry. `display/remaining.rs` contains the remaining native builders and common retained group focus handling; the former dynamic string dispatcher, JSON property map and handwritten display JS prop/event specifications are removed. The compatibility TypeScript entry only re-exports generated values/types and derives three utility types with `Pick`. ComponentSize and ComponentColor retain named generated types; groups keep stable option values and validate item identity before accepting a candidate prop batch. DropdownButton reuses the shared typed menu values and renderer directly.

Simple leaves share the small static macro in `src/registration.rs`; stateful focus and event bodies remain ordinary Rust. This section records implementation scope, not a new runtime result. A fresh full generated-display regression is still required for this migration; the earlier successful runs above belong to their stated revisions.

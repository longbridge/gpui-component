---
name: gpui-component
description: How to use the gpui-component UI library in GPUI applications, and the normative Design and Coding Guides that govern it. Use when building UIs with gpui-component components (Button, Input, Select, Dialog, Tabs, Sidebar, List, Table, etc.), setting up the library, handling component state or theming, finding the right component for a UI need, and also when designing layouts, spacing, visual hierarchy, or interaction states, writing interface copy, or making application architecture, state-ownership, or public API decisions.
---

## Read This First

Before changing UI, interaction, interface language, layout, styling,
components, or application architecture, **read the relevant guide**:

| Guide | Read before |
| --- | --- |
| [Design Guides](references/design-guides.md) | Choosing components, layout, spacing, hierarchy, color, density, interaction states, overlays, interface copy |
| [Coding Guides](references/coding-guides.md) | Crate layering, `RenderOnce` vs `Entity<T>`, state ownership, `ElementId`, focus, async, public API, testing |

These guides are requirements, not optional inspiration. Do not copy generic
web conventions, infer a design system from one existing screen, or add a
control merely because the underlying feature exists. Review the finished work
against both guides before considering it complete.

Read the guide file itself. Do not answer from what this page summarizes, from
an existing screen in the codebase, or from training data — those are the three
ways the guides get quietly ignored.

### Non-negotiables

These are a floor, not a substitute. Read the guide for anything past this list.

- **Never invent an API.** Search the current source for the real signature.
  Do not translate a React, CSS, or older-GPUI example by analogy — a
  plausible-looking method name that does not exist is the most common
  failure mode here.
- **Desktop before web convention.** Keyboard access, window chrome, menus,
  dense data views, resizable regions, persistent navigation.
- **`Button` vs `Link`.** `Button` for every in-app command — use `ghost` or
  `outline` when it should read quietly. `Link` only for external URLs and
  email addresses.
- **Tokens before values.** No raw hex or `rgb(...)` in application UI; use
  `cx.theme()` semantic tokens. Use rem-based helpers (`p_2()`, `gap_3()`,
  `text_sm()`) so window zoom works. Any spacing number you see quoted is the
  current default scale, not a literal to repeat.
- **State must be visible.** Hover, focus, selection, disabled, loading,
  validation, and destructive states each need distinct, consistent treatment.
- **Stable identity.** Repeated elements need domain-derived `ElementId`s, not
  list indexes.
- **Overlays.** Escape dismisses the topmost surface and returns focus to its
  trigger.
- **Copy.** Name the object and the verb — `Delete “Roadmap”?` with a `Delete`
  button, not `Are you sure?` with `OK`.

## Documentation

- **Full reference**: fetch `https://longbridge.github.io/gpui-component/llms-full.txt`
- **Per-component API**: fetch `https://longbridge.github.io/gpui-component/docs/components/{name}.md`
  - e.g. `button.md`, `input.md`, `select.md`, `dialog.md`, `data-table.md`
- **Any site page** can be fetched as Markdown by appending `.md` to the URL

## Quick Reference

**Setup** — always required:
```rust
gpui_component::init(cx);               // in app.run(), must be first
Root::new(view, window, cx)             // first-level view in every window
```

**Stateless** — use directly in render:
```rust
Button::new("id").primary().label("OK").on_click(|_, _, _| {})
```

**Stateful** — hold `Entity<State>` in struct, pass ref in render:
```rust
// in new():  let input = cx.new(|cx| InputState::new(window, cx));
// in render: Input::new(&self.input)
```

**Sizes**: `.xsmall()` `.small()` `.medium()` (default) `.large()`

**Theme**: `cx.theme().primary` · `.background` · `.foreground` · `.border` · `.muted`

## Component Catalog

When you need a component, find it here. For full API, fetch its `.md` doc.

### Input & Form
| Component | Import | Notes |
|-----------|--------|-------|
| `Input` | `input::{Input, InputState}` | Stateful. Text, password, mask, validation |
| `NumberInput` | `input::{NumberInput, NumberInputEvent}` | Stateful. Numeric with step |
| `OtpInput` | `input::OtpInput` | Stateful. One-time password |
| `Select` | `select::{Select, SelectState}` | Stateful. Dropdown picker |
| `Combobox` | `combobox::{Combobox, ComboboxState}` | Stateful. Searchable select |
| `Checkbox` | `checkbox::Checkbox` | Stateless. `on_click(|&bool, ...|)` |
| `Switch` | `switch::Switch` | Stateless. Toggle |
| `Radio` | `radio::{Radio, RadioGroup}` | Stateless. |
| `Slider` | `slider::{Slider, SliderState}` | Stateful. |
| `Toggle` | `button::Toggle` | Stateless. |
| `Rating` | `rating::Rating` | Stateless. |
| `Stepper` | `stepper::Stepper` | Stateless. Increment/decrement |
| `ColorPicker` | `color_picker::{ColorPicker, ColorPickerState}` | Stateful. |
| `DatePicker` | `date_picker::{DatePicker, DatePickerState}` | Stateful. |
| `Form` | `form::{v_form, h_form, field}` | Layout container for form fields |

### Display & Feedback
| Component | Import | Notes |
|-----------|--------|-------|
| `Button` | `button::{Button, ButtonGroup}` | Stateless. Primary UI action |
| `Icon` | `{Icon, IconName}` | Stateless. Lucide icons |
| `Badge` | `badge::Badge` | Stateless. |
| `Tag` | `tag::Tag` | Stateless. Closable tags |
| `Avatar` | `avatar::Avatar` | Stateless. |
| `Label` | `label::Label` | Stateless. Form label |
| `Kbd` | `kbd::Kbd` | Stateless. Keyboard key display |
| `Alert` | `alert::Alert` | Stateless. Info/success/warning/error |
| `Spinner` | `spinner::Spinner` | Stateless. Loading indicator |
| `Skeleton` | `skeleton::Skeleton` | Stateless. Loading placeholder |
| `Progress` | `progress::{Progress, ProgressCircle}` | Stateless. |
| `Tooltip` | `tooltip::Tooltip` | Via `.tooltip()` on elements |
| `HoverCard` | `hover_card::{HoverCard, HoverCardState}` | Stateful. |
| `Clipboard` | `clipboard::Clipboard` | Stateless. Copy button |

### Overlay & Popups
| Component | Import | Notes |
|-----------|--------|-------|
| `Dialog` | `dialog::Dialog` + `WindowExt` | Via `window.open_dialog(...)` |
| `AlertDialog` | `WindowExt` | Via `window.open_alert_dialog(...)` |
| `Sheet` | `sheet::Sheet` + `WindowExt` | Side panel, via `window.open_sheet(...)` |
| `Notification` | `notification::Notification` + `WindowExt` | Via `window.push_notification(...)` |
| `Popover` | `popover::Popover` | Floating overlay |
| `Menu` | `menu::{PopupMenu, DropdownMenu}` | Context menus |
| `DropdownButton` | `button::DropdownButton` | Button with dropdown menu |

### Navigation & Layout
| Component | Import | Notes |
|-----------|--------|-------|
| `Tabs` / `TabBar` | `tab::{Tab, TabBar}` | Tabbed interface |
| `Sidebar` | `sidebar::{Sidebar, SidebarMenu, ...}` | App navigation panel |
| `TitleBar` | `TitleBar` | Window title bar |
| `Breadcrumb` | `breadcrumb::Breadcrumb` | Navigation breadcrumb |
| `Pagination` | `pagination::Pagination` | Page navigation |
| `Accordion` | `accordion::Accordion` | Collapsible sections |
| `Collapsible` | `collapsible::Collapsible` | Single collapsible |
| `GroupBox` | `group_box::GroupBox` | Labeled container |
| `Resizable` | `resizable::{h_resizable, v_resizable, resizable_panel, ResizableState}` | Draggable split panes |
| `Scrollable` | `scroll::Scrollbar` | Custom scrollbar |
| `FocusTrap` | `gpui_base::focus_trap::FocusTrapElement` | Keyboard trap for modals |

### Data Display
| Component | Import | Notes |
|-----------|--------|-------|
| `DataTable` | `table::{DataTable, TableState, TableDelegate}` | Stateful. Full-featured table |
| `Table` | `table::{Table, ...}` | Simpler table |
| `VirtualList` | `{v_virtual_list, h_virtual_list}` | High-perf large lists |
| `List` | `list::{List, ListState, ListDelegate}` | Stateful. Searchable list |
| `Tree` | `tree::{Tree, TreeState, TreeItem, TreeEntry}` | Stateful. Hierarchy |
| `DescriptionList` | `description_list::DescriptionList` | Key-value pairs |
| `Settings` | `setting::Settings` | Settings panel |

### Charts
| Component | Import | Notes |
|-----------|--------|-------|
| `Chart` | `chart::{AreaChart, BarChart, LineChart, PieChart, RadarChart}` | Bar, line, area, pie charts |
| `Plot` | `plot::Plot` | `#[derive(IntoPlot)]` for data |

## Reference Files

- [usage.md](references/usage.md) — setup patterns, component types, common examples
- [style-guide.md](references/style-guide.md) — code style for contributors

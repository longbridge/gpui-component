---
name: gpui-component
description: How to use the gpui-component UI library in GPUI applications. Use when building UIs with gpui-component components (Button, Input, Select, Dialog, Tabs, Sidebar, List, Table, etc.), setting up the library, handling component state, theming, or finding the right component for a given UI need.
---

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
| `NumberInput` | `number_input::{NumberInput, NumberInputState}` | Stateful. Numeric with step |
| `OtpInput` | `otp_input::{OtpInput, OtpInputState}` | Stateful. One-time password |
| `Select` | `select::{Select, SelectState}` | Stateful. Dropdown picker |
| `Combobox` | `combobox::{Combobox, ComboboxState}` | Stateful. Searchable select |
| `Checkbox` | `checkbox::Checkbox` | Stateless. `on_click(|&bool, ...|)` |
| `Switch` | `switch::Switch` | Stateless. Toggle |
| `Radio` | `radio::{Radio, RadioGroup}` | Stateless. |
| `Slider` | `slider::{Slider, SliderState}` | Stateful. |
| `Toggle` | `toggle::Toggle` | Stateless. |
| `Rating` | `rating::Rating` | Stateless. |
| `Stepper` | `stepper::Stepper` | Stateless. Increment/decrement |
| `ColorPicker` | `color_picker::{ColorPicker, ColorPickerState}` | Stateful. |
| `DatePicker` | `time::date_picker::{DatePicker, DatePickerState}` | Stateful. |
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
| `Progress` | `progress::{ProgressBar, ProgressCircle}` | Stateless. |
| `Tooltip` | `tooltip::Tooltip` | Via `.tooltip()` on elements |
| `HoverCard` | `hover_card::{HoverCard, HoverCardState}` | Stateful. |
| `Image` | `image::Image` | Stateless. |
| `Clipboard` | `clipboard::Clipboard` | Stateless. Copy button |

### Overlay & Popups
| Component | Import | Notes |
|-----------|--------|-------|
| `Dialog` | `dialog::Dialog` + `WindowExt` | Via `window.open_modal(...)` |
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
| `TitleBar` | `title_bar::TitleBar` | Window title bar |
| `Breadcrumb` | `breadcrumb::Breadcrumb` | Navigation breadcrumb |
| `Pagination` | `pagination::Pagination` | Page navigation |
| `Accordion` | `accordion::Accordion` | Collapsible sections |
| `Collapsible` | `collapsible::Collapsible` | Single collapsible |
| `GroupBox` | `group_box::GroupBox` | Labeled container |
| `Resizable` | `resizable::Resizable` | Draggable split panes |
| `Scrollable` | `scroll::Scrollbar` | Custom scrollbar |
| `FocusTrap` | `focus_trap::FocusTrap` | Keyboard trap for modals |

### Data Display
| Component | Import | Notes |
|-----------|--------|-------|
| `DataTable` | `table::{DataTable, TableState, TableDelegate}` | Stateful. Full-featured table |
| `Table` | `table::{Table, ...}` | Simpler table |
| `VirtualList` | `{v_virtual_list, h_virtual_list}` | High-perf large lists |
| `List` | `list::{List, ListState, ListDelegate}` | Stateful. Searchable list |
| `Tree` | `tree::{Tree, TreeState, TreeDelegate}` | Stateful. Hierarchy |
| `DescriptionList` | `description_list::DescriptionList` | Key-value pairs |
| `Settings` | `settings::Settings` | Settings panel |

### Charts
| Component | Import | Notes |
|-----------|--------|-------|
| `Chart` | `chart::Chart` | Bar, line, area, pie charts |
| `Plot` | `plot::Plot` | `#[derive(IntoPlot)]` for data |

## Reference Files

- [usage.md](references/usage.md) — setup patterns, component types, common examples
- [style-guide.md](references/style-guide.md) — code style for contributors

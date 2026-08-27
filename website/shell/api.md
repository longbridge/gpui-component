---
title: API Reference
description: Every name a script can import or reach — the three built-in modules, the cx and window globals, and the element methods that are not styles.
order: 13
---

# API Reference

An inventory of the script surface: what exists, and which module it comes from. The other pages explain why each thing works the way it does — this one is for looking a name up.

The authority is not this page. `gpui-shell` rewrites a `gpui.d.ts` beside your source on every run, generated from the runtime that is about to execute the script, so a committed copy could only ever be the stale one. Put `// @ts-check` at the top of a script to have an editor check against it.

## The modules

There is one module per crate that provides the capability, so an import says which layer a script depends on. A name belongs to exactly one of them: nothing is re-exported for convenience, because a name reachable from two specifiers stops saying where it came from.

| Module | Provides |
| --- | --- |
| `"gpui"` | GPUI's own elements, plus what this runtime adds: views, the style surface, storage, scheduling, native modules |
| `"gpui-base"` | `gpui-base`'s layout helpers, components and theme |
| `"gpui-fps"` | `gpui-fps`'s performance overlay |

Two names are ambient and are not imported at all: `cx`, which every host call hands you, and `window`. The standard-runtime modules — `fs/promises`, `path`, `crypto`, `process`, `net`, `websocket` and the rest — are gated by the host's grant and are documented in [Capabilities](./capabilities.md).

Each table below names a thing exactly as you write it. A lowercase function is a free function on the Rust side too — `div()` is `gpui::div()`. A capitalized name is an object whose only member is a factory, mirroring the associated function it is named after: `Button.new(id)` is `Button::new(id)`, and `TableRow.new(id, index)` carries the one-based position a screen reader announces.

## The `gpui` module

### Elements

| Name | What it is |
| --- | --- |
| `div()` | An element with no layout of its own |
| `svg(path)` | A vector image from the application root, tinted by the surrounding text color |
| `image(path)` | A full-color image from the application root, colors preserved |
| `PathBuilder` | `fill()` and `stroke(width)`, each opening a path under construction |
| `Background` | `solid`, `stop`, `linear_gradient`, `pattern_slash`, `checkerboard` |

`PathBuilder.fill()` and `.stroke(width)` return a handle that chains `move_to`, `line_to`, `curve_to`, `cubic_bezier_to`, `arc_to`, `add_polygon`, `close` and `dash_array`, and ends in `build()`. Paint the result with `window.paint_path(path, background)` — the one element constructor reached through an object, because the thing it mirrors is a method on the window.

A string is an element too, exactly as `&str` implements `IntoElement` in GPUI: `.child("hello")` is how text is written, and the style comes from the element holding it.

### Views

| Name | What it is |
| --- | --- |
| `View` | The base class of every view; subclass it and default-export the subclass |
| `ViewClass` | A concrete `View` subclass, as `cx.new` takes it |
| `Entity` | Retained ownership of one nested view: `set_props(props)`, `release()` |
| `Props` | The property bag handed to `init` and to `cx.new` |

A subclass defines `init?(props, cx)`, which runs once, and `render(cx)`, which returns exactly one element and runs when the view is invalidated. An optional `update(props)` runs when a parent changes a nested view's props.

### Storage

| Name | What it is |
| --- | --- |
| `store` | Key-value storage that survives a restart, persisted on every write |
| `Store` | `get(key)`, `set(key, value)`, `remove(key)`, `keys()`, `flush()` |
| `Json` | Everything the store can persist, and nothing else |

`store.get` answers `null` for an unset key, and `flush()` completes once the current value is durably written.

### Native modules

| Name | What it is |
| --- | --- |
| `native(module)` | A module the host registered in Rust; throws and names the ones that exist when it finds none |
| `NativeModules` | Empty here — an application declares its own modules into it and `native("…")` becomes typed |

### Scheduling

| Name | What it is |
| --- | --- |
| `Task` | A running task: `cancel()`, `is_done()` |
| `TaskOptions` | `owner` — the view the task is cancelled with, or `null` to outlive every view |
| `Timer` | `after(ms, handler, opts?)` and `every(ms, handler, opts?)` |

### Focus and component shapes

| Name | What it is |
| --- | --- |
| `FocusHandleHandle` | A focus target the script owns: `focus()`, `is_focused()`, `release()` |
| `ComponentType` | `new(id)` — a component identified across renders |
| `PartType` | `new()` — a sub-part with no identity of its own |
| `IndexedComponentType` | `new(id, index)` — a component whose one-based position is announced |

### Shared types

| Name | What it is |
| --- | --- |
| `Length` | A number (pixels), `"12px"`, `"1.5rem"`, `"50%"` or `"auto"` |
| `DefiniteLength` | The same without `"auto"` |
| `AbsoluteLength` | Pixels or rems only |
| `LengthString` | The string forms of a length |
| `Color` | A `ColorToken` name, or a `#rgb` / `#rrggbb` / `#rrggbbaa` literal |
| `ColorToken` | The seventeen semantic tokens the installed palette defines |
| `Role` | An accessibility role, mirroring `gpui::Role` in snake_case |
| `Anchor` | Which corner of an anchored surface is pinned to its trigger |
| `MouseButton` | `"left"`, `"right"` or `"middle"` |
| `Phase` | `"render"`, `"event"`, `"task"`, `"layout"` or `"none"` |
| `SheetSide` | Which edge the sheet is anchored to |
| `DialogOptions` | `escape_dismissable`, `backdrop_dismissable` |
| `ToastOptions` | `title`, `description`, `level`, `timeout`, `id` |
| `ClickEvent` | `click_count`, `modifiers` |
| `MouseMoveEvent` | `position`, `local_position`, `bounds`, `modifiers` |
| `Modifiers` | `shift`, `control`, `alt`, `platform` |
| `Point` | `x`, `y` |
| `ElementBounds` | A `Point` with `width` and `height` |
| `MotionProperty` | `"opacity"`, `"width"`, `"height"`, `"left"`, `"top"` |
| `MotionEasing` | `"linear"`, `"ease-in"`, `"ease-out"`, `"ease-in-out"` |
| `TransitionPolicy` | `duration`, `delay`, `easing` |
| `SpringPolicy` | `response`, `damping`, `epsilon` |
| `Path` | Immutable native geometry produced by `PathBuilder.build()` |
| `PathCoordinate` | Pixels, or a percentage of the painted element's bounds |
| `BackgroundValue` | A reusable native background: `opacity(factor)`, `color_space(space)` |
| `BackgroundStop` | One gradient stop, from `Background.stop(color, percentage)` |

## The `cx` context

`cx` is the script-side context for one host call, and it is valid only for that call. An `await` returns to the host and the frame it names goes away, so a `cx` kept across one reports a stale-context error.

| Member | What it is |
| --- | --- |
| `notify()` | Requests a re-render; throws during `render`, because notifying yourself while rendering is a loop |
| `phase()` | Which `Phase` the call is in |
| `theme()` | The current `gpui_base::Theme` semantic token projection |
| `open_url(url)` | Hands an absolute `http`/`https` URL to the system handler |
| `read_from_clipboard()` | The clipboard's text, or `undefined` when it holds none |
| `write_to_clipboard(text)` | Replaces the clipboard's text |
| `focus_handle()` | A new `FocusHandleHandle`; belongs in `init` or an event handler, never in `render` |
| `new(Class, props?)` | Creates a retained nested view and answers the `Entity` that owns it |
| `spawn(body, opts?)` | Runs `body(cx)` and adopts the promise it returns, so a rejection is reported |
| `sleep(ms?)` | Resolves after `ms` on GPUI's foreground executor |
| `timer` | The `Timer`: `after` and `every` |

Several of these name the GPUI method they mirror: `open_url` is `App::open_url`, `read_from_clipboard` and `write_to_clipboard` are `App::read_from_clipboard` and `App::write_to_clipboard`, `focus_handle` is `App::focus_handle` (GPUI has no `FocusHandle::new`, and neither does this), `new` is `AppContext::new`, and `spawn` is `App::spawn`.

### `AsyncContext`

`AsyncContext` extends `Context` and adds no members. The difference is lifetime, not surface: an ordinary `Context` speaks for one host call and reports clearly once that call has returned, while an `AsyncContext` names no call at all — it resolves whichever is running when a member is used, and refuses only when none is. It is the mirror of GPUI's `AsyncApp`.

Three places hand one out: `init`, the body of `cx.spawn`, and the callbacks of `cx.timer`. Those are the three whose job is to set up or continue work that outlives the call it was started from.

## The `window` global

A global, like `cx`; nothing to import. Every call reads the host call that is running now and throws outside one, so there is no handle to hold and nothing that can go stale. An overlay belongs to the window rather than to the view that opened it, which is why these are here and not on `Context`.

| Member | What it is |
| --- | --- |
| `open_dialog(content, options?)` | Opens a dialog and answers the stack's new depth |
| `close_dialog()` | Closes the topmost dialog, and answers whether it found one |
| `close_all_dialogs()` | Closes every dialog, and answers how many |
| `has_active_dialog()` | Whether any dialog is open; legal from `render`, unlike the rest |
| `open_sheet(content)` | Opens the sheet on the right, replacing whatever was there |
| `open_sheet_at(side, content)` | The same, anchored to the side you name |
| `close_sheet()` | Closes the sheet, and answers whether one was open |
| `has_active_sheet()` | Whether the sheet is open; legal from `render` |
| `push_toast(options)` | Posts a toast and answers its id |
| `remove_toast(id)` | Retracts one toast, and answers whether it was still showing |
| `clear_toasts()` | Retracts every toast, and answers how many |
| `paint_path(path, background)` | Paints immutable geometry with a native background; `Window::paint_path` |

`open_dialog`, `open_sheet` and `open_sheet_at` take a **function returning an element**, not an element: a dialog outlives the call that opened it, and the function runs again whenever it redraws. Everything here except the two `has_active_*` queries and `paint_path` is illegal from `render`. See [Overlays](./overlays.md).

## The `gpui-base` module

The components here own behavior, focus and what a screen reader hears, and draw next to nothing themselves. The picture is the script's, written with the [style surface](./styling.md).

### Layout

| Name | What it is |
| --- | --- |
| `h_flex()` | A row |
| `v_flex()` | A column |
| `h_resizable(id)` | A row of panes with draggable dividers; sizes live in the window under the id |
| `v_resizable(id)` | The same, stacked |
| `resizable_panel()` | One pane of a resizable group, and legal nowhere else |

### Controls

| Name | What it is |
| --- | --- |
| `Button.new(id)` | Activation, focus, disabled and selected state |
| `Link.new(id)` | An external HTTP(S) resource opened through the system browser |
| `Checkbox.new(id)` | A controlled toggle; draw the indicator yourself |
| `Switch.new(id)` | A controlled switch |
| `Radio.new(id)` | One option in a group; reports `true` only, never a deselection |
| `Toggle.new(id)` | A button that stays down |
| `RadioGroup.new(id)` | A set of radios announced as one group; holds no selection |
| `ToggleGroup.new(id)` | A set of toggles announced as a toolbar |
| `Tabs.new(id)` | A tab list that holds no selection of its own |
| `Tab.new(id)` | One tab: `selected(...)` in, `on_click(...)` out |
| `Progress.new(id)` | The announcement, not the bar; `Progress.new(...)` alone draws nothing |
| `ProgressTrack.new()` | The groove: a plain element you size and color |
| `ProgressIndicator.new()` | The filled part; set its width from the percentage you announced |
| `SliderState.new(options?)` | Retained slider state, and where a drag writes |
| `Slider.new(state)` | The root: announces the value and owns the release |
| `SliderTrack.new(state)` | The press and drag surface |
| `SliderIndicator.new(state)` | The groove, and the box every pointer position is measured against |
| `SliderThumb.new(state)` | The knob; the shell gives it a place, you give it a look |

All four slider parts take the same `SliderStateHandle`, and all four are needed — a slider with no `SliderIndicator` cannot be moved at all.

### Text editing

| Name | What it is |
| --- | --- |
| `InputState.new(options?)` | Retained text state: `InputState.new({ placeholder, value })` |
| `Input.new(state)` | The frame around retained text state |
| `NumberInput.new(state)` | A spinbutton over the same `InputState`, with three slots that all carry weight |
| `TextareaState.new(options?)` | Retained multi-line text state; `rows` is an option |
| `Textarea.new(state)` | The frame around retained multi-line state |
| `OtpState.new(length, options?)` | Retained one-time-code state; the length is fixed when it is created |
| `OtpInput.new(state)` | A fixed-length code whose cells the shell draws and the script styles |

There is no numeric state type: an `InputState` becomes a number state by being given `set_step`, `set_min` and `set_max`.

### Containers and overlays

| Name | What it is |
| --- | --- |
| `Collapsible.new()` | Renders its `content` slot only while `open`; no role, chevron or trigger |
| `Popover.new(id)` | A surface anchored to a trigger and opened by a press |
| `HoverCard.new(id)` | The same, opened by resting the pointer, with its own open state |
| `Popup.new(id, trigger)` | The bare anchored surface: `Popup.new(id, trigger)`, opened by filling `content` |
| `Select.new(id)` | A combobox root: the role, the announced open state, the keyboard — none of the picture |
| `Combobox.new(id)` | The same root, announced as a combobox whose trigger is an editable field |
| `DatePicker.new(id, focus_handle)` | A date-picker root: `DatePicker.new(id, focus_handle)`; it holds no date |

Two gaps are worth knowing before you build on these: arrow-key navigation of an open `Select` or `Combobox` list is not there, and Enter and Escape do not reach a `DatePicker`. Both are described where they bite, in the declarations for each type.

### Tables and lists

| Name | What it is |
| --- | --- |
| `Table.new(id)` | A semantic table root, composed the way HTML composes one |
| `TableHeader.new(id)` | The header row group |
| `TableBody.new(id)` | The body row group |
| `TableRow.new(id, index)` | One row: `TableRow.new(id, row_index)`, one-based |
| `TableHead.new(id, index)` | One column header, one-based |
| `TableCell.new(id, index)` | One data cell, one-based |
| `TableCaption.new(id)` | The visual slot a caption belongs in; it carries no caption role |
| `v_virtual_list(…)` | A vertical list that describes only what is on screen |
| `h_virtual_list(…)` | The same along the other axis; `item_sizes` are widths |
| `VirtualListScrollHandle.new()` | A virtual list's scroll position, kept across frames |
| `Scrollbar.new(id)` | `new(id)`, `horizontal(id)`, `vertical(id)` — a bar you place yourself |

Both virtual lists take `(id, item_count, item_sizes, get_key, render)`. `render(range, cx)` is the only callback in this API that the host calls *during* a frame, which is why handlers, retained state and `cx.notify()` are all refused inside it.

### Retained handles

Each of these is created once — in `init` or an event handler, never in `render` — and released with `release()`.

| Handle | Members |
| --- | --- |
| `InputStateHandle` | `value`, `set_value`, `on("change" \| "submit" \| "focus" \| "blur")`, `set_step`, `set_min`, `set_max`, `set_masked`, `set_loading` |
| `TextareaStateHandle` | `value`, `set_value`, `on(…)`, `set_rows`, `set_auto_grow`, `set_soft_wrap` |
| `SliderStateHandle` | `value`, `set_value`, `min_value`, `max_value`, `step_value`, `on("change" \| "release")` |
| `OtpStateHandle` | `value`, `set_value`, `len`, `is_masked`, `set_masked`, `focus`, `on("change" \| "focus" \| "blur")` |
| `VirtualListScrollHandleHandle` | `scroll_to_item(index, strategy?)`, `scroll_to_bottom` |

### Theme

| Name | What it is |
| --- | --- |
| `set_theme(theme)` | Replaces `gpui-base`'s active semantic tokens with an application-owned theme |
| `Theme` | What `cx.theme()` answers: the semantic tokens plus `appearance` and `is_dark` |
| `SemanticThemeTokens` | `colors`, `spacing`, `radius` |
| `ColorTokens` | One `Color` per semantic role |
| `SpacingTokens` | `xxs` `xs` `sm` `md` `lg` `xl` `xxl` |
| `RadiusTokens` | `none` `sm` `md` `lg` `xl` `full` |

Reading the theme is `cx.theme()`. Replacing the whole palette is an application-level act with no context to speak of, which is why `set_theme` is a free function.

### Other types

| Name | What it is |
| --- | --- |
| `GroupAxis` | `"horizontal"` or `"vertical"`, announced rather than drawn |
| `ScrollbarMode` | `"scrolling"`, `"hover"` or `"always"` |
| `ItemRange` | A virtual list's visible items, as a half-open `[start, end)` |
| `SliderValue` | A number, or `[start, end]` for a range slider |
| `PopupType`, `DatePickerType`, `ScrollbarType` | The factory shapes of the three types whose constructor takes more than an id |

## The `gpui-fps` module

| Name | What it is |
| --- | --- |
| `fps_monitor()` | The native `gpui-fps` HUD, shared once per window and pinned to the top right |

Its parent must be `relative()`. The HUD owns its own presentation; ordinary styles and children do not apply to it.

## Element methods

Every element shares one prototype, so every method below type-checks on every element — which component a method actually suits is not expressed by the types. A behavior builder handed to a component that does not honour it is reported in the log rather than dropped in silence.

Every method answers the same element, so a chain is one expression. An element is consumed when it is used as a child and belongs to the render pass that built it.

### Composition

| Method | What it does |
| --- | --- |
| `child(value)` | Adds one child: an element, an `Entity`, or a string, number or boolean |
| `children(iterable)` | Adds several, in order |
| `when(condition, branch)` | Applies `branch` when `condition` is truthy, keeping the chain in one piece |
| `id(name)` | A stable name for this element, used as its identity |

### Slots

A slot is not a child: the element is consumed by the component and rendered where the component decides.

| Method | What it does |
| --- | --- |
| `content(element)` | The content of a `Collapsible`, `Popover`, `HoverCard` or `Popup` |
| `trigger(element)` | The trigger of a `Popover` or `HoverCard` |
| `input(element)` | The editor slot of a `NumberInput`; empty draws the bare editor |
| `decrement_button(element)` | The look of a `NumberInput`'s decrement button — replayed onto base's button, not rendered |
| `increment_button(element)` | The increment button, replayed the same way |
| `controls_right()` | Stacks both step buttons to the right of the text |

### Events

| Method | What it delivers |
| --- | --- |
| `on_click(handler)` | `(ClickEvent, cx)` on activation |
| `on_mouse_move(handler)` | `(MouseMoveEvent, cx)` while the element is hovered |
| `on_hover(handler)` | `(hovered, cx)` on both pointer entry and exit |
| `on_change(handler)` | `(checked, cx)` on a toggle; the script owns the new value |
| `on_step(handler)` | `("increment" \| "decrement", cx)`, and it **replaces** built-in stepping |
| `on_item_click(handler)` | `(key, cx)` when a virtual list row is clicked, keyed rather than indexed |
| `on_open_change(handler)` | `(open, cx)` when something other than the script changed a `Popover`'s open state |
| `on_confirm(handler)` | Enter in an open `Select` or `Combobox`; no payload |
| `on_dismiss(handler)` | Escape in an open `Select` or `Combobox`, before `on_open_change(false)` |
| `on_resize(handler)` | `(sizes, cx)` once a resizable group's drag has ended |

### Control state

| Method | What it sets |
| --- | --- |
| `disabled(value)` | Blocks activation and reports the state; draw it yourself |
| `selected(value)` | The selected state of a `Button` |
| `checked(value)` | The controlled value of a `Checkbox`, `Switch` or `Radio` |
| `pressed(value)` | The controlled state of a `Toggle` |
| `value(percent)` | The announced progress percentage, clamped to `0..=100`; it moves nothing on screen |
| `indeterminate(value)` | Withdraws a `Progress` value from the accessibility tree |
| `open(value)` | Whether a `Collapsible` renders its content, or a surface is showing |
| `default_open(value)` | Whether an uncontrolled `Popover` starts open |
| `start(value)` | Which thumb of a range slider a `SliderThumb` is |
| `href(url)` | The absolute HTTP(S) target of a `Link` |

### Accessibility

| Method | What it announces |
| --- | --- |
| `accessibility_label(text)` | What a screen reader says; an icon-only control announces nothing without it |
| `role(name)` | What this element announces itself as — plain elements, `Button` and `Checkbox` only |
| `aria_selected(value)` | The selected state of an option in a list the script built |
| `aria_active_descendant()` | This element as the focused one while an ancestor holds the keyboard |
| `set_position(position, size)` | One-based position and total size — "tab 2 of 5" |
| `row_count(count)` | A `Table`'s total rows, including unrendered ones |
| `column_count(count)` | A `Table`'s total columns |
| `axis(value)` | A `RadioGroup`'s or `ToggleGroup`'s orientation; semantic only, it lays out nothing |
| `tooltip(text)` | A pointer-only hover label, and no substitute for `accessibility_label` |

### Focus and keyboard

| Method | What it does |
| --- | --- |
| `track_focus(handle)` | Makes this element what the handle means |
| `content_focus_handle(handle)` | Where a `Select` or `Combobox` moves the keyboard when it opens |
| `tab_index(index)` | Where this element sits in the Tab order; it also makes it a tab stop |
| `tab_stop(value)` | Whether Tab can land here, without changing its place in the order |

### Scrolling and panels

| Method | What it does |
| --- | --- |
| `overflow_scroll()` | Owns wheel and touch scrolling on both axes |
| `overflow_x_scroll()` / `overflow_y_scroll()` | The same on one axis |
| `overflow_scrollbar()` | Scrolls both axes and paints base-layer bars |
| `overflow_x_scrollbar()` / `overflow_y_scrollbar()` | The same on one axis |
| `mode(value)` | A `Scrollbar`'s visibility policy; omitted, it follows the theme |
| `scroll_size(width, height)` | The content size a `Scrollbar` measures its thumb against |
| `viewport_from_layout()` | Makes a `Scrollbar` take its viewport from its own box |
| `track_scroll(handle)` | Gives a virtual list a scroll position the script can drive |
| `with_item_to_measure_index(index)` | Which item a virtual list measures across the axis it scrolls |
| `size_range(min, max?)` | How far a `resizable_panel()` may be dragged, in pixels |

### Anchored surfaces

| Method | What it sets |
| --- | --- |
| `anchor(value)` | Which corner is pinned to the trigger; clamped into the window either way |
| `mouse_button(value)` | Which pointer button opens a `Popover` |
| `open_delay(ms)` | How long the pointer must rest on a `HoverCard` trigger; default 600 |
| `close_delay(ms)` | How long a `HoverCard` waits before closing; default 300 |
| `overlay_closable(value)` | Whether pressing outside an open `Popover` closes it |

### Motion

| Method | What it does |
| --- | --- |
| `transition(property, policy)` | Animates later target changes entirely in native GPUI code |
| `spring(property, policy?)` | Springs them instead |

The property is one of `"opacity"`, `"width"`, `"height"`, `"left"`, `"top"`, and the frames never enter JavaScript.

### Style templates

Each takes a function that receives a detached element to collect styles on; its return value is ignored, so a chain and a block body both work.

| Method | What it styles |
| --- | --- |
| `hover(declare)` | While the pointer is over the element |
| `active(declare)` | While the element is pressed |
| `focus(declare)` | While the element has focus |
| `range_style(declare)` | The filled part of a `SliderIndicator` — how it looks, never where it is |
| `cell_style(declare)` | Every cell of an `OtpInput`; without it there is nothing on screen |
| `cell_active_style(declare)` | Layered on top, for the cell the next digit lands in |
| `caret_style(declare)` | The blinking mark in that cell while it is empty |

### Style methods

Everything else on an element is a style. There are two families, and they never overlap:

- **59 methods that take an argument**, bound by hand: the size, padding, margin, position, flex, border, radius and paint families. Which length type each accepts follows its Rust signature, so `.p("auto")` is a type error for the same reason it throws at run time.
- **3,143 no-argument methods**, generated from GPUI's reflection table with no maintenance at all: `flex_col`, `items_center`, `gap_2`, `rounded_md`, `text_sm`, `size_full`, `truncate` and the rest of the family. The figure moves when GPUI moves; `gpui-shell types` prints the one for your build.

Both are covered in [Styling](./styling.md), along with the length and color grammars and the tokens the palette defines.

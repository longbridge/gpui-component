---
title: Elements
description: Constructors, composition with child / children / when, and why an element description can only be used once.
order: 4
---

# Elements

An element in `gpui-shell` is a **description**, not an object. It exists for one `render` call and is consumed when it is used. This page covers what you can build, how to compose it, and what the runtime does when a description is used twice.

## Constructors

Each module carries what its own crate provides:

```js
import { div, text, svg, image } from "gpui";
import {
  h_flex,
  v_flex,
  Button,
  Link,
  Checkbox,
  Switch,
  Input,
  InputState,
} from "gpui-base";
import { fps_monitor } from "gpui-fps";
```

Functions are lowercase, and component types are capitalized and constructed through `.new`. That mirrors the Rust side one for one: `div()` is a free function there too, and `Button::new(id)` is an associated function on a type.

| Constructor | From | Produces |
| --- | --- | --- |
| `div()` | `gpui` | An element with no layout of its own |
| `text(value)` | `gpui` | A text element; the value is stringified |
| `svg(path)` | `gpui` | A theme-tinted vector icon from the application's own directory |
| `image(path)` | `gpui` | A full-colour image from the application's own directory |
| `h_flex()` | `gpui-base` | A row |
| `v_flex()` | `gpui-base` | A column |
| `Button.new(id)` | `gpui-base` | A base `Button`: activation, focus, disabled and selected state, no styling |
| `Link.new(id)` | `gpui-base` | A focusable external HTTP(S) link; set its target with `.href(url)` |
| `Checkbox.new(id)` | `gpui-base` | A base controlled checkbox, no styling and no indicator |
| `Switch.new(id)` | `gpui-base` | A base controlled switch, no styling |
| `Input.new(state)` | `gpui-base` | A text field backed by an [`InputState`](./state.md#retained-state) |
| `fps_monitor()` | `gpui-fps` | The native `gpui-fps` performance HUD, shared once per window |

### Performance monitor

`fps_monitor()` exposes the native `gpui-fps` HUD without moving its sampling or painting into JavaScript. The monitor is created on first use and reused per window. Render it at most once in a window, inside a `relative()` parent:

```js
div()
  .relative()
  .size_full()
  .child(content)
  .child(fps_monitor());
```

It is pinned to the top right by default. Use the existing anchor vocabulary to move it, for example `fps_monitor().anchor("bottom_left")`. The HUD owns its presentation; ordinary element styles, children, and interaction states do not apply to it.

### Why `.new(id)` and not `new Button(id)`

The JavaScript habit would be `new Button(id)`. The runtime does not offer it, and the reason is the whole subject of this page: `new` promises an object with an identity — something you can keep, store on the instance, and use again. That is exactly what a description is not. `Button.new(id)` reads as "construct a description", which is what it does, and it matches the Rust spelling character for character.

Views are the opposite case, and use the standard form: `class Counter extends View`. A view genuinely does have an identity and cross-frame state, and it is owned by GPUI. Two construction shapes in one file, because the two kinds of thing have different lifetimes.

### Ids

The `id` given to `Button`, `Link`, `Checkbox` and `Switch` identifies the element across renders, which is how GPUI preserves focus and element state. Keep it stable and unique among siblings — `` `item-${item.id}` `` rather than an array index that shifts when the list is filtered.

Anything else — a `div`, an `h_flex` — is identified by **where it sits in the tree your render built**. That is enough while the tree keeps its shape, and stops being enough the moment a conditional child appears above it: every element below shifts, and the pressed state, the focus and anything else keyed by identity shift with them.

`.id(name)` is how you say which element this is rather than where it landed:

```js
div()
  .id("toolbar")
  .active((el) => el.opacity(0.7))
```

Name anything whose identity has to survive its neighbours changing. `Button`, `Link`, `Checkbox` and `Switch` already have an identity from `new(id)` and ignore this one (with a warning, rather than silently).

### Text

`text(value)` stringifies whatever it is handed, so template literals and numbers work directly:

```js
text(`${this.remaining} of ${this.items.length} remaining`);
text(42);
```

A text element is materialized as a `div` containing the string, so it takes styles like any other element and can also take children.

### Images

```js
svg("icons/check.svg").w(14).h(14).flex_none();
image("images/brand.png").w(120).h(40);
```

Both paths resolve against the **application root** — the directory handed to `gpui-shell` — not against the file that called the constructor. That asymmetry surprises people, so it is worth stating plainly: `import "./ui.js"` resolves relative to the importing file, the way every JavaScript module system does, while `svg("icons/check.svg")` and `image("images/brand.png")` resolve relative to the application root, the way a web application's public directory does. The runtime cannot tell which module called an asset constructor, so per-file asset paths are not available to it.

Paths outside the application directory are rejected. A missing file is reported once per path with the location it was looked for, rather than silently drawing nothing.

One asset may contain at most 16 MiB. Listing the asset tree is bounded at 10,000 entries and 1 MiB of UTF-8 name bytes, so asset discovery cannot grow memory without limit.

Use `svg()` for a monochrome icon: it inherits the surrounding text colour, so an icon inside a dark button comes out light without the script saying so twice. Use `image()` when the source colours must be preserved, such as a logo, photo or illustration.

```js
renderIcon(cx) {
  return div()
    .bg(cx.theme().colors.foreground)
    .text_color(cx.theme().colors.surface)
    .child(svg("icons/check.svg").w(11).h(11));  // draws in `surface`
}
```

## Composition

| Method | Effect |
| --- | --- |
| `.child(element)` | Adds one child. The child is consumed |
| `.children(iterable)` | Adds several, in order |
| `.when(condition, branch)` | Applies `branch` only when `condition` is truthy |

```js
v_flex()
  .gap(8)
  .child(this.header())
  .children(this.visible().map((item) => this.row(item)))
  .when(this.items.length === 0, (el) => el.child(text("Nothing yet")));
```

`.when` exists so a conditional does not break the chain in two. `branch` **must return the element** — a branch that returns nothing throws immediately, rather than quietly dropping everything it built:

```text
when(...) must return the element
```

This mirrors GPUI's own `FluentBuilder` and the repository's Rust style rule: keep element construction as one fluent chain.

For a condition that chooses between two elements, an ordinary ternary is clearer than `when`:

```js
.child(
  visible.length === 0
    ? emptyState("No items yet", "Type above and press Add.")
    : v_flex().children(visible.map((item) => this.row(item))),
)
```

## Behavior methods

These are not styles; they report state to the base layer, which handles the interaction and leaves the appearance to you.

| Method | On | Effect |
| --- | --- | --- |
| `.on_click(handler)` | `Button` | `handler(event, cx)`, on click **and** on keyboard activation |
| `.on_change(handler)` | `Checkbox`, `Switch` | `handler(checked, cx)`; the script stores the value |
| `.disabled(value)` | `Button`, `Checkbox`, `Switch` | Blocks activation and reports the state |
| `.selected(value)` | `Button` | Reports the selected state |
| `.checked(value)` | `Checkbox`, `Switch` | The controlled value |
| `.accessibility_label(text)` | `Button`, `Checkbox` | What a screen reader announces |
| `.tooltip(text)` | `div`, `h_flex`, `v_flex`, `Button` | A label shown after the pointer rests on the element |
| `.id(name)` | `div`, `h_flex`, `v_flex` | A stable identity, instead of position in the tree |
| `.overflow_scrollbar()` | `div`, `h_flex`, `v_flex` | Scrolls both axes and paints native scrollbars |
| `.overflow_x_scrollbar()` | `div`, `h_flex`, `v_flex` | Scrolls horizontally and paints a native scrollbar |
| `.overflow_y_scrollbar()` | `div`, `h_flex`, `v_flex` | Scrolls vertically and paints a native scrollbar |

Disabled, selected and checked **appearance** is yours to draw. The base layer only reports the state; nothing changes on screen unless the script says so:

```js
Button.new("clear")
  .disabled(this.completed === 0)
  .when(this.completed === 0, (el) => el.opacity(0.4))
  .child(text("Clear completed"));
```

`.accessibility_label` matters most on an icon-only control, which announces nothing without it:

```js
Button.new(`remove-${item.id}`)
  .accessibility_label(`Remove “${item.caption}”`)
  .child(svg("icons/trash.svg").w(14).h(14));
```

### Controlled values report intent

A base checkbox does not change its own state. It reports what the user asked for, and the script decides:

```js
Checkbox.new(`item-${item.id}`)
  .checked(item.done)                       // the value comes from script state
  .on_change((done, cx) => {                // the callback is a request
    this.toggle(item.id, done, cx);
  })
  .child(indicator(item.done))
  .child(label(item.caption));
```

The runtime never quietly maintains a checked flag on the script's behalf. If it did, script authors and Rust authors would hold different mental models of the same control inside one application.

### Event objects

An `on_click` handler receives a plain object whose field names mirror the Rust struct:

```js
.on_click((event, cx) => {
  // event.click_count === 1
  // event.modifiers === { shift, control, alt, platform }
});
```

`platform` is Command on macOS and the Windows key elsewhere. Only semantics the base layer has already normalized are exposed — Base treats "Enter activates the button" and "the button was clicked" as the same callback, and the script does not see the difference.

::: tip Use arrow functions for handlers
An arrow function does not bind its own `this`, so `this` inside the handler is still the view instance. A `function () {}` handler gets the wrong `this`. This is the single most common mistake in scripts written for this runtime, by people and by models alike.
:::

## Focus and accessibility

A script owns its own focus targets. `cx.focus_handle()` creates one — `App::focus_handle` in GPUI, which has no `FocusHandle::new` for this to mirror — it lives on the view the way an [`InputState`](./state.md#retained-state) does, and `.track_focus(handle)` gives it to an element:

```js
init(props, cx) {
  this.search = cx.focus_handle();
}

render() {
  return Button.new("search")
    .tab_index(1)
    .track_focus(this.search)
    .child(text("Search"));
}
```

`cx.focus_handle()` needs a live host call, and a handle created inside `render` would be a new one on every frame — so the focus it tracked would be dropped by the next repaint. It belongs in `init` or in an event handler; calling it in `render` throws.

| On the handle | Answers |
| --- | --- |
| `handle.focus()` | Moves the keyboard onto the element tracking it |
| `handle.is_focused()` | Whether that element currently has the keyboard |
| `handle.release()` | Drops the handle |

`Tab` and `Shift-Tab` are handled by the window root, which walks the order below in both directions and honours the focus trap of an open dialog or sheet.

| Method | On | Effect |
| --- | --- | --- |
| `.track_focus(handle)` | `div`, `h_flex`, `v_flex`, `Button`, `Checkbox`, `Radio`, `Toggle` | Binds the element to a handle the script owns |
| `.tab_index(n)` | those, and `Link`, `Switch` | Where the element sits in the window's Tab order; it also makes the element a tab stop |
| `.tab_stop(value)` | the same set | Whether Tab can land on it at all. `false` keeps its place in the order without making it reachable |
| `.role(name)` | `div`, `h_flex`, `v_flex`, `Button`, `Checkbox` | What the element announces itself as |
| `.aria_selected(value)` | `div`, `h_flex`, `v_flex` | The selected state of an option in a list the script built |
| `.aria_active_descendant()` | `div`, `h_flex`, `v_flex` | Announces this element as the focused one while an ancestor holds the keyboard — the highlighted option of a combobox whose input keeps focus |

The sets differ because the components differ. `Button`, `Checkbox`, `Radio` and `Toggle` build their focus handle from a value you can replace; `Link` and `Switch` build their own and have no builder to replace it. Every component except `Button` and `Checkbox` announces a role of its own — a `Tab` is a tab, a `Radio` is a radio — and only those two treat the role as an override, which is what lets a button announce itself as a menu item. A call a component cannot honour is **reported in the log**, not silently dropped:

```text
`role` is not wired on a Tab: base's Tab owns this part of its own focus and
accessibility. Put it on an element around it
```

The plain elements take all six, which is how a script builds the listbox, toolbar or dialog the base layer has no component for:

```js
div()
  .id(`cadence-${index}`)
  .role("list_box_option")
  .aria_selected(index === this.chosen)
  .when(index === this.chosen, (el) => el.aria_active_descendant())
  .child(text(name))
```

Role names mirror `gpui::Role` in snake_case — `list_box`, `list_box_option`, `combo_box`, `menu_item` — and the whole set is in `gpui.d.ts` as the `Role` union, so an editor completes them and a name that is not one fails at the call site:

```text
unknown accessibility role `listbox`; the names mirror gpui::Role in snake_case
— see the Role type in gpui.d.ts
```

## Elements are single-use

This is the rule that most often surprises a new reader, so here is what it looks like and why it holds.

```js
const row = h_flex().child(text("hello"));

v_flex()
  .child(row)
  .child(row);   // throws
```

```text
element `h_flex` was already added to a parent; elements are single-use values
```

Storing one across frames fails the same way:

```js
init() {
  this.header = h_flex().child(text("Todo"));   // wrong
}

render() {
  return v_flex().child(text("Todo list")).child(this.header);
}
```

```text
this element belongs to a previous render pass; elements are single-use values
and must be rebuilt each time render runs
```

One rough edge worth knowing about: the arena is cleared and its indices reused on every pass, so a stale element occasionally holds the index the runtime has just handed to the node it is being attached to. The misuse is still caught, but the message reads `an element cannot be added to itself` instead. Both mean the same thing — the element belongs to a pass that has ended.

### Why

The restriction comes from GPUI itself: `RenderOnce::render` takes `self` **by value**, and `.child()` takes its child by value. In Rust the compiler enforces that with move semantics: using a moved value is a compile error. JavaScript has no move semantics and no compiler, so the runtime enforces the same rule at run time — and the description arena already has the bookkeeping needed to do it, because it marks a node as parented the moment it is attached.

The alternative would be to copy the description on reuse. That was rejected: it would make the same script mean different things in Rust and in JavaScript, and reuse is almost always a mistake rather than an intention.

### The shape that works

Build in `render`, and factor repetition into **functions that return a new element each time**:

```js
const label = (value, cx) => text(value).text_size(12).text_color(cx.theme().colors.foreground);

render(cx) {
  return v_flex()
    .child(label("first", cx))
    .child(label("second", cx));
}
```

That is how the [example application](https://github.com/longbridge/gpui-component/tree/main/examples/js_todolist) is written: `ui.js` exports `button`, `label`, `icon`, `checkbox` and the rest as functions, and `main.js` calls them. It reads like a component library and costs nothing, because a function call is where a fresh description comes from.

## Callbacks belong to their render

A handler passed to `.on_click` belongs to the description that render produced — not to a frame. That description is [replayed by every frame until something invalidates it](./state.md#when-render-runs), and the handler stays callable for all of them. The description records only an id; the closure Rust assembles holds a weak reference to the runtime plus that id.

The description a render replaced is kept one generation longer, because an event can be dispatched against a frame that has already been superseded. An event that arrives later than that is dropped with a `debug` log rather than an error — the author did nothing wrong, and there is nothing for them to fix.

The practical consequence is that a rendered callback is not a subscription. For something that must outlive the pass that created it — reacting to an input's `change` event, say — see [State and views](./state.md#input-events).

## Unknown methods are errors

A method that is neither a style nor one of the behavior methods above fails at the call site, with a suggestion when there is a close one:

```text
unknown element method `items_centre` (did you mean `items_center`?)
```

```text
unknown element method `on_clicked`; it is neither a style method nor one of
child, children, when, on_click, on_change, disabled, selected, checked, id
```

This matters more than it looks. A mistyped style name changes nothing on screen — it simply fails to — and without a diagnostic it is invisible. See [Styling](./styling.md#unknown-methods) for how the runtime produces that message without paying for it on every render.

## Not there yet

The element surface now includes Tabs, Table, Progress, form controls, anchored
Popover/HoverCard surfaces, Textarea, Scrollbar, PathBuilder, and VirtualList.
Still missing, deliberately:

- the higher-level List and Tree systems and the remaining `gpui-base` components;
- `gpui.memo`, which would let an unchanged subtree skip the script work that rebuilds its description;
- dock panels, and the renderer traits that would let a script draw the dock's own chrome.

Focus is now the script's to own, but not all of it. Still missing:

- **Keyboard navigation inside a composite.** Tab and Shift-Tab move between controls; the arrow keys that move within a listbox, a menu or a tab list do not exist, because the runtime has no action or key-binding layer for a script to reach.
- **The first Tab into an unfocused window.** While nothing at all holds focus, the root's Tab binding has no dispatch path to reach; focus has to arrive some other way first — a click, or `handle.focus()`.
- **`Tab`, `Tabs`, and the table, group and progress parts** stay out of the Tab order. Base leaves them out of keyboard focus, and `tab_index` on one of them is reported rather than honoured.
- **`track_focus` on `Link` and `Switch`**, for the same reason: they build their own handle and expose no builder to replace it.

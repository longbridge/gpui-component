---
title: Elements
description: Constructors, composition with child / children / when, and why an element description can only be used once.
order: 3
---

# Elements

An element in `gpui-shell` is a **description**, not an object. It exists for one render pass and is consumed when it is used. This page covers what you can build, how to compose it, and what the runtime does when a description is used twice.

## Constructors

One import provides the whole namespace:

```js
import { div, h_flex, v_flex, text, svg, Button, Checkbox, Switch, Input, InputState } from "gpui";
```

Functions are lowercase, and component types are capitalized and constructed through `.new`. That mirrors the Rust side one for one: `div()` is a free function there too, and `Button::new(id)` is an associated function on a type.

| Constructor | Produces |
| --- | --- |
| `div()` | An element with no layout of its own |
| `h_flex()` | A row |
| `v_flex()` | A column |
| `text(value)` | A text element; the value is stringified |
| `svg(path)` | An image from the application's own directory |
| `Button.new(id)` | A base `Button`: activation, focus, disabled and selected state, no styling |
| `Checkbox.new(id)` | A base controlled checkbox, no styling and no indicator |
| `Switch.new(id)` | A base controlled switch, no styling |
| `Input.new(state)` | A text field backed by an [`InputState`](./state.md#retained-state) |

### Why `.new(id)` and not `new Button(id)`

The JavaScript habit would be `new Button(id)`. The runtime does not offer it, and the reason is the whole subject of this page: `new` promises an object with an identity — something you can keep, store on the instance, and use again. That is exactly what a description is not. `Button.new(id)` reads as "construct a description", which is what it does, and it matches the Rust spelling character for character.

Views are the opposite case, and use the standard form: `class Counter extends View`. A view genuinely does have an identity and cross-frame state, and it is owned by GPUI. Two construction shapes in one file, because the two kinds of thing have different lifetimes.

### Ids

The `id` given to `Button`, `Checkbox` and `Switch` identifies the element across renders, which is how GPUI preserves focus and element state. Keep it stable and unique among siblings — `` `item-${item.id}` `` rather than an array index that shifts when the list is filtered.

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
```

An `svg` path resolves against the **application root** — the directory handed to `gpui-shell` — not against the file that called it. That asymmetry surprises people, so it is worth stating plainly: `import "./ui.js"` resolves relative to the importing file, the way every JavaScript module system does, while `svg("icons/check.svg")` resolves relative to the application root, the way a web application's public directory does. The runtime cannot tell which module called `svg`, so per-file asset paths are not available to it.

Paths outside the application directory are rejected. A missing file is reported once per path with the location it was looked for, rather than silently drawing nothing.

An icon inherits the surrounding text colour, so an icon inside a dark button comes out light without the script saying so twice:

```js
div()
  .bg("foreground")
  .text_color("surface")
  .child(svg("icons/check.svg").w(11).h(11));  // draws in `surface`
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

It is not a restriction the runtime invented. GPUI's `RenderOnce::render` takes `self` **by value**, and `.child()` takes its child by value. In Rust the compiler enforces that with move semantics: using a moved value is a compile error. JavaScript has no move semantics and no compiler, so the runtime enforces the same rule at run time — and the description arena already has the bookkeeping needed to do it, because it marks a node as parented the moment it is attached.

The alternative would be to copy the description on reuse. That was rejected: it would make the same script mean different things in Rust and in JavaScript, and reuse is almost always a mistake rather than an intention.

### The shape that works

Build in `render`, and factor repetition into **functions that return a new element each time**:

```js
const label = (value) => text(value).text_size(12).text_color("foreground");

render() {
  return v_flex()
    .child(label("first"))
    .child(label("second"));
}
```

That is how the [example application](https://github.com/longbridge/gpui-component/tree/main/examples/js_todolist) is written: `ui.js` exports `button`, `label`, `icon`, `checkbox` and the rest as functions, and `main.js` calls them. It reads like a component library and costs nothing, because a function call is where a fresh description comes from.

## Callbacks belong to their render

A handler passed to `.on_click` is stored in an arena that is replaced by the next render. The description records only an id; the closure Rust assembles holds a weak reference to the runtime plus that id.

The previous generation is kept one frame longer, because an event can be dispatched between render and paint. An event that arrives more than two generations late is dropped with a `debug` log rather than an error — the author did nothing wrong, and there is nothing for them to fix.

The practical consequence is that a rendered callback is not a subscription. For something that must outlive the pass that created it — reacting to an input's `change` event, say — see [State and views](./state.md#input-events).

## Unknown methods are errors

A method that is neither a style nor one of the behavior methods above fails at the call site, with a suggestion when there is a close one:

```text
unknown element method `items_centre` (did you mean `items_center`?)
```

```text
unknown element method `on_clicked`; it is neither a style method nor one of
child, children, when, on_click, on_change, disabled, selected, checked
```

This matters more than it looks. A mistyped style name changes nothing on screen — it simply fails to — and without a diagnostic it is invisible. See [Styling](./styling.md#unknown-methods) for how the runtime produces that message without paying for it on every frame.

## Not there yet

The element surface is the M0 set. Missing, deliberately, and each belonging to a later milestone:

- Select, tabs, list, table, tree and the other `gpui-base` components;
- `gpui.memo`, which would let an unchanged subtree skip the script work that rebuilds its description;
- dock panels, and the renderer traits that would let a script draw the dock's own chrome;
- an `img()` constructor — `svg()` is the only image element today.

# gpui-shell

`gpui-shell` is a scriptable application runtime for [GPUI](https://gpui.rs),
built on [`gpui-base`](../base/README.md). The host owns rendering, layout,
input and system capabilities; the script owns composition, presentation and
business logic. JavaScript is the default scripting language.

Its design is specified in [`docs/research/gpui-shell.md`](../../docs/research/gpui-shell.md).
This crate is at milestone M0: a feasibility baseline, not a stable interface.

## Base-First: The Script Owns Presentation

`gpui-base` controls carry no visual style. `Button::new("save")` has no
padding, background, radius or size, and that is an API contract rather than a
missing feature. The JavaScript bindings preserve it: `Button.new("save")` with
no styling draws nothing but its children.

```js
// Unstyled: activation, focus and disabled state work, but nothing is drawn
// around the label.
Button.new("save").on_click(save).child(text("Save"));

// Styled: every visual decision is written out, in the script.
Button.new("save")
  .h(32)
  .px(14)
  .items_center()
  .justify_center()
  .text_sm()
  .bg("primary")
  .text_color("primary_foreground")
  .rounded(6)
  .on_click(save)
  .child(text("Save"));
```

This is the same trade the Rust side makes when an application builds directly
on `gpui-base` instead of `gpui-component`. Colors are named as semantic theme
tokens, so a shared visual language stays available without the runtime making
visual decisions on the application's behalf. Applications that want ready-made
product visuals wait for a `gpui-component` module, a later milestone.

## Quick Start

Run the bundled example from the repository root:

```bash
cargo run -p gpui-shell -- examples/js_todolist
```

The runtime loads `main.js` from the given directory, takes the view class it
default-exports, and mounts one instance of it as the window's root view:

```js
// examples/js_todolist/main.js
import Counter from "./counter.js";

export default Counter;
```

```js
// counter.js, trimmed down; the example has the full version
import { View, v_flex, text, Button } from "gpui";

export default class Counter extends View {
  init(props = {}) {
    this.count = props.start ?? 0;
  }

  render() {
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(20)
      .bg("background")
      .child(text(`${this.count}`).text_3xl().text_color("foreground"))
      .child(
        Button.new("increment")
          .h(32)
          .px(14)
          .items_center()
          .justify_center()
          .bg("primary")
          .text_color("primary_foreground")
          .rounded(6)
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(text("Increment")),
      );
  }
}
```

See [`examples/js_todolist`](../../examples/js_todolist) for the complete
version, including a button helper that applies consistent styling from theme
tokens.

## Naming

Method names on the bindings keep their Rust snake_case spelling —
`items_center`, `on_click`, `gap_2`, `text_sm`. They are not a style choice:
the no-argument style surface is generated from GPUI's reflection table, so the
name in JavaScript is the name in Rust, and a method GPUI adds upstream appears
here without anyone renaming it. Everything an application declares itself —
its own variables, functions, classes and object keys — is ordinary camelCase
JavaScript. The contrast is the point: a snake_case call is host surface, a
camelCase one is script code.

## M0 API Surface

One import provides the whole namespace:

```js
import { View, div, h_flex, v_flex, text, Button, Checkbox, Switch } from "gpui";
```

| API | Form | Description |
| --- | --- | --- |
| `div()` | function | An element with no layout of its own |
| `h_flex()` / `v_flex()` | function | A row / column flex element |
| `text(value)` | function | A text element |
| `Button.new(id)` | type | A base `Button`: activation, focus, disabled and selected state, no styling |
| `Checkbox.new(id)` / `Switch.new(id)` | type | A base controlled toggle, no styling |
| `View` | class | Base class of every view; subclass it and default-export the subclass |

Functions are lowercase and types are capitalized and constructed through
`.new`, mirroring the Rust side one for one.

### Elements

| Method | Description |
| --- | --- |
| `.child(element)` | Adds one child. The child is consumed; using it again is an error |
| `.children([a, b])` | Adds several children |
| `.when(condition, el => el)` | Applies the function only when `condition` holds, keeping the chain in one piece |

### Styling

Every element accepts the no-argument GPUI style surface as methods
(`.size_full()`, `.items_center()`, `.justify_center()`, `.flex_col()`,
`.rounded_md()`, `.text_sm()`, `.font_semibold()`, and the rest), plus about
fifty-seven methods that take arguments:

| Method | Argument |
| --- | --- |
| `.w(n)` `.h(n)` `.size(n)`, and the `min_` / `max_` forms | length |
| `.p(n)` `.px(n)` `.py(n)` `.pt(n)` `.pb(n)` `.pl(n)` `.pr(n)` | length |
| `.m(n)` `.mx(n)` `.my(n)` `.mt(n)` `.mb(n)` `.ml(n)` `.mr(n)` | length |
| `.inset(n)` `.top(n)` `.bottom(n)` `.left(n)` `.right(n)` | length |
| `.gap(n)` `.gap_x(n)` `.gap_y(n)` `.flex_basis(n)` | length |
| `.flex_grow(n)` `.flex_shrink(n)` `.opacity(n)` | number |
| `.border(n)` and its per-edge forms, `.rounded(n)` and its per-corner forms | length |
| `.bg(color)` `.text_color(color)` `.text_bg(color)` `.border_color(color)` | color |
| `.text_size(n)` `.line_height(n)` | length |

A number is pixels. A string length is `"auto"`, `"50%"`, `"12px"` or `"1rem"`;
which of those a given method accepts follows the Rust signature, so `.p()`
rejects `"auto"` and `.rounded()` rejects percentages. A color is either a
semantic token name — `background`, `foreground`, `surface`,
`surface_foreground`, `primary`, `primary_foreground`, `secondary`,
`secondary_foreground`, `muted`, `muted_foreground`, `accent`,
`accent_foreground`, `destructive`, `destructive_foreground`, `border`, `input`,
`ring` — or a `#rrggbb` literal. Token names are preferred; a literal bypasses
the theme.

A style name that is neither reflected nor bound is an error at the call site,
not a silently ignored no-op.

### Components

| Method | On | Description |
| --- | --- | --- |
| `.disabled(bool)` | all | Blocks activation and reports the disabled state |
| `.selected(bool)` | `Button` | Reports the selected state |
| `.on_click(handler)` | `Button` | `handler(event, cx)`, called on click and on keyboard activation |
| `.checked(bool)` | `Checkbox`, `Switch` | The controlled value |
| `.on_change(handler)` | `Checkbox`, `Switch` | `handler(checked, cx)`; the script stores the new value and notifies |

Disabled, selected and checked appearance is the caller's to draw; the base
layer only reports the state.

### Views

```js
export default class Counter extends View {
  init(props) {}   // called once, when the view is created
  render(cx) {}    // returns exactly one element
}
```

`cx.notify()` requests a re-render. It is legal only inside an event callback;
calling it during `render` throws, because notifying yourself while rendering is
a loop.

Elements are single-use values. Build them in `render` and never store one on
the instance — a stored element belongs to a render pass that has already ended,
and reusing it throws rather than drawing something unexpected.

## The Engine Seam

The scripting engine sits behind one internal interface,
[`src/engine/mod.rs`](src/engine/mod.rs). Everything above it — the spec arena,
the materializer, the call scope, the style table, the theme, the capability
model — is engine independent, and only the engine module knows what a script
value is.

QuickJS is the default, via `rquickjs`. The same runtime also builds on Lua:

```bash
cargo run -p gpui-shell --no-default-features --features lua -- path/to/app
```

Exactly one engine may be enabled; enabling both is a compile error, because
`gpui_shell::ShellRuntime` would be ambiguous.

The seam exists because the engine choice is the one decision in this runtime
that cannot be validated on paper. Per-call cost across the language boundary
decides whether the whole approach is viable (design document §20), and that
number has to be measured on both engines rather than argued about. JavaScript
is the default because application code reads better in it and the language is
more widely known; keeping the Lua engine behind a feature flag means the
measurement can be repeated and a reversal stays a feature change rather than a
rewrite.

The scripts themselves are not portable between the two — they are different
languages — but the binding surface, the render protocol and the semantics
described above are the same on either engine.

## Not Here Yet

M0 covers `div`, `text`, `Button`, `Checkbox`, `Switch`, `on_click`,
`on_change`, `when`, `cx.notify()`, the style surface and the default token
palette. Deliberately absent:

- `gpui.open_window` and multi-window applications; the host opens the window.
- Select, tabs, list, table and tree bindings, and `InputState` and the other
  host state entities.
- Semantic state styles (`style_disabled`), `hover` / `active` modifiers, and
  animation.
- Promises, `gpui.spawn`, timers, and every asynchronous API.
- System capabilities: `fs`, `http`, `store`, `clipboard`, `log`.
- Dock panels, the plugin model, the sandbox, and hot reload.

Each of these belongs to a later milestone. The full roadmap, with the exit
criteria that gate every stage, is section 26 of
[`docs/research/gpui-shell.md`](../../docs/research/gpui-shell.md).

## How It Works

GPUI elements are values that are consumed when used: `RenderOnce::render`
takes `self` by value, `child` takes its child by value, and a view rebuilds its
entire element tree on every redraw. A JavaScript object can therefore never
*be* an element. Instead, a script builder records its calls into an arena of
element descriptions; when GPUI asks the view to render, Rust replays those
recorded calls into real elements, hands them to GPUI, and clears the arena.
Nothing survives the pass — not an element, not a callback, not the `cx` handed
to `render`. This is why elements are single-use, why callbacks belong to the
render that registered them, and why the runtime can check both at runtime and
throw a script error instead of failing in undefined ways.

## Related Resources

- [GPUI Shell design document](../../docs/research/gpui-shell.md)
- [`gpui-base`](../base/README.md), the foundation this runtime binds
- [Architecture](../../docs/ARCHITECTURE.md) and [Styling and Motion](../../docs/STYLING-AND-MOTION.md)
- [GPUI](https://gpui.rs)

## License

Apache-2.0. See [`../../LICENSE-APACHE`](../../LICENSE-APACHE).

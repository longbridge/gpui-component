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
// main.js
import { View, v_flex, text, Button, InputState } from "gpui";

export default class Notes extends View {
  init() {
    this.draft = InputState.new({ placeholder: "What needs doing?" });
    this.draft.on("submit", (_event, cx) => this.add(cx));
    this.items = [];
  }

  add(cx) {
    const caption = this.draft.value().trim();
    if (caption === "") return;
    this.items = [...this.items, caption];
    this.draft.set_value("");
    cx.notify();
  }

  render() {
    return v_flex()
      .size_full()
      .p(24)
      .gap(12)
      .bg("background")
      .children(this.items.map((item) => text(item).text_color("foreground")));
  }
}
```

See [`examples/js_todolist`](../../examples/js_todolist) for the complete
version: retained input state, controlled checkboxes, a confirmation dialog, a
toast, icons, and storage that degrades to memory when it is not granted.

### Checking an application without running it

JavaScript has no compiler, so the runtime provides what would otherwise be
missing:

```bash
cargo run -p gpui-shell -- check examples/js_todolist    # exit 0 or 1
cargo run -p gpui-shell -- check examples/js_todolist --print-spec
cargo run -p gpui-shell -- types examples/js_todolist    # writes gpui.d.ts
```

`check` loads and renders the application once without showing a window. It
reports syntax errors, unresolved imports, a missing or malformed default
export, unknown style methods with a suggestion, wrongly typed style arguments,
and an element used twice — each with the script's own stack. `types` writes
TypeScript declarations generated from the same tables the runtime dispatches
through, so an editor catches a mistyped style method before it runs.

### Working on an application

```bash
cargo run -p gpui-shell -- examples/js_todolist --watch
cargo run -p gpui-shell -- examples/js_todolist --dev    # implies --watch
```

A reload re-reads every module, entry included. If the new code fails to load,
the previous view keeps running and the error is reported — a broken save never
costs you the window.

## Naming

Method names on the bindings keep their Rust snake_case spelling —
`items_center`, `on_click`, `gap_2`, `text_sm`. They are not a style choice:
the no-argument style surface is generated from GPUI's reflection table, so the
name in JavaScript is the name in Rust, and a method GPUI adds upstream appears
here without anyone renaming it. Everything an application declares itself —
its own variables, functions, classes and object keys — is ordinary camelCase
JavaScript. The contrast is the point: a snake_case call is host surface, a
camelCase one is script code.

## API Surface

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

Present today: the element and style surface, state styles (`hover` / `active` /
`focus`), `Button`, `Checkbox`, `Switch`, retained `InputState` with input
events, icons through `svg()`, dialogs, sheets and toasts on `cx`, promises and
timers, `fs` / `store` / `clipboard` / `log` / `process` behind capabilities,
hot reload, `check`, and generated TypeScript declarations.

Deliberately absent:

- `gpui.open_window` and multi-window applications; the host opens the window.
- Select, combobox, tabs, list, table and tree bindings.
- Charts, the code editor and its LSP surface, and WebView — these stay in Rust
  on purpose; binding a trait-and-generics interface across a language boundary
  costs more than it returns.
- Asynchronous `fs`: the filesystem calls are synchronous, which is wrong for a
  large file on the render thread. They are shaped so the move onto the
  scheduler is mechanical.
- Packaging and installing an application as a distributable archive.

The design, what is implemented, and what is not are in
[`docs/gpui-shell.md`](../../docs/gpui-shell.md).

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

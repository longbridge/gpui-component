# gpui-shell

`gpui-shell` is a scriptable application runtime for [GPUI](https://gpui.rs),
built on [`gpui-base`](../base/README.md). The host owns rendering, layout,
input and system capabilities; the script owns composition, presentation and
business logic. JavaScript is the default scripting language.

Its design is specified in [`docs/gpui-shell.md`](../../docs/gpui-shell.md).
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
import { View, div, h_flex, v_flex, text, Button, Link, Checkbox, Switch } from "gpui";
```

| API | Form | Description |
| --- | --- | --- |
| `div()` | function | An element with no layout of its own |
| `h_flex()` / `v_flex()` | function | A row / column flex element |
| `text(value)` | function | A text element |
| `Button.new(id)` | type | A base `Button`: activation, focus, disabled and selected state, no styling |
| `Link.new(id)` | type | A base external link; pair it with `.href("https://…")` |
| `Checkbox.new(id)` / `Switch.new(id)` | type | A base controlled toggle, no styling |
| `InputState.new(options)` / `Input.new(state)` | types | Retained text state and its rendered input |
| `View` | class | Base class of every view; subclass it and default-export the subclass |

Functions are lowercase and types are capitalized and constructed through
`.new`, mirroring the Rust side one for one.

### Elements

| Method | Description |
| --- | --- |
| `.child(element)` | Adds one child. The child is consumed; using it again is an error |
| `.children([a, b])` | Adds several children |
| `.when(condition, el => el)` | Applies the function only when `condition` holds, keeping the chain in one piece |
| `.href(url)` | Gives a `Link` an absolute HTTP(S) target opened by the host |
| `.transition(property, policy)` | Animates a later target change in native Rust code |
| `.spring(property, policy?)` | Springs a later target change in native Rust code |

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

Read semantic values once per render with `const tokens = cx.theme()`. The
returned light/dark snapshot contains direct color roles plus `colors`,
`spacing`, `radius`, `mode`, and `is_dark`; it and all nested token groups are
frozen. `gpui.theme()` remains a compatibility accessor. Calling
`set_theme("light" | "dark")` selects one of the shell palettes and refreshes
the windows.

Motion is target-based, not a JavaScript frame callback. `transition` and
`spring` accept `opacity`, `width`, `height`, `left`, or `top`; length targets
are currently pixels. JavaScript publishes the new target once, while retained
state, sampling, interruption, reduced motion, and frame requests stay native.

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

**`render` does not run every frame.** It runs when the view has been
invalidated — a `notify`, a hot reload, a theme change — and publishes a
description that every frame after that replays in Rust. A hover, a scroll, a
blinking cursor or an animation repaints without entering the VM at all, so
script cost follows what your application does rather than the frame rate.

Elements are single-use values. Build them in `render` and never store one on
the instance — a stored element belongs to a render that has already ended, and
reusing it throws rather than drawing something unexpected.

## Capabilities and Asynchronous I/O

System access is denied by default. A local application can declare the exact
grant the CLI installs in `gpui-shell.json`:

```json
{
  "id": "com.example.viewer",
  "name": "Viewer",
  "version": "1.0.0",
  "entry": "main.js",
  "capabilities": {
    "fs": { "read": ["${pluginDir}"], "write": ["${dataDir}"] },
    "network": {
      "hosts": ["stream.example.com"],
      "http": [{
        "host": "api.example.com",
        "methods": ["GET"],
        "paths": ["/v1/profile"],
        "path_prefixes": ["/v1/items/"]
      }]
    },
    "store": true,
    "clipboard": { "write": true }
  }
}
```

`network.hosts` grants the host to HTTP, raw TCP, and WebSocket clients;
`network.http` narrows HTTP to listed methods and paths without granting TCP or
WebSocket access. `fetch` supports GET/POST, safe headers, string or
`Uint8Array` bodies, a 30-second request timeout, and 8 MiB request/response
limits. Every redirect target must be granted; HTTPS downgrade is refused, as
are cross-origin POST replays and cross-origin redirects carrying Authorization
or any caller-supplied header.

`WebSocket.connect(url, { headers })` resolves after the handshake and returns
async `read`, `write`, and `close` methods for text and binary messages. Frames
and messages are limited to 8 MiB. Connect/handshake and writes have 30-second
transport deadlines. A pending `read()` has no public timeout and waits for a
message, close, or error; only one read may be outstanding, while writes and
close are still serviced as it waits. Credential and handshake-control headers
are refused.

Both `fs` and `fs/promises` expose the same promise-only subset: `readFile`,
`writeFile`, `readdir`, `exists`, `unlink`, `rmdir`, and `mkdir`. Capability
checks happen at the call site, then filesystem work runs off the UI/VM thread.
There are no synchronous filesystem calls.

## The Engine Seam

The scripting engine sits behind one internal interface,
[`src/engine/mod.rs`](src/engine/mod.rs). Everything above it — the spec arena,
the materializer, the call scope, the style table, the theme, the capability
model — is engine independent, and only the engine module knows what a script
value is.

QuickJS is what ships, via `rquickjs`, and is the only engine today. JavaScript
is the choice because application code reads better in it and the language is
more widely known.

**Call it dependency isolation, not a replaceable-engine contract.**
`ShellRuntime` and the two handle types are re-exports of QuickJS types, not
associated types behind a trait, so a second engine would be a port rather than
an implementation of something already written down.

What the isolation buys is still worth having: no module above `engine/` names a
script value, host configuration cannot be silently dropped by an engine that
does not implement it, and `build_snapshot` is the single enforcement point for
the rule that a repaint never enters the VM. Turning it into an actual contract —
an internal trait with opaque handles, and a fake engine to compile it against —
is worth doing when there is a second engine to write, and is make-work before
that.

## Not Here Yet

Present today: the element and style surface, state styles (`hover` / `active` /
`focus`), `Button`, `Checkbox`, `Switch`, retained `InputState` with input
events, icons through `svg()`, dialogs, sheets and toasts on `cx`, promises and
timers, `fs` / `store` / `clipboard` / `log` / `process` behind capabilities,
capability-gated HTTP and text/binary WebSocket clients, native target-value
transitions and springs, hot reload, `check`, and generated TypeScript
declarations.

Deliberately absent:

- `gpui.open_window` and multi-window applications; the host opens the window.
- Select, combobox, tabs, list, table and tree bindings.
- Charts, the code editor and its LSP surface, and WebView — these stay in Rust
  on purpose; binding a trait-and-generics interface across a language boundary
  costs more than it returns.
- Packaging and installing an application as a distributable archive.

The design, what is implemented, and what is not are in
[`docs/gpui-shell.md`](../../docs/gpui-shell.md).

## Types for the Script

`import ... from "gpui"` is opaque without declarations, and the style surface
is far too large to memorize. **There is nothing to run.** Every `gpui-shell`
invocation — running an application, `check`, `types` — writes `gpui.d.ts` into
each directory that imports the module, from the runtime it is about to use:

```bash
cargo run -p gpui-shell -- path/to/app           # runs it, and writes them
cargo run -p gpui-shell -- check path/to/app     # checks it, and writes them
cargo run -p gpui-shell -- types path/to/app     # writes them and nothing else
```

Add `gpui.d.ts` to `.gitignore`; the file's own first line says so.

The style methods, their argument types and the colour-token union are generated
from the tables the runtime dispatches through, so a name that type-checks is a
name the dispatcher accepts.

Host-registered native modules cannot be generated — only the host knows what it
granted — so an application declares its own and gets a checked module name with
completing functions:

```ts
declare module "gpui" {
  interface NativeModules {
    market: { quotes(): Quote[]; watch(symbol: string): boolean };
  }
}
```

`crates/story/js/quotes/` has both files plus a `jsconfig.json` that turns on
checking, and is the shape to copy.

### Keeping it current

`gpui.d.ts` is an **output**, not a source, and a stale one is worse than none:
it completes methods that no longer exist and refuses ones that do, and nothing
about editing against it feels wrong until the script runs. So it is not
something to write down and remember — it is rewritten by whatever is about to
run the script.

| Situation | What keeps it current |
| --- | --- |
| The `gpui-shell` binary | Every run, `check` and `types` refreshes every directory that imports the module. Nothing to remember. |
| An application embedded in a host | The host calls `gpui_shell::typings::refresh_tree(&app_root)` where it loads the application. Same guarantee, one line. |

Nothing is written when the file already matches, so an editor watching the
directory is not woken on every launch and a read-only checkout is not an error.
A directory that refuses the write is logged, never fatal.

Do not commit it. This repository ignores `gpui.d.ts` everywhere, including
beside its own example and story scripts — a committed copy could only ever be
the stale one. What *is* committed is the hand-written part: a `jsconfig.json`
that turns checking on, and the application's own native-module declarations.

The header names the script API version it was generated for, so a mismatch is
visible on the first line rather than at the first call.

## Embedding It

Three host-side calls carry most of the weight:

```rust,ignore
// Editing a script changes the window, with no rebuild and no button — in a
// debug build. Compiled out of a release build, which must not poll a
// directory nobody is editing.
let watch =
    gpui_shell::watch::reload_in_debug(runtime, &view, app_root, "main.js", window, cx);
// Keep the handle for as long as the view is mounted; dropping it stops the
// watcher, so an unmounted panel does not leave one polling for it.

// The script changed something on screen? GPUI already knows. But when *Rust*
// changes state the script reads, say so — a bare notify is only a repaint.
script_view.update(cx, |view, cx| view.refresh(cx));

// What it is costing: script renders against frames, with the time each took.
let reading = runtime.metrics().read();

// Native module closures capture host entity handles, so a host that goes away
// clears them. GPUI's leak check catches a host that forgets.
gpui_shell::clear_native_modules();
```

## How It Works

GPUI elements are values that are consumed when used: `RenderOnce::render`
takes `self` by value, `child` takes its child by value, and a view rebuilds its
entire element tree on every redraw. A JavaScript object can therefore never
*be* an element. Instead, a script builder records its calls into an arena of
element descriptions, and Rust replays those recorded calls into real elements.

The description, though, is **not** rebuilt every redraw. It is published as a
snapshot when the script says its state moved, and every frame after that
replays the same snapshot in Rust — so a hover, a scroll, a blinking cursor or
an animation repaints without entering the VM. Script cost follows what the
application does rather than the frame rate. Elements are still single-use and
callbacks still belong to the render that registered them; what changed is how
long that render's output lives.

## Related Resources

- [GPUI Shell design document](../../docs/gpui-shell.md)
- [`gpui-base`](../base/README.md), the foundation this runtime binds
- [Architecture](../../docs/ARCHITECTURE.md) and [Styling and Motion](../../docs/STYLING-AND-MOTION.md)
- [GPUI](https://gpui.rs)

## License

Apache-2.0. See [`../../LICENSE-APACHE`](../../LICENSE-APACHE).

---
title: GPUI Shell
description: A scriptable application runtime for GPUI — application code in JavaScript, rendering and system capabilities in Rust.
order: 1
---

# GPUI Shell

`gpui-shell` is a scriptable application runtime for [GPUI](https://gpui.rs), built directly on [`gpui-base`](/base/). The application is written in **JavaScript**, running on QuickJS inside the host process. Rust keeps rendering, layout, text editing, virtualization, focus, overlays and every system capability. The script owns composition, presentation and business logic.

```js
import { View, v_flex, text, Button } from "gpui";

export default class Counter extends View {
  init() {
    this.count = 0;
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

## The one thing that makes it different

Most scripting layers hand a script a set of finished widgets and let it arrange them. This one does not, because the layer underneath it has no finished widgets to hand over.

`gpui-base` controls carry no visual style at all. `Button::new("save")` in Rust has no padding, no background, no radius and no size, and that is a contract rather than an omission. The JavaScript bindings preserve it exactly: `Button.new("save")` with no styling draws nothing but its children.

The consequence is the point. **Because the foundation ships no presentation, the script owns all of it** — every colour, every pixel of spacing, every hover state, every corner radius. That is the same trade a Rust application makes when it builds on `gpui-base` instead of `gpui-component`; the difference is that here the trade is made in a file you can save and see the result of immediately, with no `cargo build` in between.

What the script gains in exchange for the extra typing is the whole application layer. Changing a button's radius does not mean going back to Rust.

## How a script becomes an interface

<img src="/shell-architecture.svg" alt="How a script becomes an interface: the script describes elements, Rust materializes them, GPUI paints" class="shell-architecture" />

The diagram traces one frame, and the shape of it explains most of this documentation.

GPUI elements are values that are **consumed** when used: `RenderOnce::render` takes `self` by value, `.child()` takes its child by value, and a view rebuilds its whole element tree on every redraw. A JavaScript object can therefore never *be* a GPUI element — there is nothing for it to hold onto.

So the script does not build elements. It **describes** them. Every call in a builder chain records one operation into an arena of element descriptions; the object the script holds carries nothing but an integer index into that arena. When GPUI asks the view to render, Rust replays the recorded operations into real elements, hands them to GPUI, and clears the arena. Layout, painting, hit testing, scrolling and IME never return to the script.

Three consequences follow directly, and each has a page below:

- **Elements are single-use.** The description is gone at the end of the pass, so a stored element throws on its next use rather than drawing something unexpected. See [Elements](./elements.md).
- **The `cx` handed to a call belongs to that call.** It carries a generation number, checked against the live call stack, so a `cx` kept across an `await` reports a clear error instead of touching a dead stack frame. See [State and views](./state.md).
- **Callbacks belong to the render that registered them.** They are replaced wholesale by the next render, which is what keeps script closures from accumulating in the host. See [Elements](./elements.md).

None of that is a design flourish. It is what falls out of binding a script to an element model that consumes its values.

## Who it is for

| You are | What the runtime gives you |
| --- | --- |
| Adding panels, commands or tools to an existing Rust GPUI application | A sandboxed script surface with capability grants the host decides, so an extension does not mean a fork |
| Writing an internal tool — a dashboard, an ops panel, a data viewer | A low starting cost, a real desktop window, and a save-and-see loop instead of a compile |
| Generating interfaces with a model | The most widely covered language there is, errors that are recoverable rather than fatal, and a generated `gpui.d.ts` that describes the whole API |

It is deliberately **not** a way to rewrite a product's core in JavaScript. Text editing, syntax highlighting, LSP, virtualization and animation stay in Rust, where they belong.

## Where it sits

```text
  JavaScript application       main.js · views · styles · business logic
            │  import { … } from "gpui"
            ▼
  gpui-shell                   engine seam · element descriptions · call scope
                               style table · theme tokens · capabilities
                               ShellRoot (dialogs, sheet, toasts) · scheduler
            │
            ▼
  gpui-base                    behavior · state · infrastructure (no style)
            │
            ▼
  gpui / gpui_platform         elements · styling · rendering · GPU · platform
```

`gpui-shell` sits beside `gpui-component` rather than beneath it: both are consumers of `gpui-base`, and both supply a presentation layer that Base does not. `gpui-component` supplies one in Rust, finished and coherent. `gpui-shell` supplies the machinery for a script to supply its own.

## Read next

| Page | What it covers |
| --- | --- |
| [Getting started](./getting-started.md) | Running the example, the smallest application, `check` and `types` |
| [Elements](./elements.md) | Constructors, `child` / `children` / `when`, and why an element is single-use |
| [Styling](./styling.md) | The fluent style surface, lengths, colour tokens and state styles |
| [State and views](./state.md) | `init` / `render`, `cx.notify()`, retained state, async |
| [Overlays](./overlays.md) | Dialogs, the sheet, toasts, and the phase rule |
| [Capabilities](./capabilities.md) | The default-deny model, `fs` / `store` / `clipboard` / `log` / `process` |
| [The engine seam](./engine.md) | QuickJS, the LuaJIT fallback, and the measurement that decides between them |

## Status

The crate is at milestone **M0**: a feasibility baseline, not a stable interface. It is not published to crates.io, and the script API is expected to change. What is documented here exists and works; what is missing is called out on the page where you would go looking for it.

The design is specified in the [GPUI Shell design document](https://github.com/longbridge/gpui-component/blob/main/docs/gpui-shell.md), and the crate lives at [`crates/shell`](https://github.com/longbridge/gpui-component/tree/main/crates/shell).

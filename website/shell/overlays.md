---
title: Overlays
description: Dialogs, the sheet and toasts on cx, their stacking and dismissal order, and why they may only be opened from an event.
order: 6
---

# Overlays

Dialogs, the sheet and toasts are **host** capabilities, reached through `cx`. They are not something a script draws.

A dialog is not a floating `div`. It is a place in the window's stacking order, a focus trap, an Escape target, and a promise about what pressing the backdrop means — all of which the window's root has to decide, because only something that sees every overlay at once can order them. A script drawing its own dialog would own none of that, and two scripts drawing two dialogs would own even less.

So the script says **what** to put in front of the user, and the root says where it goes and how it leaves. What crosses the boundary is small: a view class to instantiate, a side to anchor to, a sentence to show.

## The surface

```js
const depth = cx.open_dialog(ConfirmClear, {
  escape_dismissable: false,
  backdrop_dismissable: false,
  props: { count },
});
cx.close_dialog();        // -> did anything close?
cx.close_all_dialogs();   // -> how many closed

cx.open_sheet("right", FiltersPanel, { props: { filters } });
cx.close_sheet();         // -> did anything close?

cx.toast({ title: "Saved", description: "3 files", level: "success",
           timeout: 4000, id: "save" });
cx.dismiss_toast("save");
cx.dismiss_all_toasts();
```

## Dialogs

`cx.open_dialog(ViewClass, options?)` takes **the class itself** — not an instance, and not an element:

```text
expected a view class; open_dialog and open_sheet take the class itself,
not an instance and not an element
```

The runtime constructs one instance, passing `options.props` to it, and mounts it as a script view like any other. The dialog's content is an ordinary view with its own `init` and `render`:

```js
// confirm.js
import { View, v_flex, h_flex, text } from "gpui";

export default class ConfirmClear extends View {
  init(props) {
    this.count = props?.count ?? 0;
  }

  render() {
    return v_flex()
      .w(360)
      .bg("surface")
      .border(1)
      .border_color("border")
      .p(24)
      .gap(12)
      .child(text(`Delete ${this.count} completed items?`))
      .child(text("This cannot be undone."))
      .child(
        h_flex()
          .justify_end()
          .gap(8)
          .child(cancelButton((_event, cx) => cx.close_dialog()))
          .child(deleteButton((_event, cx) => cx.close_dialog())),
      );
  }
}
```

Note what the root supplies and what it does not. It supplies the backdrop, the position, the layering, the focus trap and the surface it sits on; the width, the padding, the border, the type and the buttons are the script's, like everything else in this runtime.

| Option | Default | Effect |
| --- | --- | --- |
| `escape_dismissable` | `true` | Whether Escape closes it |
| `backdrop_dismissable` | `true` | Whether pressing the backdrop closes it |
| `props` | — | Passed to the class's constructor, and so to `init` |

An unknown option is refused rather than ignored, which is the point:

```text
unknown option `escapeDismissable` for cx.open_dialog(view, options);
expected escape_dismissable, backdrop_dismissable or props
```

A silently ignored `escapeDismissable` would look like it worked, and the dialog would be dismissable anyway.

`open_dialog` returns the **new depth of the stack**, not a handle. The root addresses dialogs by position and never by identity, so a handle would have to promise "close *this* dialog", which is not an operation that exists. The depth is what a script can use — to assert one opened, or to unwind to a known level. `close_dialog` returns whether it found one to close; `close_all_dialogs` returns how many it closed.

::: warning Do not carry `cx` into the dialog
The `cx` in the handler that opened the dialog belongs to that handler. By the time the dialog's own button is pressed, it is stale, and using it reports a stale-context error. Pass **data** through `props`, and take `cx` from the dialog's own callback arguments.
:::

## The sheet

```js
cx.open_sheet("right", FiltersPanel, { props: { filters } });
```

At most one sheet is open at a time, anchored to `"left"`, `"right"`, `"top"` or `"bottom"`. Its only option is `props`; it has no dismissal options, because there is only ever one and it is dismissed by Escape or by its overlay whenever no dialog is above it.

```text
unknown sheet side `middle`; expected left, right, top or bottom
```

## Toasts

A toast is the one overlay that is **data rather than a view** — no class, no instance, nothing for the script to render — which is why its whole content crosses the boundary as an options object.

| Field | Default | Meaning |
| --- | --- | --- |
| `title` | required | The sentence the user reads |
| `description` | — | A second line |
| `level` | `info` | `info`, `success`, `warning` or `error` |
| `timeout` | 5 s | Milliseconds, or `null` to stay until dismissed |
| `id` | generated | Identity, for replacing and dismissing |

An omitted `timeout` keeps the default and an explicit `null` makes the toast sticky, so the two cannot be collapsed into one option.

The `id` is what turns a repeated failure into one standing message instead of a pile. The `--watch` loop uses exactly this: a failed reload posts a sticky error toast with a fixed id, so saving a broken file five times replaces the message rather than stacking five of them, and the next successful reload retracts it with `dismiss_toast`.

```text
unknown toast level `fatal`; expected info, success, warning or error
```

Three toasts are mounted at once. Older ones stay in the manager and reappear as newer ones leave, so a burst is throttled rather than lost.

## Stacking and dismissal

Painted back to front:

1. **Content** — the script's root view.
2. **Sheet** — at most one, anchored to an edge. A sheet is a *place* in the window, so it sits below the dialog stack: a dialog raised from inside a sheet must be readable, and a sheet raised under a dialog must not cover it.
3. **Dialog stack** — in open order, oldest at the bottom.
4. **Toasts** — above everything. A toast reports the outcome of the action the user just took, and an open dialog is exactly the situation where that outcome matters most, so it is the one layer that is never occluded.

Only the topmost dialog draws a backdrop: a stack of three dims the window once, not three times, and that single backdrop is what separates the live dialog from the inert ones behind it.

Dismissal is always **one layer, never a cascade**:

- **Escape** closes the topmost dialog only. Lower dialogs render with keyboard handling disabled, so repeated Escapes unwind the stack one dialog per press and never reach the sheet while a dialog is open.
- `escape_dismissable: false` withdraws the **key binding**, not the underlying cancel action. A close control the script puts inside the dialog still works — which is what makes an undismissable dialog one the user must answer rather than one they cannot leave.
- **Backdrop press** closes the topmost dialog, and only if it was opened with `backdrop_dismissable`.
- **Enter does nothing** at this layer. Base's dialog host treats Enter as "confirm and close"; that belongs to the dialog's own primary button, which the script owns, so the root vetoes the built-in confirmation rather than guessing which content wanted it.
- A **sheet** is dismissed by Escape or by its overlay only when no dialog is open, because a dialog above it holds focus.
- `close_all_dialogs` is the one operation that unwinds the whole stack, and it leaves the sheet alone.

**Focus** is restored through the stack's own history. Opening an overlay records what was focused and focuses the overlay; closing it restores that handle. Closing the second dialog returns focus to the first, and closing the first returns it to whatever the window was on before either opened. Tab and Shift-Tab honour the focus trap, so tabbing inside an overlay cycles within it rather than walking into the content behind it.

## The phase rule

**An overlay may only be opened or closed from an event handler or a task.**

```text
cx.open_dialog(view, options) is not allowed during the `render` phase;
overlays may only be opened or closed while handling an event or a task
```

Opening or closing an overlay mutates the window, and the render pass is reading it. GPUI's borrow model has no way to express "the script may notify from here but not from there", so the runtime carries the [phase](./state.md#phases) explicitly and every overlay entry point refuses `render`, `layout`, and being called from outside any host call at all — in the last case there is no window to reach either.

The refusal names the phase it came from, because that is the only clue the author has.

## Overlays need a `ShellRoot`

Every one of these calls reaches the window's root view. A window whose first view is not a `ShellRoot` refuses them, and says which mistake it was — a host wiring problem, not a script one:

```text
cx.open_dialog(view, options) needs a ShellRoot as the window's first view;
this window was opened with another view
```

See [Getting started](./getting-started.md#add-the-runtime-to-a-rust-application).

## Not there yet

- **A result from a dialog.** `open_dialog` returns a depth, not a promise that settles when the dialog closes. Pass a callback through `props`, or have the dialog write back to state the opener reads.
- **Popovers, tooltips and context menus.** Base has the parts; the script surface has none of them.
- **Positioning options.** A dialog is centred and a sheet is edge-anchored; neither can be placed.

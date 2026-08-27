---
title: State and Views
description: Views, init and render, cx.notify(), retained input state, and asynchronous work.
order: 6
---

# State and Views

A view is the one thing in this runtime that has an identity, survives a frame, and is owned by GPUI. Everything else — elements, callbacks, the `cx` handed to a call — belongs to the pass that created it.

## Defining a view

```js
import { View } from "gpui";

export default class Counter extends View {
  init(props) {
    this.count = props?.start ?? 0;
  }

  render(cx) {
    return v_flex().child(text(`${this.count}`));
  }
}
```

`init` runs once, when the view is created. It is where state that survives frames is set up — plain fields, and any [retained entity](#retained-state) the view needs.

`render` **returns exactly one element**, and runs when the view has been invalidated rather than on every frame — see [When `render` runs](#when-render-runs). Returning something that is not an element built with `gpui` fails immediately:

```text
render(cx) must return an element built with gpui
```

`main.js` must `export default` a view class. The host constructs one instance and mounts it as the window's root view; a module whose default export is not a class is refused with a message saying so.

Never store an element on the instance. See [Elements](./elements.md#elements-are-single-use).

## `cx.notify()`

Nothing repaints on its own. There are no signals, no observables and no automatic dependency tracking. Change state, then ask for a re-render:

```js
add(cx) {
  this.items = [...this.items, { id: this.nextId, caption, done: false }];
  this.nextId += 1;
  cx.notify();
}
```

This runs against the whole default assumption of the front-end ecosystem, so it is worth stating flatly: **there is no `useState` here, and no dependency array.** Three reasons the runtime does not add one.

GPUI is itself an explicit-`notify` model, and two reactive mental models inside one application interfere with each other rather than compose. Automatic tracking would mean wrapping every view instance in a `Proxy`, which is a permanent cost on the render path — and QuickJS has no JIT to amortize it. And a missing `notify` has a determinate symptom: the interface does not update. That is far cheaper to find than an automatic system that fires too often.

Several `notify` calls inside one event handler collapse into a single repaint — and into a single `render`.

## When `render` runs

`render` does **not** run once per frame. GPUI repaints for reasons your application never hears about — a pointer moving over a button, a text cursor blinking, a list scrolling, an animation advancing — and none of those are a reason to run JavaScript.

So a `render` call does not describe *this frame*. It describes the interface once, into a snapshot the runtime keeps:

```text
cx.notify()  ──▶  render()  ──▶  snapshot  ──┬──▶  frame
                                             ├──▶  frame
                                             └──▶  frame  …
```

The snapshot is rebuilt when, and only when, something invalidates it:

- `cx.notify()` from an event handler or an async task
- a [hot-reload](./getting-started.md) replacing the script
- a theme change, because `bg(cx.theme().colors.surface)` records a real colour while `render` runs and bakes it into the snapshot
- the host calling `ScriptView::refresh`, which is how Rust says it changed state your script reads through a [native module](./capabilities.md). A plain `cx.notify()` from the host is a repaint and runs no script — the two are different requests

Everything else replays the description you already produced, in Rust, without running any JavaScript.

Three consequences worth holding on to:

**Your `render` cost follows your users, not your frame rate.** A view that changes ten times a second costs ten renders a second, whether the window is repainting at 60 FPS or 120. Describing a large panel is affordable precisely because it is not being redescribed sixty times for no reason.

**Hover, focus and active styles never call back into script.** `.hover(s => s.opacity(0.8))` is resolved into a native style description while the snapshot is built, and GPUI applies it from there. A pointer moving across your interface runs no JavaScript at all. The same is true of an [`Input`](#retained-state)'s cursor and selection.

**A failed `render` does not destroy the interface.** A snapshot is published only after `render` returns successfully, so a script that throws leaves the previous description — and the handlers registered with it — exactly as they were. The failure appears as a banner **over** the interface that still works, saying it is one version behind and offering the detail for pasting somewhere; you keep your scroll position and your focus. A view whose very first render failed has nothing to keep, and gets the full error surface instead. Either way the failing `render` is not re-run until something invalidates the view again.

## Phases

Every call from Rust into the script opens a scope carrying a **phase**, and the phase decides what the `cx` for that call may do.

| Phase | When | May | May not |
| --- | --- | --- | --- |
| `render` | Building an element tree | Read state, build elements, register callbacks | `notify`, open overlays, create retained state |
| `event` | Handling a click or a change | Everything | Block |
| `task` | Resuming asynchronous work | Everything | Block |
| `layout` | Rendering one virtualized item inside GPUI's layout pass | Read state, build elements | `notify`, open overlays, create retained state |

`cx.phase()` reports the current one, and `"none"` outside any host call.

`cx.theme()` returns a deeply read-only snapshot of gpui-base's current semantic theme for this call: direct color roles as well as `colors`, `spacing`, `radius`, `appearance`, and `is_dark`. Prefer it over the compatibility `theme()` export, because the context spelling makes the call lifetime and current host theme explicit.

Each refusal is a specific message, not undefined behaviour:

```text
cx.notify() is not allowed during the `render` phase;
request a re-render from an event handler instead
```

Notifying yourself while rendering is a loop, which is why it is refused rather than deferred.

## The `cx` belongs to its call

`&mut Window` and `&mut App` are borrows in GPUI: they live exactly as long as one call. A script object outlives any borrow, so the script-side `cx` cannot hold them. It holds a **generation number** instead, checked against the live scope stack on every use.

Keep a `cx` past its call and you get an error rather than a corrupted frame:

```text
cx is no longer valid: it was captured during an earlier call and used later.
Use gpui.spawn or take cx from the callback arguments instead.
```

`cx` exposes nothing but functions — `Object.keys(cx)` shows the methods and no generation — so a script cannot forge one.

The most common way to hit this is an `await`:

```js
async save(cx) {
  await sleep(100);
  cx.notify();                              // wrong: this cx belongs to a call that returned
  with_cx((cx) => cx.notify());             // right
}
```

An `await` returns control to the host, the call frame goes away, and the borrows go with it. `with_cx(fn)` asks for a fresh `cx` belonging to whatever call is running now.

## Retained state

A view's own fields hold plain data. Anything with cross-frame machinery of its own — a text field's content, cursor position and undo history — lives in a GPUI entity, and the script holds a **handle** to it.

```js
import { InputState, Input } from "gpui";

init() {
  this.draft = InputState.new({ placeholder: "What needs doing?" });
  this.draft.on("submit", (_event, cx) => this.add(cx));
}

render(cx) {
  return Input.new(this.draft)
    .flex_1()
    .h(28)
    .px(8)
    .border(1)
    .border_color(cx.theme().colors.input)
    .bg(cx.theme().colors.surface)
    .text_size(12);
}
```

| Call | Effect |
| --- | --- |
| `InputState.new({ placeholder, value })` | Creates the state; both options are optional |
| `state.value()` | The current text |
| `state.set_value(text)` | Replaces it |
| `state.on(event, handler)` | Subscribes; see below |
| `state.release()` | Drops the handle |
| `Input.new(state)` | The element that renders it |

**Create it in `init` or an event handler, never in `render`.** Creating an entity needs a live window, and the render pass is the one place where doing so would be wrong anyway:

```text
InputState.new(...) cannot run during render; create state in init()
or in an event handler and keep it on the view
```

The script holds a handle, not the entity — GPUI owns that. Using a released handle throws rather than returning `undefined`, because an `undefined` in JavaScript travels a long way before it fails and by then the origin is gone:

```text
this input state has been released
```

`Input` is the one element the runtime gives defaults to, and only three: a centred row, full width, and a click anywhere in the frame focuses it. Each is a default a script can override but should not have to remember — without the first, text sits at the top of whatever height the frame was given, which on screen looks like a bug rather than a missing style.

### Input events

```js
this.draft.on("submit", (event, cx) => this.add(cx));
```

| Event | Fires on |
| --- | --- |
| `change` | The text changed |
| `submit` | Enter was pressed; `event.secondary` and `event.shift` say how |
| `focus` | The field gained focus |
| `blur` | It lost focus |

Unlike a rendered `on_click`, this subscription **outlives the render that created it**. The subscription is owned by the runtime's handle store rather than by the script, because a script has nowhere to keep it and a handler that stops firing because a value was garbage collected is the kind of bug nobody finds. It is released when the handle is.

A misspelled event name lists the valid ones:

```text
unknown input event `changed`; expected one of: change, submit, focus, blur
```

## Asynchronous work

Script code is asynchronous in the ordinary JavaScript way — `async` functions and native promises. The runtime supplies the parts a bare QuickJS does not have: a clock, an owner for pending work, and something to pump the job queue.

| Export | Effect |
| --- | --- |
| `sleep(ms)` | A promise resolved after `ms` on GPUI's foreground executor |
| `spawn(body, opts?)` | Calls `body(cx)` and adopts the promise it returns |
| `timer.after(ms, handler, opts?)` | Calls `handler(cx)` once |
| `timer.every(ms, handler, opts?)` | Calls `handler(cx)` repeatedly |
| `with_cx(body)` | Runs `body(cx)` with a context belonging to the call in progress |

All of them return work that runs on the main thread. Nothing script-visible ever leaves it: there is no `Worker`, and the VM and GPUI's `App` are both main-thread only.

```js
import { spawn, sleep, with_cx } from "gpui";

flash(cx) {
  this.saved = true;
  cx.notify();

  spawn(async () => {
    await sleep(1500);
    with_cx((cx) => {
      this.saved = false;
      cx.notify();
    });
  });
}
```

::: tip Two ways to import
`import { spawn, sleep } from "gpui"` names what you use, and is what the example application does. `import * as gpui from "gpui"` puts the UI and scheduling surface under one name — for example `gpui.spawn` and `gpui.timer.after`. Filesystem and process APIs remain separate standard modules. There is no default export.
:::

**`spawn` adopts the promise, and that is the point.** An unhandled rejection is JavaScript's most common silent failure: the work stops, the interface keeps the state it had, and nothing is written anywhere. Here it reaches `tracing::error!` with the script's own stack.

### Ownership and cancellation

Every task belongs to a view — `opts.owner`, or the view that is running when it is created. The task holds a weak reference, so when the panel that started the work goes away the callback is skipped rather than writing into state nothing will render again.

```js
import { timer } from "gpui";

const handle = timer.every(1000, (cx) => this.tick(cx));
handle.cancel();
handle.is_done();
```

`owner: null` opts out and outlives every view; it is the only value other than the current view the runtime accepts today.

Cancelling a `sleep` leaves its promise **pending for ever**. That is what cancellation means for a promise: the continuation does not run, and no error is invented for code that asked to stop.

`timer.every` measures its interval from the end of one call, so a slow handler delays the next tick rather than stacking ticks behind it.

### Timers and standard host APIs

```text
setTimeout  -> gpui.timer.after(ms, callback)
setInterval -> gpui.timer.every(ms, callback)
clearTimeout / clearInterval -> cancel() the Task returned by after / every
```

`setTimeout`, `setInterval`, `clearTimeout` and `clearInterval` are throwing stubs. Use `gpui.timer.after` for one-shot work, `gpui.timer.every` for repeated work, and call `cancel()` on the returned `Task` to stop either one. Global `fetch` and the safe standard modules documented under [Capabilities](./capabilities.md), including `websocket`, are real asynchronous host APIs. CommonJS `require` remains unavailable; use ES modules.

Browser DOM and storage are absent: there is no `document` or `localStorage`. The global `window` is gpui-shell's overlay host for dialogs, sheets and toasts; it is not a browser `Window` and exposes no DOM.

## Not there yet

- **Global and cross-view state.** There is no store beyond the persistence layer in [Capabilities](./capabilities.md) and ordinary module scope.
- **Actions and key bindings.** `gpui.action` and `gpui.keymap` are designed but not bound; the only key handling today is what `ShellRoot` installs (Tab, Shift-Tab, Escape).
- **Multiple windows.** The host opens the window; there is no `gpui.open_window`.
- **`gpui.gc_stats()`**, and the debug panel that would read it.

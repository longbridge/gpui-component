---
title: Hosting the Runtime
description: The Rust side in full — runtime lifetime, mounting script views, refreshing them from host state, metrics, exit requests and hot-reload.
order: 10
---

# Hosting the Runtime

[Getting Started](./getting-started.md) shows the four lines that put a script view on screen. This page is the rest of the Rust surface: what to call, when, and the two or three places where the obvious call is the wrong one.

## The runtime

One `ShellRuntime` owns one VM. It is an `Rc` with interior mutability — neither `Send` nor `Sync` — so it lives on the thread that owns the `App`.

```rust
gpui_shell::init(cx);                     // gpui-base, the token palette, the style table

let runtime = ShellRuntime::new()?;       // one VM
runtime.set_global(cx);                   // reachable later as ShellRuntime::global(cx)
```

`set_global` is what lets a callback, a native module or a hot-reload find the runtime again without the host threading a handle through everything. A host running one application calls it once; there is one global, so a host that wants two isolated runtimes keeps the second handle itself.

## Loading and instantiating

Loading turns source into a **view type** — the class the script default-exports. Instantiating turns that type into a **view object**, one live instance:

```rust
let view_type = runtime.load_app(&root, "main.js")?;   // a directory
let view_type = runtime.load_source("inline", source)?; // a string, for tests

let object = runtime.instantiate(&view_type, window, cx)?;
```

`load_app` resolves the directory, reads the entry file, and evaluates the module. Every failure here is a `ShellError` carrying the script's own stack — a syntax error, an import that resolves outside the application root, a missing or misshapen default export.

Instantiating runs the script's `init`, which means it needs a live `Window`: it may create retained state such as an `InputState`.

## Mounting

A script view is a GPUI view like any other, and it goes **under a `ShellRoot`**:

```rust
cx.open_window(options, move |window, cx| {
    let object = runtime.instantiate(&view_type, window, cx).expect("view");
    let content = cx.new(|_| ScriptView::new(runtime.clone(), object));
    cx.new(|cx| ShellRoot::new(content.into(), window, cx))
})
```

`ShellRoot` owns the dialog stack, the sheet, the toast stack, focus restoration and Tab navigation — the same role `Root` plays for a `gpui-component` window. `cx.open_dialog` and friends reach it through the window, so a script mounted under any other root view gets a refusal naming the reason rather than a silent no-op.

The host can drive the same surfaces directly, which is how a plugin panel and the host's own UI end up in one stack:

```rust
root.update(cx, |root, cx| {
    root.open_dialog(view.into(), window, cx);
    root.push_toast(ToastRequest::new("Saved").with_level(ToastLevel::Success), window, cx);
    root.close_all_dialogs(window, cx);
});
```

## Refreshing a view from host state

This is the one call that is easy to get wrong, and the mistake is silent.

```text
cx.notify()        ── draw this view again          (no script runs)
view.refresh(cx)   ── and its description is stale  (the script runs)
```

Because a script `render` is [not a frame render](./state.md#when-render-runs), a plain `cx.notify()` repaints the snapshot that already exists. If the host changed something the script *reads* — an entity behind a native module, a setting, a document — the view must be told the description itself is out of date:

```rust
script_view.update(cx, |view, cx| view.refresh(cx));
```

`refresh` is `invalidate` plus `notify`. `invalidate` alone marks the view without scheduling a frame, which is what you want when several changes land together and one repaint should cover them.

Getting it wrong in the other direction is visible immediately — the interface simply does not update — which is the same failure mode as a forgotten `cx.notify()` in GPUI itself.

## What a script may reach

Three grants, each read at call time so a change takes effect on the next call rather than the next restart:

```rust
gpui_shell::set_capabilities(
    Capabilities::new()
        .with_read_roots([app_root.clone()])
        .with_write_roots([data_dir.clone()])
        .store(true),
);
gpui_shell::set_store_path(data_dir.join("store.json"));
gpui_shell::set_native_modules(modules);
```

All three default to nothing: no file access, no storage location, no native modules. See [Capabilities](./capabilities.md) and [Native Modules](./native.md).

## Watching what it costs

The runtime counts two events separately, and the gap between them is the point:

```rust
let reading = runtime.metrics().read();
reading.script_renders();      // follows cx.notify(), reloads, theme changes
reading.materializations();    // follows frames
reading.script_render_time();  // total time inside script `render`
reading.native_time();         // of which, inside native modules
reading.slowest_script_render();
```

`RuntimeMetrics::since(&earlier)` gives the delta between two readings, which is how a per-second rate is built without resetting anything. `metrics().reset()` starts a fresh measurement — the Shell story resets it whenever its feed changes, so the readout answers "what is this feed costing" rather than "what has this window done since it opened".

A regression test can assert on `script_renders` directly; that is what keeps [the benchmark's third figure](./engine.md#the-measurement) honest.

## Exit requests

`process.exit(code)` from a script is **a request, never `exit(2)`**. One plugin must not be able to take the host process down, and the host may have unsaved state. The runtime hands the request to the host, and the host decides:

```rust
gpui_shell::on_exit_request(|request, window, cx| {
    match request.view() {
        Some(view) => close_the_panel_showing(view, window, cx),
        None => cx.quit(),
    }
});
```

`request.code()` is the exit code the script asked for, and `request.view()` names the view it came from, when there is one — a plugin host closes *that* plugin's panel, where one that quit the window would let a plugin end someone else's work.

**A host that grants exit without installing a handler is told at the call**, not never: `process.exit()` throws, naming `on_exit_request`. A request nobody answers is a lie told in the flattering direction — the script gets a success and nothing happens.

## Hot-reload

One call starts it, and it is the same one the `--watch` flag uses:

```rust
gpui_shell::watch::reload_in_debug(
    &runtime, &view, app_root.clone(), "main.js", window, cx,
).forget();
```

Two things about that signature. It **does nothing in a release build** — it returns an inert handle, so leaving the call in costs a branch. And the returned `Watch` is the watch: dropping it stops the loop, which is what a host unmounting a panel wants, while `.forget()` lets it run for as long as the view does. The loop also ends on its own when the view, the runtime or the window goes away, because it holds all three weakly — a strong handle here would keep a panel's runtime alive after the dock removed it.

A reload re-reads **every** module, entry point included — a hot-reload that quietly served a stale import would be worse than none, because it looks like it worked. It does all of its fallible work before touching the live view: if the new code fails to load, the previous view keeps running, the error goes to `tracing`, and a toast with a stable id reports it in the window. The next successful reload retracts that toast.

The view survives a reload. `ScriptView::replace_object` swaps what the script produced while keeping the entity, and with it the window, the focus and the element identities.

## When a script fails

A script that throws does not take the interface with it. The last good snapshot stays mounted and the failure is reported over it, so the reader keeps their scroll, their focus, and whatever they were reading. The runtime does not re-run a failing `render` until something invalidates the view again.

Install a `tracing` subscriber. The runtime reports script errors, unhandled promise rejections and illegal-phase calls through `tracing` with the target `gpui_shell::script`; with no subscriber every one of them is discarded, and the symptom is a view that quietly stopped responding.

## Not there yet

- **Two runtimes in one process.** There is one global handle; a second runtime has to be threaded by hand.
- **A supervisor for scripts that hang.** The interpreter's own interrupt cuts a call off, but nothing restarts a runtime that keeps hitting it.
- **The plugin model.** `PluginManager` and `PluginManifest` are written and tested but crate-private, because nothing loads a plugin yet: publishing them would be a promise about an API no caller has exercised. The grants above are the whole story for a host running one application, and `Policy` is public for a host running more than one.

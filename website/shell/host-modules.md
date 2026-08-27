---
title: Host Modules
description: How a host lends its own Rust to a script — registration, the import that reaches it, the plain-data boundary, and the rules a host function runs under.
order: 9
---

# Host Modules

[Capabilities](./capabilities.md) is the half that says what a script may **not** reach. This is the other half: what the host chooses to hand it.

A script cannot load a native extension. `dlopen`-ed Rust has no stable ABI, and once it is inside the process it holds every permission the process holds — a sandbox that permits that does not mean anything. So the direction is reversed. **The host registers, at compile time, the Rust it is willing to expose**, and a script reaches exactly that and nothing else.

```rust
use gpui_shell::{HostModules, HostValue};

let mut modules = HostModules::new();
modules.register("workspace", |module| {
    module.function("project_name", |_| Ok(HostValue::from("gpui-component")));
});
gpui_shell::export_modules(modules)?;
```

```js
import { project_name } from "workspace";

project_name();      // "gpui-component"
```

A registered module is an ordinary ES module, resolved by the same loader that answers `gpui` and `path`. The rest of this page is what that costs and what it refuses.

## Why an import rather than a lookup

The earlier shape was a call — `native("workspace")` answering with a bag of functions. Two things were wrong with it, and both are about *when* you find out:

- **A misspelled export was a run-time failure.** `workspace.projectName()` type-checked, loaded, rendered, and then threw on the frame that first reached it.
- **The type declarations could not say anything.** Only the host knows what it registered, so the best a generated `gpui.d.ts` could offer was `Record<string, (...args: any[]) => any>`, and an application that wanted real types hand-wrote a `.d.ts` that nothing checked against the registry.

As an import, a wrong name fails while the module graph is linked — before a line of the application runs — and the declarations are [generated from the registry itself](#typing-them).

What the import does **not** freeze is the function behind the name. Every export is a forwarding stub that resolves through the registry on each call, so withdrawing a module still takes effect immediately: a script holding an imported function gets a refusal, not the withdrawn closure. Only the *set of names* is fixed, at the moment the importing module is linked — which is why a host calls `export_modules` **before** it loads an application.

## The registry is the grant

The default registry is **empty**, the same shape as `Capabilities::default()`. A host that registers nothing has granted no extension surface, and a script that imports a module is told so by name:

```text
host module `market` is not available: this host registered none.
Host modules are granted by the embedding application, with
gpui_shell::export_modules(...).
```

Register something and the message changes to name what does exist:

```text
unknown host module `marker`; this host registered: market, theme
```

```text
host module `market` has no function `quote`; it provides: quotes, ticks, watch, watch_all
```

There is deliberately no per-module capability to grant on top of this. The host chose the list, so **the list is the grant** — and revoking one is a matter of exporting a different set, which takes effect on the next call rather than the next restart.

For a multi-application host, each public `Policy` carries its own frozen capabilities and its own module registry. That is how two plugins in one runtime receive different authority without swapping thread-local state across `await` boundaries. Identity and requested system permissions live in `gpui-shell.json`; host modules do not, because contributions are executable behavior registered by the host.

## Names the runtime keeps

A host module shares one specifier namespace with the built-in modules and the [Standard Runtime](./engine.md), and the resolver reaches those first. So registering `path` would not shadow the real `path` — it would register a module nothing can ever import, silently.

`export_modules` refuses those names instead, and names them:

```text
these module names belong to the runtime and cannot be registered: path, gpui.
The reserved names are: gpui, gpui-base, gpui-fps, buffer, console, crypto,
fs/promises, net, os, path, process, url, websocket, zlib
```

The full list is `gpui_shell::RESERVED_SPECIFIERS`. Everything else is yours — and cannot be shadowed by a file in the application directory either, because host modules resolve before the application's own files.

## The boundary is plain data

A host function receives `HostArguments` and returns a `HostValue`: null, boolean, number, string, array, or object. Those six cases are the intersection of what a script engine and JSON can both carry, which is what lets one registry serve any engine behind the [seam](./engine.md).

It never receives a script handle. A handle would let the host keep a reference to a script value past the call that produced it — and past the call scope that made the surrounding context valid.

Arguments come out by position, with the type check and the error message included:

| Call | Yields |
| --- | --- |
| `arguments.string(0)` | `&str`, or an error naming what arrived instead |
| `arguments.number(0)` | `f64` |
| `arguments.integer(0)` | `i64`, refusing a fractional number |
| `arguments.boolean(0)` | `bool` |
| `arguments.value(0)` | The raw `HostValue`, for a function that accepts more than one shape |
| `arguments.get(0)` | `Option<&HostValue>`, for an optional argument |

Returning a record is a builder rather than a map, because an object frequently *is* the row a script renders and insertion order should be the host's to decide:

```rust
use gpui_shell::HostObject;

HostObject::new()
    .field("symbol", "AAPL.US")
    .field("last", 224.22)
    .field("watched", true)
```

An error is a message, not a type: `HostError::new("no such symbol")` reaches the script as a thrown `Error` the script can catch.

## Three rules a host function runs under

**It must not call back into the script engine.** A host call happens inside a script call, which is inside a host call; re-entering the VM from there would run script code with an engine frame already on the stack, in the middle of a render pass. Holding no script handle makes that hard to express by accident, and the dispatcher refuses a nested call outright so a host that finds another route gets a diagnosable error rather than undefined behavior.

**Reading and writing host state is the point.** A function reaches the ambient `App` through `gpui_shell::with_current_app`, which is `None` outside a live call:

```rust
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, HostError> {
    gpui_shell::with_current_app(read)
        .ok_or_else(|| HostError::new("only reachable while a script call is in progress"))
}
```

**`cx.notify()` from inside one is delivered after the call unwinds.** So a host function may mutate an entity and ask the views watching it to re-render, without that re-render happening underneath the script that called it.

## Typing them

A module describes its own TypeScript face, in Rust, beside the registration:

```rust
modules.register("market", |module| {
    module.declarations(r#"
        /** One row of the board, as it crosses the boundary. */
        export interface Quote { symbol: string; last: string; watched: boolean }

        /** Every row on the board. */
        export function quotes(): Quote[];
        /** Flips one row's watched flag and answers the new value. */
        export function watch(symbol: string): boolean;
    "#);

    module.function("quotes", /* … */);
    module.function("watch", /* … */);
});
```

The generated `gpui.d.ts` emits that verbatim inside `declare module "market"`, so `import { quotes } from "market"` is checked exactly the way `import { div } from "gpui"` is.

Writing it here rather than in a `.d.ts` beside the script is what makes the two halves one thing. `export_modules` compares the declared exports with the registered ones and refuses a mismatch:

```text
host module `market` declares a different set of functions than it registers;
registered but not declared: quotes; declared but not registered: prices
```

Renaming a function on one side is now a sentence at start-up rather than an editor that keeps completing a function the host deleted.

Declaring nothing is allowed and costs only precision. An undeclared module is emitted with permissive signatures:

```ts
declare module "audit" {
  export function observe(...args: any[]): any;
}
```

which still checks the module name and every export name.

## A real one

The gallery's Shell story registers one market module, and it is the entire extension surface its script has. Theme values come from `cx.theme()` instead. This is the host side:

```rust
fn install_host_modules(market: &Entity<Market>) {
    let mut modules = HostModules::new();

    modules.register("market", |module| {
        module.declarations(MARKET_TYPES);

        let read = market.clone();
        module.function("quotes", move |_| with_app(|cx| read.read(cx).to_host_value()));

        let flip = market.clone();
        module.function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // Delivered after this call unwinds, so it cannot re-enter
                    // the engine: the story and the script view re-render together.
                    cx.notify();
                    Ok(HostValue::from(watched))
                })
            })?
        });
    });

    gpui_shell::export_modules(modules).expect("`market` is not a reserved name");
}
```

And this is the script that uses it — the same `Market` entity a Rust panel beside it is rendering from:

```js
import { quotes, watch } from "market";

const rows = quotes();
const watched = rows.filter((quote) => quote.watched).length;
```

Run it with `cargo run -- shell`. The two panels read one entity through two paths, which is what makes a mismatch between them visible immediately.

## Not there yet

- **Asynchronous host functions.** A function returns a value, not a promise; long work blocks the thread that renders.
- **Classes and object identity.** A module exports functions. Exporting a class would mean handing the script a live host object, which the plain-data boundary above rules out; a factory function returning a record does the same work today.
- **Per-function grants inside one registry.** A policy grants the registry the host assembled; it does not add another permission switch for each function.
- **Streaming or callbacks into the host.** A script cannot hand a function to a host module; the module can only be called.

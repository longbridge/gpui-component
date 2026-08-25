---
title: Native Modules
description: How a host lends its own Rust to a script — registration, the plain-data boundary, and the rules a native function runs under.
order: 8
---

# Native Modules

[Capabilities](./capabilities.md) is the half that says what a script may **not** reach. This is the other half: what the host chooses to hand it.

A script cannot load a native extension. `dlopen`-ed Rust has no stable ABI, and once it is inside the process it holds every permission the process holds — a sandbox that permits that does not mean anything. So the direction is reversed. **The host registers, at compile time, the Rust it is willing to expose**, and a script reaches exactly that and nothing else.

```rust
use gpui_shell::native::{NativeModules, NativeValue};

let mut modules = NativeModules::new();
modules.register("workspace", |module| {
    module.function("project_name", |_| Ok(NativeValue::from("gpui-component")));
});
gpui_shell::set_native_modules(modules);
```

```js
import { native } from "gpui";

const workspace = native("workspace");
workspace.project_name();      // "gpui-component"
```

That is the whole mechanism. The rest of this page is what it costs and what it refuses.

## The registry is the grant

The default registry is **empty**, the same shape as `Capabilities::default()`. A host that registers nothing has granted no native access, and a script that asks for a module is told so by name:

```text
native module `market` is not available: this host registered none.
Native modules are granted by the embedding application, with
gpui_shell::set_native_modules(...).
```

Register something and the message changes to name what does exist:

```text
unknown native module `marker`; this host registered: market, theme
```

```text
native module `market` has no function `quote`; it provides: quotes, ticks, watch, watch_all
```

There is deliberately no per-module capability to grant on top of this. The host chose the list, so **the list is the grant** — and revoking one is a matter of registering a different set, which takes effect on the next call rather than the next restart.

## The boundary is plain data

A native function receives `NativeArguments` and returns a `NativeValue`: null, boolean, number, string, array, or object. Those six cases are the intersection of what a script engine and JSON can both carry, which is what lets one registry serve any engine behind the [seam](./engine.md).

It never receives a script handle, and that is not a convenience. A handle would let the host keep a reference to a script value past the call that produced it — and past the call scope that made the surrounding context valid.

Arguments come out by position, with the type check and the error message included:

| Call | Yields |
| --- | --- |
| `arguments.string(0)` | `&str`, or an error naming what arrived instead |
| `arguments.number(0)` | `f64` |
| `arguments.integer(0)` | `i64`, refusing a fractional number |
| `arguments.boolean(0)` | `bool` |
| `arguments.value(0)` | The raw `NativeValue`, for a function that accepts more than one shape |
| `arguments.get(0)` | `Option<&NativeValue>`, for an optional argument |

Returning a record is a builder rather than a map, because an object frequently *is* the row a script renders and insertion order should be the host's to decide:

```rust
use gpui_shell::native::NativeObject;

NativeObject::new()
    .field("symbol", "AAPL.US")
    .field("last", 224.22)
    .field("watched", true)
```

An error is a message, not a type: `NativeError::new("no such symbol")` reaches the script as a thrown `Error` the script can catch.

## Three rules a native function runs under

**It must not call back into the script engine.** A native call happens inside a script call, which is inside a host call; re-entering the VM from there would run script code with an engine frame already on the stack, in the middle of a render pass. Holding no script handle makes that hard to express by accident, and the dispatcher refuses a nested call outright so a host that finds another route gets a diagnosable error rather than undefined behavior.

**Reading and writing host state is the point.** A function reaches the ambient `App` through `gpui_shell::scope::with_current_app`, which is `None` outside a live call:

```rust
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, NativeError> {
    gpui_shell::scope::with_current_app(read)
        .ok_or_else(|| NativeError::new("only reachable while a script call is in progress"))
}
```

**`cx.notify()` from inside one is delivered after the call unwinds.** So a native function may mutate an entity and ask the views watching it to re-render, without that re-render happening underneath the script that called it.

## A real one

The gallery's Shell story registers exactly two modules, and they are the entire extension surface its script has. This is the host side:

```rust
fn install_native_modules(market: &Entity<Market>) {
    let mut modules = NativeModules::new();

    modules.register("market", |module| {
        let read = market.clone();
        module.function("quotes", move |_| with_app(|cx| read.read(cx).to_native()));

        let flip = market.clone();
        module.function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // Delivered after this call unwinds, so it cannot re-enter
                    // the engine: the story and the script view re-render together.
                    cx.notify();
                    Ok(NativeValue::from(watched))
                })
            })?
        });
    });

    modules.register("theme", |module| {
        module.function("palette", |_| with_app(palette));
    });

    gpui_shell::set_native_modules(modules);
}
```

And this is the script that uses it — the same `Market` entity a Rust panel beside it is rendering from:

```js
import { native } from "gpui";

const market = native("market");
const quotes = market.quotes();
const watched = quotes.filter((quote) => quote.watched).length;
```

Run it with `cargo run -- shell`. The two panels read one entity through two paths, which is what makes a mismatch between them visible immediately.

## Typing them

`gpui-shell types` cannot know what a host registered, so the generated `gpui.d.ts` leaves an empty `NativeModules` interface for the application to augment:

```ts
declare module "gpui" {
  interface NativeModules {
    market: {
      quotes(): Quote[];
      ticks(): number;
      watch(symbol: string): boolean;
      watch_all(on: boolean): number;
    };
  }
}
```

Declaring that in a `.d.ts` beside the script is what turns `native("market")` into a checked call — the module name is verified and its functions complete. Declaring nothing costs nothing: an untyped overload still applies, so an application that never writes one keeps working.

## Not there yet

- **Asynchronous native functions.** A function returns a value, not a promise; long work blocks the thread that renders.
- **Per-module grants.** The registry is one set per host by design. Giving two plugins two different sets is what a `Policy` is for, and the plugin model that hands them out is not documented yet.
- **Streaming or callbacks into the host.** A script cannot hand a function to a native module; the module can only be called.

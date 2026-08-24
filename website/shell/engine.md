---
title: The Engine Seam
description: QuickJS by default, a compiling LuaJIT fallback, why the seam exists, and the measurement that decides between them.
order: 8
---

# The Engine Seam

The scripting engine sits behind one internal interface. Everything above it — the element description arena, the materializer, the call scope, the style table, the theme, the capability model, the overlay host, hot reload — is engine independent, and only the engine module knows what a script value is.

```bash
# The default: QuickJS, via rquickjs.
cargo run -p gpui-shell -- examples/js_todolist

# The fallback, which still compiles and still runs.
cargo run -p gpui-shell --no-default-features --features luajit -- path/to/app
cargo run -p gpui-shell --no-default-features --features lua -- path/to/app
```

Exactly one engine may be enabled. Enabling both is a **compile error**, not a silent fallback to the default, because both export a type of the same name and `gpui_shell::ShellRuntime` would be ambiguous. A wrong feature combination should be reported at build time with the command to fix it.

## Why there is a seam at all

The engine choice is the one decision in this runtime that could not be settled on paper.

Everything else in the design follows from GPUI's element model and can be argued about with a whiteboard. The engine cannot, because the whole approach stands or falls on a single number: **how long it takes script code to describe a realistic interface.** The script sits on the path that rebuilds the element tree every frame, and every method call in a builder chain is one crossing of the language boundary. If that per-call cost is too high, no amount of design fixes it.

So the seam is a way of not having to be right in advance. The decision is made by measurement, and reversing it is a cargo feature rather than a rewrite.

JavaScript is the default for one reason, and it is a product reason rather than a technical one: **application code reads better in it.** With presentation owned by the script, the vast majority of an application is composing elements, writing styles and handling events — and the readability of that code decides whether the runtime is worth using. Classes, arrow functions, template literals and destructuring land squarely on that kind of code. The secondary benefit is that JavaScript is the best-covered language in model training data, which matters for one of the [three audiences](./index.md#who-it-is-for).

The cost is stated rather than glossed over. QuickJS is larger than LuaJIT and **has no JIT** — it is a bytecode interpreter, so hot loops and per-call costs will not beat LuaJIT on principle.

## The measurement

The benchmark describes a 40 × 5 grid of styled cells — 443 description nodes, roughly ten recorded operations each — fifty times, and reports the mean per pass. It is the one test that decides the engine, so it runs on both:

```bash
cargo test -p gpui-shell --release --test benchmark -- --nocapture

cargo test -p gpui-shell --release --no-default-features --features luajit \
    --test benchmark -- --nocapture
```

| Engine | 443 nodes, per description pass |
| --- | --- |
| **QuickJS** | **1.14 ms** |
| LuaJIT | 1.36 ms |

A re-run on different hardware measured 1.09 ms and 1.43 ms — the absolute figure moves with the machine, the ordering did not. Both are release-build numbers; run it in release or the figure means nothing.

The surprise is worth naming: **QuickJS came out ahead**, despite having no JIT and despite LuaJIT being the faster interpreter by a wide margin on hot numeric loops. This workload is not a hot loop. It is thousands of short calls that each cross into Rust and record one operation, and on that shape the two engines are close enough that a JIT never gets to matter.

The design's own budget is worth holding the result against, and the answer is "inside, with less room than hoped". The target is 1.5 ms for a script render during continuous interaction, and a panel of this size clears it on either engine. But that budget was derived from roughly 150 ns per recorded operation across 800 nodes, and the benchmark reports about 250 ns on QuickJS and 320 ns on LuaJIT. A panel three times this size would not fit. The three levers the design names for that case — driving the per-call cost down, memoizing unchanged subtrees, and virtualizing long lists — are exactly the ones that are [not implemented yet](./elements.md#not-there-yet).

Two implementation choices came out of the same measurement and are visible in the runtime today:

- **Elements are plain objects sharing one prototype**, with the style methods installed on that prototype by a JavaScript prelude that loops over the name list. Not one class per element, not a fresh closure per property access, and not 3,000 Rust closures.
- **The diagnostic `Proxy` prototype is not the default.** Wrapping the prototype in a `Proxy` so a mistyped method can be named costs about 30% of the whole description pass, so the runtime keeps a plain prototype and re-runs a failed render once against the diagnostic one purely to produce the message. See [Styling](./styling.md#unknown-methods).

## What is on each side

The proportion is itself the argument for the seam: above it is the actual design, below it is "what does a script value look like".

| Above the seam — shared by both engines | Below the seam — written once per engine |
| --- | --- |
| The element description arena, single-use checking, and the debug tree | Converting an engine value to the runtime's neutral value type |
| Materialization: descriptions into real GPUI elements, pure Rust | The module system's shape — ES modules and a resolver, versus `require` and a path list |
| The call scope: phases, generations, and the crate's only `unsafe` | Method dispatch — functions on a shared prototype, versus an `__index` metamethod |
| The style table, parametric styles and spelling suggestions | The callback handle type |
| The default token palette and colour token resolution | Converting the neutral error type into the language's own exception |
| The capability model and path resolution | How a view is defined — `class extends View`, versus a metatable |
| Length and colour coercion | The language-specific part of the sandbox |
| The neutral error type, the callback arena, the error overlay | |
| `ScriptView`, `ShellRoot`, hot reload | |

None of the modules on the left names a VM anywhere in its source. That is what makes the seam real: it is not a trait, it is the fact that the rest of the crate reaches the engine through about a dozen entry points and nothing else.

A trait would actually be worse here. The two handle types — a view class and a view instance — carry lifetimes of their own on the QuickJS side, and forcing them through a trait would move that complexity into the type system without removing any of it.

## Portability

**Scripts are not portable between the engines.** They are different languages: a view is `class Counter extends View` in JavaScript and a metatable in Lua.

What *is* the same on both is everything else — the binding surface, the render protocol, the phase rules, the capability model, the error messages. The requirement the design imposes is behavioural: the same use case must produce the **same description tree** under either engine. That is what keeps the seam from rotting into two divergent runtimes.

## Known gap: async is not fully behind the seam

The seam's contract does not yet cover asynchronous work, and that is a real hole rather than an oversight.

Lua's coroutines and JavaScript's promises are not the same mechanism, and QuickJS additionally requires the host to drain its job queue itself — nothing after an `await` runs until somebody asks. So the scheduler cannot sit entirely above the seam. It needs two operations from each engine: turning a host task into something the script can await, and running the pending jobs, which is a no-op on the Lua side.

Until those are added to the contract, the scheduler is QuickJS-specific. The rule it will be held to is the one that applies to any new capability: it goes above the seam unless it genuinely cannot be expressed there, and if it must live below, **both engines implement it or the missing side throws a clear error**. One engine having a feature and the other silently doing nothing is how a seam like this decays.

## Why not WebAssembly, or a separate process

Two questions the seam invites.

`gpui-shell` runs the VM **in the host process, on the main thread**, alongside GPUI's `App`. That is not a shortcut — it is what makes a per-call cost of a few hundred nanoseconds possible at all. A separate process would put an IPC round trip on the path that rebuilds the element tree, and there is no budget for one. For the same reason there is no `Worker`: the VM and the `App` are both main-thread only.

The wasm target is the other reason the seam is drawn where it is. QuickJS is plain C and compiles to WebAssembly; LuaJIT does not. LuaJIT also generates machine code, which is a constraint on platforms that forbid writable-executable memory. Neither fact decides today's default, but they are why "the engine is a parameter, not a part of the architecture" is written down rather than assumed.

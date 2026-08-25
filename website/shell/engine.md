---
title: The Engine Seam
description: QuickJS behind one internal interface, why the seam exists, and the three measurements that tell script cost apart from frame cost.
order: 8
---

# The Engine Seam

The scripting engine sits behind one internal interface. Everything above it — the element description arena, the materializer, the call scope, the style table, the theme, the capability model, the overlay host, hot-reload — is engine independent, and only the engine module knows what a script value is.

```bash
cargo run -p gpui-shell -- examples/js_todolist
```

[QuickJS](https://github.com/quickjs-ng/quickjs), via [`rquickjs`](https://github.com/DelSkayn/rquickjs) — which vendors the `quickjs-ng` fork — is the engine that ships and the only one today. It sits behind a `quickjs` cargo feature all the same, and building with no engine is a **compile error** rather than a crate that exports nothing.

## Why there is a seam at all

The engine choice is the one decision in this runtime that could not be settled on paper.

Everything else in the design follows from GPUI's element model and can be argued about with a whiteboard. The engine cannot, because the whole approach stands or falls on a single number: **how long it takes script code to describe a realistic interface.** Every method call in a builder chain is one crossing of the language boundary, and if that per-call cost is too high, no amount of design fixes it.

What the number is *compared against* changed once a script `render` stopped being a frame render. A description is built when application state moves and [replayed by every frame until it moves again](./state.md#when-render-runs), so the cost below is paid per user action rather than per repaint. That makes the boundary cost matter less than it did — but it does not make it free, and it is still the number that would decide a second engine.

So the seam is a way of not having to be right in advance. The decision is made by measurement, and a second engine would be a new module rather than a rewrite.

JavaScript is the default for one reason, and it is a product reason rather than a technical one: **application code reads better in it.** With presentation owned by the script, the vast majority of an application is composing elements, writing styles and handling events — and the readability of that code decides whether the runtime is worth using. Classes, arrow functions, template literals and destructuring land squarely on that kind of code. The secondary benefit is that JavaScript is the best-covered language in model training data, which matters for one of the [three settings](./index.md#where-it-fits).

The cost is stated rather than glossed over. QuickJS **has no JIT** — it is a bytecode interpreter, so hot loops and per-call costs will not beat a JIT-compiled engine on principle. That is a real trade, and the benchmark below is where it would show up if it mattered.

## The measurement

There are three costs here, and treating them as one was the original mistake. The benchmark describes a 40 × 5 grid of styled cells — 443 description nodes, roughly ten recorded operations each — and reports each cost separately:

```bash
cargo test -p gpui-shell --release --test benchmark -- --nocapture
```

| | What it measures | 443 nodes | Paid |
| --- | --- | --- | --- |
| **A** | script → snapshot | **1.4 ms** | once per application change |
| **B** | snapshot → GPUI elements | **0.7 ms** | every frame |
| **C** | a full cached repaint | **1.8 ms**, **no JavaScript at all** | every frame |

Run it in release or the figures mean nothing. Every absolute number on this page comes from a release build on a MacBook Pro (M3, 8 cores, 24 GB), and moves with the machine.

**C is the one that is an assertion rather than a timing.** Fifty repaints of an unchanged view run no JavaScript at all. If a single one of them ever does, the runtime has regressed to charging script cost per frame, and the benchmark fails rather than merely getting slower.

One size cannot show which of the three costs scale, so a fourth test walks the same panel up to 8,403 nodes. It sits behind `--ignored` because the largest size takes seconds:

```bash
cargo test -p gpui-shell --release --test benchmark -- --ignored --nocapture
```

Describing costs 1.1 ms at 443 nodes, then 5.1, 10.3 and 20.5 ms as the panel grows to 2,103, 4,203 and 8,403. A whole frame — B plus GPUI's layout and paint, which is what C measures — costs 1.3, 5.9, 12.0 and 27.0 ms. Both scale close to linearly with the node count. What does not scale is the JavaScript: no frame at any size runs a line of it. Three things that settles:

- **4,203 nodes is where the snapshot decides the outcome.** 12 ms a frame holds 60 FPS; rebuilding the description for every frame would cost 22 ms and drop them. Below that size both models have room to spare, which is worth knowing before reading too much into the ratio.
- **The description cost did not vanish, it moved.** 20 ms for 8,403 nodes is paid when the user acts rather than sixty times a second, but it is still 20 ms — which is why the per-call cost remains the number a second engine would be judged on.
- **Past a few thousand nodes the bill is not script at all.** 27 ms a frame at that size, with the VM untouched, is materialization, layout and paint. A view that large wants virtualizing; a faster engine would not move it.

Read A against the design's own budget — 1.5 ms for one script `render` — and it clears it, but with less room than hoped: the budget was derived from roughly 150 ns per recorded operation across 800 nodes, and the measurement reports about 320 ns across 443. A panel three times this size would not fit in one pass. What changed is how often that matters. At 120 FPS the old model would have spent 168 ms of every second describing an interface nobody had changed; the same panel now costs 1.4 ms when the user actually changes something, and 0.7 ms to repaint. The levers the design names for genuinely enormous panels — driving the per-call cost down, memoizing unchanged subtrees, virtualizing long lists — are still [not implemented](./elements.md#not-there-yet), and are now optimizations rather than prerequisites.

Two implementation choices came out of the same measurement and are visible in the runtime today:

- **Elements are plain objects sharing one prototype**, with the style methods installed on that prototype by a JavaScript prelude that loops over the name list. Not one class per element, not a fresh closure per property access, and not 3,000 Rust closures.
- **The diagnostic `Proxy` prototype is not the default.** Wrapping the prototype in a `Proxy` so a mistyped method can be named costs about 30% of the whole description pass, so the runtime keeps a plain prototype and re-runs a failed render once against the diagnostic one purely to produce the message. See [Styling](./styling.md#unknown-methods).

## Threads and memory

The VM and GPUI's `App` share one thread — the main one — inside one process. `ShellRuntime` is an `Rc` with `RefCell` interiors, so it is neither `Send` nor `Sync`. There is no worker and no second VM.

<img class="architecture-light" src="/shell-threads-memory-light.svg" alt="The host process. On the main thread, GPUI's App and the QuickJS VM exchange plain function calls across the FFI boundary. A background thread holds only timers, which resolve back onto the foreground executor. Memory splits four ways: the JavaScript heap capped at 256 MiB, the description arena owned by the snapshot, the callback arena keyed by snapshot generation, and GPUI's frame arena which lasts one draw.">
<img class="architecture-dark" src="/shell-threads-memory-dark.svg" alt="The host process. On the main thread, GPUI's App and the QuickJS VM exchange plain function calls across the FFI boundary. A background thread holds only timers, which resolve back onto the foreground executor. Memory splits four ways: the JavaScript heap capped at 256 MiB, the description arena owned by the snapshot, the callback arena keyed by snapshot generation, and GPUI's frame arena which lasts one draw.">

Two things do run elsewhere, and neither touches the VM. Timers (`gpui.sleep`, `gpui.timer`) count down on the background executor and resolve back onto the foreground one, so the continuation itself runs on the main thread in a `Task` scope. And GPUI does its own work on its own threads once the elements exist.

Three consequences matter when profiling:

- **A builder call is a function call.** It crosses the FFI boundary and nothing else — no serialization, no IPC round trip, no copy beyond the conversion of the argument itself. The benchmark reports that cost per recorded operation, and across the four panel sizes it lands at **240–340 ns**.
- **Blocking the VM blocks the frame.** The synchronous `fs` surface is the sharp edge here: a read from an event handler stalls the same thread that is about to paint.
- **A runaway script cannot be preempted from another thread.** What cuts it off is the interpreter's own interrupt — 50 ms inside `render`, 500 ms inside an event handler — and a `catch` block cannot swallow it.

Memory splits four ways, each with a different owner and a different moment of release:

| What | Where it lives | Released when |
| --- | --- | --- |
| Objects, closures, module scope | The QuickJS heap, capped at 256 MiB | Its GC runs, or the runtime drops |
| The element description arena | Rust; moved into the snapshot it produced | That snapshot drops |
| Registered callbacks | A Rust arena keyed by snapshot generation | That snapshot drops and retires its generation |
| GPUI elements | GPUI's own frame arena | The draw that built them ends |

A view holds **two** snapshots rather than one: the live description, and the one it replaced. The previous is kept a generation longer because a frame already in flight may still be reading it, and releasing it early would retire callbacks that frame still needs.

Nothing that crosses the boundary is an object. An element handle is an integer index into the arena, retained host state — an `InputState`'s rope, cursor and selection — lives in a GPUI entity the script addresses through a handle, and every argument and result is plain data.

## What is on each side

The proportion is itself the argument for the seam: above it is the actual design, below it is "what does a script value look like".

| Above the seam — engine independent | Below the seam — what an engine implements |
| --- | --- |
| The render snapshot: what one script `render` produces and what frames replay | Converting an engine value to the runtime's neutral value type |
| The element description arena, single-use checking, and the debug tree | The module system's shape — ES modules and a resolver, versus `require` and a path list |
| Materialization: descriptions into real GPUI elements, pure Rust | Method dispatch — functions on a shared prototype, versus an `__index` metamethod |
| The call scope: phases, generations, and the crate's only `unsafe` | The callback handle type |
| The style table, parametric styles and spelling suggestions | Converting the neutral error type into the language's own exception |
| The default token palette and colour token resolution | How a view is defined — `class extends View`, versus a metatable |
| The capability model and path resolution | The language-specific part of the sandbox |
| Length and colour coercion | |
| The neutral error type, the callback arena, the error overlay | |
| `ScriptView`, `ShellRoot`, hot-reload | |

None of the modules on the left names a VM anywhere in its source. That is what makes the seam real: it is not a trait, it is the fact that the rest of the crate reaches the engine through about a dozen entry points and nothing else.

A trait would actually be worse here. The two handle types — a view class and a view instance — carry lifetimes of their own on the QuickJS side, and forcing them through a trait would move that complexity into the type system without removing any of it.

The contract's load-bearing rule is about *when*, not what: **the engine's `build_snapshot` is the only entry into script `render`, and nothing calls it per frame.** An engine that rendered opportunistically — on a repaint, on a hover, on a timer — would put script cost back on the frame budget, which is the coupling the seam exists to prevent. Benchmark C is what would catch it.

## Portability

If a second engine is ever added, **scripts will not be portable between them.** They would be different languages: a view is `class Counter extends View` in JavaScript and would be something else anywhere else.

What has to be the same is everything around them — the binding surface, the render protocol, the phase rules, the capability model, the error messages. The requirement the design imposes is behavioural: the same use case must produce the **same description tree** under either engine, and the same application activity must trigger the **same number of script `render` calls**. That is what would keep the seam from rotting into two divergent runtimes.

## Known gap: async is not fully behind the seam

The seam's contract does not yet cover asynchronous work.

QuickJS requires the host to drain its job queue itself — nothing after an `await` runs until somebody asks — and that is not a shape every engine shares. So the scheduler cannot sit entirely above the seam. It needs two operations from an engine: turning a host task into something the script can await, and running the pending jobs.

There is a second, sharper reason to finish this. Draining the job queue currently happens at the end of a snapshot build, which means arbitrary application code — anything after an `await` — runs on the path a render took. Snapshot caching makes that rare rather than per-frame, but the coupling is still there and belongs on the event loop instead.

Until both are addressed, the scheduler is QuickJS-specific. The rule it will be held to is the one that applies to any new capability: it goes above the seam unless it genuinely cannot be expressed there.

## Why not WebAssembly, or a separate process

Two questions the seam invites.

`gpui-shell` runs the VM **in the host process, on the main thread**, alongside GPUI's `App`. That is what makes the 240–340 ns per recorded call possible at all. A separate process would put an IPC round trip on every recorded builder call, and there is no budget for one even at the reduced frequency snapshots buy. For the same reason there is no `Worker`: the VM and the `App` are both main-thread only.

The wasm target is the other reason the seam is drawn where it is. QuickJS is plain C and compiles to WebAssembly; not every candidate engine does, and some generate machine code, which is a constraint on platforms that forbid writable-executable memory. Neither fact decides today's engine, but they are why "the engine is a parameter, not a part of the architecture" is written down at all.

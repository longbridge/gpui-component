# GPUI Shell Architecture

## Status and Scope

This document describes the architecture implemented by `crates/shell`. It is a
source-derived reference, not a proposal. The public exports in
`crates/shell/src/lib.rs`, the Rust API documentation, and the generated
`gpui.d.ts` remain authoritative for individual methods.

`gpui-shell` is a scriptable application runtime built on `gpui-base`. The host
owns rendering, layout, text editing, focus, overlays, and system access; the
script owns composition, presentation, and business logic. JavaScript is the
default scripting language, running on QuickJS.

Two documents come before this one: [Architecture](ARCHITECTURE.md) for the
`gpui-base` seam this runtime sits on, and [Styling and
Motion](STYLING-AND-MOTION.md) for the token model it exposes to scripts.

The runtime is real and runnable — `cargo run -p gpui-shell -- examples/js_todolist`
opens a working application — but it is not complete. §26 states plainly what
exists and what does not.

The crate is under active development, so §26 in particular is a snapshot and
will need re-checking against the source. At the time of writing, `plugin.rs`
holds a complete manifest and plugin-manager implementation that `lib.rs` does
not yet declare as a module, and `dock.rs` is complete on the Rust side with no
engine binding above it — both are noted where they appear below.

---

## 1. Overview

GPUI Shell gives an application layer written in JavaScript the same standing
that a Rust application built directly on `gpui-base` has: it composes base
behavior and owns every visual decision. Rust supplies the element model,
layout, text input, the overlay stack, the theme tokens, and the capability
gate. The script supplies the tree and the style.

Four things make up the runtime:

1. an embeddable script runtime — VM, scheduler, error recovery, hot reload —
   with the VM behind an explicit seam (§6.5);
2. bindings over the `gpui` element and style layer and the `gpui-base`
   behavior layer;
3. a capability-gated system API (`fs`, `store`, `clipboard`, `log`,
   `process`);
4. a command-line host that runs an application directory, checks it, and
   generates its type declarations (§23).

The plugin model — manifests, contribution points, distribution — is not built.
The capability type it would populate is (§18).

The goal is one sentence: build an application layer at the iteration speed of
a scripting language while keeping Rust and the GPU on the render path.

---

## 2. Why It Exists

### 2.1 Three costs in this repository

**Compile cost.** Every application-layer adjustment goes through
`cargo build`. This workspace depends on the Zed git `gpui`, tree-sitter,
syntect, and reqwest; a cold build takes minutes and even an incremental one
pays for a link. For "change a gap, change a color, add a filter," the compile
is longer than the thought behind the change.

**Extension cost.** `crates/base/src/dock` already has half of what a plugin
system needs: `PanelRegistry` rebuilds a panel from a `panel_name` string,
`PanelInfo::panel(serde_json::Value)` lets a panel persist private state, and an
unregistered panel is carried by a placeholder so a layout round-trips intact.
The missing half is that a panel's implementation has to be compiled into the
host binary. Nobody can contribute a panel without forking it.

**Generation cost.** Generating Rust UI requires correct types, correct
borrows, and a successful compile; the feedback loop is the compiler.
Generating a script interface executes immediately, draws immediately, and on
failure throws a recoverable exception while the host process survives.

JavaScript sharpens the third point and blunts nothing about the first two. It
is the best-covered language in public training data, and its type declarations
(§14.4) are a format both editors and models already read.

The same coverage is also a liability: a model writing for this runtime will
reach for `document`, `window`, `fetch`, `require("fs")`, npm packages, and
`setTimeout`. None of them exist here. §19.1 answers that with named stubs that
throw and point at the replacement, rather than with a bare `ReferenceError`.

### 2.2 Who it is for

| Audience | Situation | What it needs |
| --- | --- | --- |
| Plugin authors | Adding a panel, command, or side tool to an existing Rust application | Stable contribution points, a sandbox, dock persistence |
| Internal tool authors | Dashboards, ops panels, data viewers, one-off tools | Low start-up cost, complete system API, packaging |
| Generated interfaces | A model writing the interface and its interactions | Common syntax, recoverable errors, hot reload, a typed API contract |

None of these is "rewrite the product core in a script." That distinction
decides nearly every trade-off below.

### 2.3 What the reference projects do and do not prove

VS Code shows that JavaScript extensions over a single host namespace, with
declared capabilities and contribution points, can carry a very large ecosystem.
Figma shows that QuickJS works as a restricted UI plugin VM in production.
Neovim shows the general shape: the host provides capability, the script
provides extension, and `vim.api` is a stable contract.

What none of them proves is the one thing this runtime depends on: their
scripts are not on the path that rebuilds an element tree. This one is (§20).

---

## 3. Scope and Non-Goals

The runtime binds the `gpui` element and style layer, part of the `gpui-base`
behavior layer, the semantic theme tokens, the window-level overlay stack, and a
capability-gated system API. The script holds complete presentation authority:
style, color, spacing, and state styles are all expressed in script.

Seven things are deliberately absent, and will stay absent:

1. **Rust is not replaced for the product core.** The text editing engine,
   syntax highlighting, LSP, virtualization, and animation stay in Rust.
2. **There is no UI DSL, markup, or JSX.** Interfaces come from ordinary
   functions and builder chains (§5.3). JSX needs a compile step, and "edit a
   line, save, see it" is why this runtime exists.
3. **Script never enters the layout or paint path.** Layout, painting, hit
   testing, scrolling, and IME are entirely in Rust (§8.4).
4. **There is no multi-threaded script.** The VM and GPUI's `App` are both
   main-thread only (§12.4). There is no `Worker`.
5. **There is no dynamic native plugin loading.** Rust has no stable ABI, and
   `dlopen`ed native code inside the process defeats the sandbox outright.
6. **`gpui-base` is not modified.** Everything lives in `crates/shell`.
7. **There is no Node.js or browser compatibility layer.** No `document`, no
   `window`, no `fetch`, no `require`, no `process` module, no npm. A partial
   compatibility layer would pull the whole npm ecosystem in and then shatter on
   the first native dependency. Host capability goes through one namespace,
   `gpui` (§17).

---

## 4. Relation to the Existing Architecture

### 4.1 Layering

```text
     JS application            main.js · views · styles · business logic
              │  import ... from "gpui"
              ▼
     crates/shell ── gpui-shell
     ┌──────────────────────────────────────────────┐
     │ engine/ seam: QuickJS (default) | Lua        │
     ├──────────────────────────────────────────────┤
     │ CallScope · SpecArena · style reflection     │
     │ materialize · theme tokens · capabilities    │
     │ ShellRoot · entities · typings · watch       │
     └──────────────────────────────────────────────┘
              │
              ▼
     gpui-base              behavior · state · infrastructure (unstyled)
              │
              ▼
     gpui / gpui_platform   elements · style · rendering · GPU · platform
```

Against the dependency diagram in [ARCHITECTURE.md](ARCHITECTURE.md),
`crates/shell` plus a script application occupies the **application-owned UI**
branch: parallel to `gpui-component`, not downstream of it. The seam is one thin
line in that picture, and everything above it is language-independent (§6.5).

### 4.2 Why it binds `gpui-base` and `gpui`

**Presentation authority goes to the script, which is the whole point.** Binding
`gpui-component` would leave a script calling visuals somebody else already
decided; changing a button's corner radius would still mean going back to Rust.
Binding base puts style, state style, spacing, and color entirely in script.

**Layer neutrality.** The shell depends on no product visual system, so any
host can embed it, including one with its own design system. The moment the
shell depended on `gpui-component`, it would impose one set of visuals on every
embedder.

**A binding surface an order of magnitude smaller.** `gpui_base::Button` has 13
public functions against `gpui_component::Button`'s 52; base has 18 direct
dependencies against 31. Base's interfaces are narrower and more stable
precisely because they carry no visuals, which is what makes complete coverage
possible at all — and a binding layer that covers only part of its target is the
hardest kind to use.

**Build size and reach.** The runtime's own iteration speed, binary size, and
WebAssembly viability all benefit from a smaller dependency tree. QuickJS is
already larger than LuaJIT; every dependency saved elsewhere is worth having.

**A working precedent.** `crates/base/examples/showcase` is a base-only
application: it implements the dock renderer traits itself, supplies its own
`InputEditorStyle` and colors, wires syntect highlighting, and builds for
WebAssembly. `examples/js_todolist` is that same posture with the composition
and styling written in JavaScript instead of Rust, and `ui.js` deliberately
follows the showcase's visual language.

### 4.3 What base-first makes the shell carry

Four costs follow from binding base rather than a styled component library.
They are real, and all four are paid in `crates/shell`.

**The default color tokens are transparent, so the shell ships a palette.**
`gpui_base::Theme`'s `ColorTokens` derives `Default`, meaning every color starts
as `Hsla { h: 0, s: 0, l: 0, a: 0 }` — fully transparent. `RadiusTokens` and
`SpacingTokens` have real defaults; colors do not. A runtime that only called
`gpui_base::init` would paint an invisible window. `theme.rs` embeds
`theme/default-tokens.json` with `include_str!` and installs it, so a shell
binary is self-contained and cannot start unstyled because a file is missing
(§13.3).

**There is no `Root`, so the shell provides `ShellRoot`.** `Root` lives in
`crates/ui` and belongs to `gpui-component`. Base ships the parts — `Dialog` and
`Sheet` each build their own viewport-sized host, `ToastManager` and
`ToastStackState` own stacking geometry, `FocusTrapElement` owns focus trapping
— but nothing in base decides what happens when two of them are open at once.
`root.rs` is that decision (§16).

**There is no Icon, TitleBar, or Notification component.** `Icon`, `IconName`,
`TitleBar`, and `window_border` are all in `crates/ui`. Scripts load icons with
`svg(path)`, resolved against the application directory by `assets.rs`, and draw
their own chrome.

**The dock draws nothing.** A `DockArea` built without a renderer docks, drags,
and persists, but paints no chrome at all. Supplying those renderers is the work
described in §15, and it is not done.

### 4.4 Constraints on existing crates

`crates/base` and `crates/ui` are unchanged; `crates/shell` depends on
`gpui-base` (with its `inspector` feature) and `gpui`, and on neither
`gpui-component` nor `crates/ui`. Consumers who do not add `crates/shell` see no
change to their build output or dependency tree.

`crates/shell` enables `gpui-base/inspector` unconditionally, which forwards to
`gpui/inspector`. That is not optional: the style reflection tables are behind
`#[cfg(any(feature = "inspector", debug_assertions))]`, so without the feature a
release build would expose an empty style surface (§13.1).

---

## 5. Design Principles

**5.1 The host provides capability; the script composes and presents.** A
script can do exactly what the host registered, no more. Adding capability is an
explicit host action — which is also why quickjs-libc's `std` and `os` modules
are never registered and there is no Node compatibility layer.

**5.2 Elements are values, not objects.** `Button.new("id")` returns an element
*description* that expires when the render pass ends. This is a direct
consequence of GPUI's element model (§8.1), not a stylistic choice.

**5.3 No DSL, no JSX.** Interfaces are built with builder chains that
correspond one-to-one with the Rust API, so learning one teaches the other. A
DSL would need its own parser, diagnostics, editor support, and version
evolution. JSX would need a compile step, and "edit a line, save, see it" is the
reason this runtime exists. This matches the GPUI builder style in
`CLAUDE.md`: keep one fluent chain and express conditions with `when`.

**5.4 A context is valid only for the duration of a call.** `&mut App`,
`&mut Window`, and `&mut Context<T>` are borrows. `CallScope` turns "am I inside
a legal host call?" into a runtime-checkable fact, so an out-of-scope access
throws a script exception rather than reading a dead stack frame (§9).

**5.5 Binding tables are data.** The no-argument style surface comes from
GPUI's reflection tables with no hand-written names, and `gpui.d.ts` is
generated from the same tables the dispatcher uses (§14.4). The failure mode of
hand-written bindings is not that they are tedious; it is that upstream changes
a signature and the binding does not follow.

**5.6 Presentation belongs to the script and must be replaceable.** The Rust
side installs no visual decision beyond the color tokens, which exist only in
overridable form. Anything more would amount to a third, uncontrolled visual
system on top of base.

**5.7 No capability by default.** `Capabilities::default()` is the empty set,
and every field is private with a builder (`capability.rs`). "No capability by
default" is therefore a fact about the type, not a promise in prose (§19.2).

**5.8 Failure is recoverable.** Every script error collapses into one exception
carrying the script's own stack: it is logged, it is shown as a failure surface
where the interface should have been, the rest of the host keeps working, and no
Rust panic crosses the boundary (§21.1).

**5.9 The engine is a parameter, not part of the architecture.** Anything that
can live above the seam lives above the seam, and anything that lives inside an
engine has to justify why it could not (§6.5).

---

## 6. Runtime Overview

### 6.1 Modules

| Module | Responsibility | Side of the seam |
| --- | --- | --- |
| `engine/` | VM lifecycle, module loading, method dispatch, callbacks, exception conversion | below |
| `engine/quickjs/` | The default engine: prelude, host API, scheduler, overlays, entity API, native bridge, theme API, sandbox | below |
| `engine/lua.rs` | The fallback engine (mlua) | below |
| `scope` | `CallScope`: the host-context stack, its phases, and generation checks | above |
| `spec` | `SpecArena`: single-pass element descriptions, single-use checks, `debug_tree` | above |
| `materialize` | Replays descriptions into real GPUI elements; pure Rust | above |
| `style` | The reflected style table, the parametric bindings, spelling suggestions | above |
| `theme` | The default semantic palette and token-name resolution | above |
| `value` | `Bridged`: the neutral script argument value, and color and length coercion | above |
| `error` | `ShellError`: the neutral error type | above |
| `entities` | Retained state addressed by handle, and its subscriptions | above |
| `capability` | The capability set, the path resolver, and denial messages | above |
| `runtime` | `CallbackArena<T>`, application-root resolution, the failure surface | above |
| `root` | `ShellRoot`: the window-level overlay stack | above |
| `dock` | `ScriptPanel`, `ScriptDockSkin`, panel registration and name interning | above |
| `native` | The host-registered native module registry | above |
| `plugin_api` | The script API version and its compatibility check | above |
| `view` | `ScriptView`: the one bridge into GPUI's render loop | above |
| `assets` | Application-directory asset source for `svg(path)` | above |
| `watch` | Source watching and in-place reload | above |
| `typings` | `gpui.d.ts` generation | above |

The ratio is the argument. Above the seam are the element model, styling,
theming, capabilities, and context safety — the actual design. Below it is what
a script value looks like.

### 6.2 Key Rust types

`ScriptView` is the only way script output reaches GPUI. Every script-defined
view, dialog body, and sheet body is carried by one:

```rust,ignore
pub struct ScriptView {
    runtime: Rc<ShellRuntime>,
    object: ViewObject,
}

impl Render for ScriptView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = self.runtime.clone();
        let object = self.object.clone();
        let entity = cx.entity();
        runtime.render_view(object, entity, window, cx)
    }
}
```

`ScriptView` only carries `ViewObject`. Under QuickJS that is a
`Persistent<Object>`; under Lua it is an `mlua::Table`. `ScriptView` needs to
know neither. It also exposes `replace_object`, which is what makes a hot reload
keep the window, the focus, and the element identities while swapping only what
the script produced (§21.2).

`gpui_shell::init(cx)` must run once at startup. It calls `gpui_base::init`,
installs the default palette, and builds the style reflection table so the first
script call does not pay for it.

### 6.3 The engine choice

QuickJS via `rquickjs` 0.12 is the default, with the `macro`, `loader`,
`classes`, and `properties` features. An mlua 0.11 engine lives behind the `lua`
and `luajit` features as a compilable, runnable fallback.

| | QuickJS (default) | LuaJIT | Lua 5.4 |
| --- | --- | --- | --- |
| Language | ES2023: classes, modules, `async`/`await`, Proxy, destructuring | Lua 5.1 semantics plus extensions | Lua 5.4 |
| Execution | Bytecode interpreter, **no JIT** | Hand-written assembly interpreter plus trace JIT | Bytecode interpreter |
| Hot loops | Weakest of the three | Strongest, by an order of magnitude | Comparable to QuickJS |
| WebAssembly | Yes (pure C) | **No** | Yes |
| W^X platforms | Unaffected — emits no machine code | Restricted (Apple Silicon and similar) | Unaffected |
| Size | Largest: full ES semantics, regex, Unicode | Small | Smallest |
| GC | Reference counting plus cycle collection | Incremental mark-and-sweep | Generational/incremental mark-and-sweep |
| Corpus and tooling | Best | Fair | Fair |

JavaScript is the default for one reason, and it is a product reason rather than
a technical one: application code reads better in it, and the model corpus is
the best there is. The costs are real and are not offset. QuickJS has no JIT, so
neither hot loops nor per-call boundary cost will ever beat LuaJIT, and it is the
largest of the three.

Two properties of QuickJS turned out to matter more than the table suggests.
Reference counting means a host handle — `Persistent<Function>`,
`Persistent<Object>` — is released the moment its last reference goes away,
which removes a layer of uncertainty from the cross-GC cycle problem in §7.4;
true cycles still wait for the collector. And because it emits no machine code
it has no W^X problem, which LuaJIT does have on Apple Silicon.

The measurement is in §20.3. On the M0 benchmark QuickJS came out *ahead* of
LuaJIT for describing an interface, which is not what the table predicts and is
worth understanding before reading anything else into it: describing an element
tree is almost entirely cross-boundary calls and argument conversion, not the
arithmetic a trace JIT is good at.

The constraint the seam imposes is that the script API stays inside the
semantic intersection of both engines. That does not mean the two languages look
alike — a view is `class extends View` in JavaScript and `gpui.view("Counter")`
plus methods in Lua — it means the same use case produces the same description
tree.

### 6.4 The JavaScript surface

One import, one module. Components are type tables with a single `.new`:

```js
import { View, v_flex, text, Button } from "gpui";

export default class Counter extends View {
  init(props = {}) {
    this.count = props.start ?? 0;
  }

  render() {
    return v_flex()
      .gap(12)
      .child(text(`${this.count}`))
      .child(
        Button.new("increment")
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(text("Increment")),
      );
  }
}
```

`"gpui"` is the only built-in module name; every other `import` resolves inside
the application directory (§19.1). The entry point is `main.js`, and it must
`export default` a class extending `View`. The host takes that class, constructs
one instance, and mounts it as the window's root view.

Naming follows the Rust side directly:

| Rust | JavaScript | Example |
| --- | --- | --- |
| Type plus `::new` | Capitalized type table with only `.new` | `Button::new(id)` → `Button.new(id)` |
| Free function | Lowercase function | `div()`, `h_flex()`, `v_flex()`, `text(s)` |
| State entity | Capitalized type table | `InputState::new(...)` → `InputState.new({...})` |
| System capability | Lowercase module member | `fs`, `store`, `clipboard`, `log`, `process` |
| Scheduling | Lowercase module member | `spawn`, `timer`, `sleep`, `with_cx` |
| View base class | `class X extends View` | `export default class Counter extends View` |

#### Style and behavior methods keep their Rust snake_case spelling

`items_center`, `size_full`, `gap_2`, `text_3xl`, `on_click`, and `border_color`
are spelled exactly that way in JavaScript. There are no camelCase aliases, and
that is a deliberate break with JavaScript convention for three reasons.

These names are not hand-written. The whole no-argument style table comes from
GPUI reflection (§13.1); when upstream adds a method, the script surface gets it
for free. Adding camelCase aliases would convert a zero-maintenance table into a
maintained one.

Mechanical conversion is also not well defined over this particular set.
`items_center` → `itemsCenter` is obvious; `gap_2` → `gap2` or `gapTwo`?
`text_3xl` → `text3xl` or `text3Xl`? `rounded_tl` → `roundedTl` or `roundedTL`?
Any single rule produces something awkward across a few dozen names that get
typed every day.

And there should be one spelling for one thing. Two equivalent spellings
immediately split the examples, the type declarations, the documentation, and
the code a model generates.

The cost is honest: a JavaScript author's first `.items_center()` does not look
like JavaScript, and one file then carries two naming conventions. Bound names
are snake_case; anything the author writes — `visible()`, `setFilter`,
`onConfirm` — is camelCase. `examples/js_todolist` reads that way, and in
practice the contrast is useful: a snake_case call is host surface, a camelCase
one is script code.

#### `Button.new(id)`, not `new Button(id)`

The JavaScript habit would be `new Button(id)`. It is not used because the
return value is not an object; it is a description valid for one render pass
(§8.3). `new` implies an instance with identity that can be stored and reused,
which is precisely what an author must not assume here — reusing one throws.
`Button.new(id)` matches Rust exactly and stays neutral about what is being
constructed.

Views, by contrast, use the standard `class extends View`, because a view really
does have identity and cross-frame state and really is owned by GPUI (§7.3). Two
construction shapes in one file, because the two kinds of thing have different
lifetimes.

### 6.5 The engine seam

`crates/shell/src/engine/mod.rs` defines the contract. An engine module exports
one `ShellRuntime` type plus two handle types, `ViewType` and `ViewObject`, that
are opaque to every caller:

```text
ShellRuntime::new() -> anyhow::Result<Rc<Self>>
ShellRuntime::set_global(&Rc<Self>, &mut App)
ShellRuntime::global(&App) -> Option<Rc<Self>>
ShellRuntime::arena_mut(&self) -> RefMut<'_, SpecArena>

ShellRuntime::load_app(&Rc<Self>, &Path, entry: &str) -> anyhow::Result<ViewType>
ShellRuntime::load_source(&Rc<Self>, &str, &str) -> anyhow::Result<ViewType>
ShellRuntime::instantiate(&Rc<Self>, &ViewType, &mut Window, &mut App)
    -> anyhow::Result<ViewObject>

ShellRuntime::render_view(&Rc<Self>, ViewObject, Entity<ScriptView>, &mut Window, &mut App)
    -> AnyElement
ShellRuntime::render_to_spec(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
    &mut Window, &mut App) -> anyhow::Result<String>

ShellRuntime::dispatch_click(&Rc<Self>, CallbackId, &ClickEvent, &mut Window, &mut App)
ShellRuntime::dispatch_change(&Rc<Self>, CallbackId, bool, &mut Window, &mut App)
```

The rest of the crate calls nothing else. That sentence is the definition of the
seam: it is not a trait, it is the fact that the layer above uses only these
entry points. A trait would not work — `ViewType` and `ViewObject` carry their
own lifetimes and `'js` annotations, and forcing them through one would move the
complexity into the type system rather than removing it.

`instantiate` takes a `Window` and an `App` because a view's `init` is where it
creates the state it keeps across frames, and creating a GPUI entity needs both.
Construction therefore opens a scope of its own rather than running in the gap
between host calls.

`load_app` takes the entry file name rather than assuming `main.js`, because a
plugin declares its own entry in its manifest (§18) and the engine is the only
thing that knows the extension a given engine loads.

#### Exactly one engine

```rust,ignore
#[cfg(all(feature = "quickjs", any(feature = "lua", feature = "luajit")))]
compile_error!(
    "enable exactly one scripting engine: `quickjs` (default) or `lua`/`luajit`. ..."
);

#[cfg(not(any(feature = "quickjs", feature = "lua", feature = "luajit")))]
compile_error!("enable one scripting engine: `quickjs` (default) or `lua`/`luajit`");
```

Both engines export the same type names, so enabling both makes
`gpui_shell::ShellRuntime` ambiguous. There is no silent fallback to a default
engine: a wrong feature combination fails at compile time and the message says
how to fix it. The fallback build is:

```bash
cargo run -p gpui-shell --no-default-features --features luajit -- path/to/app
```

#### The two sides

| Above the seam (shared; no VM name appears in the source) | Below the seam (one implementation per engine) |
| --- | --- |
| `spec.rs`: description arena, single-use checks, `debug_tree` | Engine value → `Bridged` conversion |
| `materialize.rs`: descriptions → real elements, pure Rust | Module system (ES modules and a resolver, versus `require` and `package.path`) |
| `scope.rs`: `CallScope`, phases, generation checks, the crate's only `unsafe` | Method dispatch (a shared prototype, versus an `__index` metamethod and a method cache) |
| `style.rs`: reflection table, parametric styles, spelling suggestions | Callback handle type (`Persistent<Function>` versus `mlua::Function`) |
| `theme.rs`: default palette and token resolution | Exception conversion (`ShellError` → `Exception` / `LuaError`) |
| `capability.rs`: capability set and path resolution | View definition shape (`class extends View` versus a metatable) |
| `value.rs`, `error.rs`, `entities.rs`, `runtime.rs`, `root.rs`, `view.rs`, `watch.rs`, `typings.rs`, `assets.rs` | Sandbox specifics: language trimming, intrinsics, promise pumping (§19) |

#### Adding capability

Any new capability goes above the seam unless the language genuinely prevents
it. Three questions in order: does it need to know what a script value looks
like? Can it be expressed with `Bridged`, `SpecOp`, and `ShellError`? If it truly
must live in an engine, then either both engines implement it or the missing side
throws an explicit exception. One engine having a feature while the other
silently does nothing is how this seam rots, and it is the failure the shared
test suite exists to catch (§22.3).

That rule has not held. The QuickJS engine has grown `host.rs`, `scheduler.rs`,
`sandbox.rs`, `overlay.rs`, and `entity_api.rs`; the Lua engine has none of
them, and also lacks `svg`, `Input`, `InputState`, state styles, and
`accessibility_label`. Much of that growth is legitimately engine-specific —
promise pumping and intrinsic trimming have no Lua analogue — but the parts that
are not (the `fs` and `store` surfaces, whose bodies are a capability check plus
one `std::fs` call) should have landed above the seam with only argument
shuffling left in the engine. §25 treats this as the standing risk it is.

Asynchrony is the one gap the original design named and the implementation
closed inside the engine rather than in the contract. Lua coroutines and
JavaScript promises are not the same thing, and QuickJS additionally requires
the host to drain its job queue or nothing after an `await` ever runs (§12.2).
The scheduler therefore lives in `engine/quickjs/scheduler.rs` and does not
appear in the contract above. That is a defensible place for the pumping, and an
indefensible place for the ownership and cancellation model, which is neither
engine-specific nor duplicated anywhere.

---

## 7. Object Model

Every object crossing between script and Rust belongs to exactly one of three
classes.

| Class | Rust side | Script side | Lifetime | Examples |
| --- | --- | --- | --- | --- |
| **Value** | Small `Copy`/`Clone` data | number, string, boolean, plain object | Copied on transfer | `Pixels`, `Hsla`, `ElementId`, enums |
| **Description** | A node id in an arena | A lightweight object over a shared prototype, carrying `__id` | **One render pass** | `div()`, `Button.new(...)` |
| **Entity** | `Entity<T>` behind a handle | A handle object with methods | Across frames, owned by GPUI | `InputState`, `ScriptView` |

### 7.1 Values

`value.rs` owns every coercion, so the rules are defined once. `Bridged` has
four cases — `Nil`, `Bool`, `Number`, `Str` — and everything above the seam sees
only those.

| Script input | Target | Rule |
| --- | --- | --- |
| `12` | `Pixels` | `px(12.)` |
| `"50%"` | `DefiniteLength` | `relative(0.5)` |
| `"12px"`, `"1rem"` | `AbsoluteLength` | Explicit unit |
| `"auto"` | `Length` | `Length::Auto` |
| `"#1e88e5"`, `"#1e88e5cc"`, `"#f00"` | `Hsla` | Hex parsing, three lengths |
| `"accent"` | `Hsla` | Semantic token lookup (§13.3) |

`null` and `undefined` both collapse to `Bridged::Nil`, because at a call site
they mean the same thing: the argument was not given.

An error over an enumerated set names the valid members. The implemented
wording is:

```text
unknown color token `surfacee`; expected one of: background, foreground, surface, … — or a #rrggbb literal
```

That is an order of magnitude more useful than `invalid argument #1`, and it is
the reason the token name list is a real constant rather than something derived
at the call site.

The length grammar is narrowed per method rather than parsed per method: the
three GPUI length types nest (`Length` ⊃ `DefiniteLength` ⊃ `AbsoluteLength`),
so `style.rs` parses once and narrows afterwards, which lets the error say
*which* form was rejected. `.p("auto")` reports that padding needs a definite
length; `.rounded("50%")` reports that radius needs an absolute one.

`line_height` is the single exception in the grammar: a bare number is a
multiplier, not pixels, because `line_height(1.45)` means 1.45× the font size
everywhere else in the industry and 1.45px is never what anyone meant. A string
still goes through the ordinary grammar.

### 7.2 Descriptions

See §8. The constraint is that a description expires when the pass that built it
ends, and reusing one is an error rather than a surprise.

### 7.3 Entities

Retained state lives in `entities.rs`, and a script holds a **handle**, not an
entity reference. The store is a thread-local slot vector; a released slot is
reused before the vector grows, so an application that opens and closes many
inputs does not leak handle space.

```js
const state = InputState.new({ placeholder: "Search" });
state.set_value("hello");
state.value();
state.on("submit", (event, cx) => { /* ... */ });
state.release();
```

The rules:

1. A handle that no longer resolves throws — `this input state has been
   released` — rather than returning `undefined`. In JavaScript an `undefined`
   travels a long way before it fails, and where it finally fails says nothing
   about where it came from.
2. Creating an entity needs a live host call and is refused during `render` and
   `layout`: `InputState.new(...) cannot run during render; create state in
   init() or in an event handler and keep it on the view`.
3. **Subscriptions are owned by the store, not returned to the script.** A
   dropped GPUI `Subscription` stops delivering, and a script has nowhere
   sensible to keep one, so a handler that silently stopped firing would be
   nearly undiagnosable. The store holds them for the lifetime of the handle.
4. Releasing a handle does not release the Rust entity; GPUI still owns it.

`entities.rs` also installs the editor style when it creates an input, for the
same reason `theme.rs` exists: `InputEditorStyle::default()` is entirely
transparent, so an input built without one renders invisible text. The shell
owns the default palette, so it owns this too.

The event names a script can subscribe to are `change`, `submit`, `focus`, and
`blur` — named for what they mean rather than for the key that produced them, so
`submit` covers base's `InputEvent::PressEnter`. An unknown name reports the
full valid set.

### 7.4 Cycles across two garbage collectors

The classic embedded-script leak: Rust holds a script closure, the closure
captures a handle to a Rust entity, and neither collector can see the other's
edge.

Per-frame callbacks (`on_click`, `on_change`) live in `CallbackArena`, which is
replaced wholesale on the next render, so they never form a long-lived cycle.
`CallbackId` encodes the generation in its high 16 bits, and the previous
generation is kept one pass longer because an event can be dispatched between a
render and its paint.

Long-lived callbacks — entity subscriptions, timers, task continuations — are
bound to an owner. A timer or spawned task holds a `WeakEntity<ScriptView>`, so
when the view goes away the callback is skipped rather than writing into state
nothing will render (§12.3). `ShellRuntime::drop` clears the callback arena,
shuts the scheduler down, and clears the entity store, all before the QuickJS
runtime is torn down — a `Persistent` released after its runtime aborts the
process, which is why field declaration order in `ShellRuntime` is load-bearing
and commented as such.

What is not yet built is observability: there is no `gc_stats`, so a slow leak
would be found by noticing memory rather than by reading a number.

---

## 8. The Render Protocol

This chapter is language-independent; both engines share `spec.rs` and
`materialize.rs`.

### 8.1 The constraint: GPUI elements are consumed values

```rust,ignore
#[derive(IntoElement)]
pub struct Button { /* ... */ }

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement { /* ... */ }
}
```

Three facts decide everything downstream. `render(self, ...)` consumes the
element by value, so an element value can be used exactly once.
`.child(impl IntoElement)` likewise takes its child by value. And a view's
`Render::render` rebuilds the entire tree from scratch on every redraw.

A JavaScript object therefore cannot *be* an element, and a mapping from a
script `Button` object to a Rust `Button` entity does not exist, because
`Button` was never an entity.

### 8.2 Why descriptions rather than a retained tree

Two alternatives were considered and both stay rejected.

A **retained script control tree with a Rust mirror** would mean building a
virtual DOM and a reconciler on top of GPUI, which already rebuilds from scratch
every frame. The reconciler would exist only to undo GPUI's model.

**Plain data object literals** — a script returning nested objects for Rust to
interpret — are exactly equivalent to the builder chain but constitute a second
way to write the same thing. Examples, documentation, type declarations, and
generated code would immediately split into two dialects, and JavaScript makes
that temptation sharper than Lua did, because object literals are the most
natural thing in the language and React has made "UI is data" reflexive. What it
would buy — fewer host calls per operation — is available from memoization and
virtualization instead (§20.4).

### 8.3 The description arena

```rust,ignore
pub struct SpecArena {
    nodes: Vec<SpecNode>,
    /// Nodes already attached to a parent. Re-using one is an error.
    parented: Vec<bool>,
    /// Nodes consumed by an op rather than by a parent — a state style's
    /// declarations. They take style ops but can never enter the tree.
    claimed: Vec<bool>,
}

struct SpecNode {
    component: Option<Component>,
    ops: SmallVec<[SpecOp; 8]>,
    children: SmallVec<[SpecId; 4]>,
}

pub enum SpecOp {
    NullaryStyle(u16),                       // index into the reflection table
    ParamStyle(&'static str, SmallVec<[Bridged; 2]>),
    Method(&'static str, SmallVec<[Bridged; 2]>),
    Callback(&'static str, CallbackId),
    StateStyle(&'static str, SpecId),        // hover / active / focus
}
```

A script-side element wraps a `SpecId` and nothing else — in JavaScript, an
`__id` property on an object created from the shared prototype. Each method call
pushes one `SpecOp` and returns the same object, which is what makes the chain
work.

`Component` currently covers `Div`, `HFlex`, `VFlex`, `Text`, `Button`,
`Checkbox`, `Switch`, `Svg`, and `Input`. `Input` is addressed by its entity
handle rather than by an id, because the state is what identifies it and the
state outlives the description.

**Rust's move semantics survive the trip into a garbage-collected language as an
explicit runtime error.** Attaching a node sets `parented`; touching it again —
adding it to a second parent, or reusing it across frames — reports:

```text
element `Button` was already added to a parent; elements are single-use values
```

A node from an earlier pass reports differently, because it is a different
mistake:

```text
this element belongs to a previous render pass; elements are single-use values
and must be rebuilt each time render runs
```

`claimed` covers the third case: the detached node that collects a state style's
declarations (§10.4) takes style operations but can never enter the tree, and
says so if a script tries.

`debug_tree` renders the arena as text, which is what makes interface structure
assertable without a GPU (§22.1):

```text
v_flex .size_full .items_center .gap[Number(12.0)] .bg[Str("background")]
  text "Count: 0" .text_size[Number(12.0)] .text_color[Str("foreground")]
  Button "increment" :accessibility_label[Str("Increment")] .h[Number(28.0)] .bg[Str("primary")] :hover(.opacity[Number(0.9)]) :on_click(fn)
    text "Increment"
```

### 8.4 The render pass

```text
cx.notify() / an event / a state change
        │
        ▼
ScriptView::render(window, cx)
        ├─ SpecArena::reset() · CallbackArena::swap()
        ├─ CallScope::enter(phase = Render)                    §9
        ├─ call the script's render(cx)  →  root SpecId
        ├─ CallScope::exit()
        ├─ CallScope::enter(phase = Task) · drain the job queue §12.2
        ├─ materialize(root) → AnyElement                       (pure Rust)
        └─ the previous pass's callbacks are released by the swap
        │
        ▼
GPUI layout / paint (never re-entering script)
```

`materialize` is a depth-first walk in pure Rust: it takes each node out of the
arena, replays its ops in order, recurses into children, and produces an
`AnyElement`. Nothing survives the pass — not an element, not a callback, not
the `cx` handed to `render`. Because it never touches script, it can be
benchmarked and snapshot-tested independently of the VM.

Two things materialization decides that the description cannot.

**Text color is inherited while walking the description.** GPUI resolves
inherited text color at paint time, but an `svg` will not paint at all unless
the color is on its *own* style — and by paint time the description is gone. So
`materialize_node` carries a color down the tree: each node passes its own
`text_color` if it set one and the ambient color otherwise, and an `svg` writes
the result into its own style. That is what makes an icon inside a dark button
come out light without the script saying so twice, and it is the reason
`examples/js_todolist` can write `icon("check", 11)` and have it follow its row.

**An element becomes stateful only when a state style needs identity.** GPUI's
`hover` works on any interactive element, but `active` and `focus` need a stable
element identity. A plain `div` therefore stays identity-free unless a state
style demands one, at which point it takes an `ElementId` derived from its
position in the description — stable across renders for a stable tree, the same
property GPUI relies on for its own ids.

`text(...)` materializes as a `div` carrying a string child rather than as a
distinct element type, so every style method works on it unchanged. `Input`
materializes as an `InputBase` frame — not a bare `div`, because `InputBase`
carries the input semantics, the focused state style, and the accessibility role
— with three defaults applied before the script's own styling: a centered row,
full width, and click-anywhere-to-focus. A script can override all three and
does not have to remember any of them.

A component that cannot honor a state style says so rather than dropping it
silently: a state style on a `Switch` logs a warning, because `Switch` itself is
not the interactive element (`SwitchTrack` is) and the style has nowhere to land.

### 8.5 Re-entrancy

Several base components call application code back during GPUI's layout and
prepaint phases to render one item: `VirtualList`, `Tree`'s `TreeEntry`,
`Calendar`'s `CalendarItem`, table cells, and the dock renderers of §15. Those
callbacks happen outside `ScriptView::render`.

`ScopePhase::Layout` exists for them. It permits reading state and building
elements, and refuses `notify`, entity creation, and spawning, because changing
state during layout produces either an inconsistent frame or a recursive
invalidation.

`ScriptDockSkin` is the first thing to use it (§15). Every chrome callback runs
inside `in_layout_scope`, which pushes a *new* frame rather than reusing an
enclosing one — a dock area nested in a script view already has an outer scope —
so each callback starts on a fresh render-time budget and a `cx` captured during
an earlier call is still rejected. The scope inherits the enclosing view, because
chrome is drawn on behalf of whatever view is rendering and owns no view of its
own. The item renderers of `VirtualList`, `Tree`, `Calendar`, and `Table` will
take the same shape when they are bound.

### 8.6 Memoization

Elements cannot be cached across frames — they are consumed values — but
descriptions could be, and `gpui.memo` was the planned form. It is not
implemented. §20.4 explains why it is still the most valuable optimization on
the list.

---

## 9. Context Safety: CallScope

`scope.rs` is the crate's only `unsafe` module.

### 9.1 The problem

GPUI's core contexts are borrows:

```rust,ignore
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement
fn on_click(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut App)
```

A script object's lifetime is decided by the script's collector and cannot carry
a Rust borrow. A `cx` stashed in a module-level variable and used later from a
timer points at a stack frame that is long gone. JavaScript makes this easier to
do than Lua did, because an arrow function captures its enclosing scope with no
explicit act at all.

### 9.2 The design

```rust,ignore
pub fn enter(window: &mut Window, app: &mut App, phase: ScopePhase,
             view: Option<Entity<ScriptView>>) -> (CallScopeGuard, u64);
pub fn with_context<R>(generation: u64,
             f: impl FnOnce(&mut Window, &mut App) -> R) -> Result<R, StaleContext>;
```

Every Rust → script entry point — render, event dispatch, timer, task
resumption, view construction — pushes a frame with a fresh generation, and
`CallScopeGuard` pops it on return. The script-side `cx` is an ordinary object
carrying nothing but that generation; every use compares it against the top of
the stack, and a mismatch throws:

```text
cx is no longer valid: it was captured during an earlier call and used later.
Use gpui.spawn or take cx from the callback arguments instead.
```

The `unsafe` is confined to this one module and its preconditions are written
into the module header: the VM and `App` are both main-thread only, so no other
thread can observe the stack; frames are strictly last-in-first-out, enforced by
the guard; and a frame's pointers are only reachable while its guard is alive.

A script cannot forge a generation. The `cx` object exposes no field carrying
it — the number exists only in the Rust closures the object's methods were built
from — so even `Object.keys(cx)` shows nothing but the methods.

Three accessors read the stack without a generation, and each exists for a
specific reason. `with_current_app` lets value conversion resolve a theme token
without threading a context through every coercion. `with_current` gives entity
creation the `Window` and `App` it needs at a point the host does not control.
`current_view` is what a callback registration records so a later `notify` knows
what to reach.

### 9.3 What each phase permits

| Phase | Permits | Refuses |
| --- | --- | --- |
| `Render` | Reading state and the theme, building elements, registering callbacks | `notify`, creating entities, opening overlays |
| `Event` | Everything: mutating state, `notify`, `spawn`, overlays | Blocking |
| `Task` | Same as `Event` | Blocking |
| `Layout` | Reading and building elements (§8.5) | `notify`, creating or destroying entities, `spawn` |

Every refusal is a specific message, not undefined behavior:

```text
cx.notify() is not allowed during the `render` phase;
request a re-render from an event handler instead
```

```text
cx.open_dialog(view, options) is not allowed during the `render` phase;
overlays may only be opened or closed while handling an event or a task
```

The phase is also what gives the interrupt handler a per-call deadline (§19.3):
render runs on a tighter budget than an event handler, and the change of
generation is what tells the handler a new call has begun.

---

## 10. Events and Callbacks

### 10.1 Registration and lifetime

```js
Button.new("save")
  .child(text("Save"))
  .on_click((event, cx) => {
    this.saved = true;
    cx.notify();
  });
```

The arrow function is not a style preference. It does not bind its own `this`,
so `this` remains the view instance and the handler can mutate view state
directly. A `function () {}` handler gets the wrong `this` — the mistake
JavaScript authors and models make most often here — which is why every example
and every declaration comment uses an arrow function.

`on_click` stores the function in this pass's `CallbackArena` and records only a
`CallbackId` in the description; the high 16 bits are the render generation and
the low 16 the index. At materialization Rust builds a closure holding a
`Weak<ShellRuntime>` and that id.

A callback belongs to the render that produced it. The next render replaces the
whole arena, keeping the previous generation one pass longer because an event
can be dispatched between a render and its paint. An event that arrives more
than a generation late is dropped with a `debug` log rather than an error — the
author did nothing wrong.

### 10.2 Event objects

Events arrive as plain objects whose field names mirror the Rust structs:

```js
.on_click((event, cx) => {
  // event.click_count === 1
  // event.modifiers === { shift: false, control: false, alt: false, platform: true }
});
```

Only semantics base has already normalized are exposed; platform events are not.
Base has already collapsed "Enter activates the button" and "click the button"
into one callback, and the script should not see that difference.

### 10.3 Controlled values

Base's controlled components report intent rather than mutating their own state
(see "Controlled values" in [ARCHITECTURE.md](ARCHITECTURE.md)), and the
bindings preserve that:

```js
Checkbox.new("agree")
  .checked(this.agreed)                    // the value comes from script state
  .on_change((checked, cx) => {            // this is only a request
    this.agreed = checked;
    cx.notify();
  });
```

The shell never quietly maintains a checked state on the script's behalf. Doing
so would give script authors and Rust authors different mental models of the
same control, in the same application.

### 10.4 State styles

Hover, active, and focus styles reuse the ordinary style methods on a detached
node, so there is no second grammar for what a style is:

```js
Button.new("save")
  .bg("primary")
  .hover((style) => style.opacity(0.9))
  .active((style) => style.opacity(0.8))
  .focus((style) => style.border_color("ring"));
```

The declaring function receives a detached element; its return value is ignored,
so both a chain and a block body work. In the arena this is
`SpecOp::StateStyle(name, node)` pointing at a node marked `claimed`, and
`materialize` resolves it into a `StyleRefinement` applied through GPUI's native
`hover`, `active`, and `focus` modifiers.

The semantic state styles base offers through `state_style::resolve_style` —
checked, selected, disabled — are **not** bound. A script expresses those
conditionally instead:

```js
Button.new("save")
  .when(disabled, (el) => el.opacity(0.4))
  .when(selected, (el) => el.bg("muted").border_color("foreground"))
```

That is a real gap rather than a simplification, because it means the semantic
precedence rules in [Styling and Motion](STYLING-AND-MOTION.md) are not
available to a script; the script re-derives them with `when`.

### 10.5 Actions and key bindings

Not implemented. GPUI actions are types generated by `actions!` at compile time,
and a script cannot produce a Rust type, so the intended shape is a single
`ShellAction { id: SharedString }` with script ids interned into `&'static str`.
The only key bindings installed today are `ShellRoot`'s own Tab and Shift-Tab
(§16).

### 10.6 Entity subscriptions

```js
const state = InputState.new({ placeholder: "What needs doing?" });
state.on("submit", (event, cx) => this.add(cx));
```

A subscription is a long-lived callback and is owned by the entity store rather
than the script (§7.3). The valid event names come from `InputEventName`, and a
misspelling reports all of them.

---

## 11. State

### 11.1 Three layers

| Layer | Where it lives | Suited to | After a change |
| --- | --- | --- | --- |
| View-local | Fields on the view instance (`this.count`) | Expansion, filters, drafts | `cx.notify()` |
| Host entity | `Entity<T>` behind a handle (`InputState`) | Text, and later trees, tables, dock layout | The entity notifies itself |
| Application-wide | `gpui.store` (§17.3) or module scope | Settings, caches | Subscribers notify explicitly |

### 11.2 There is no automatic dependency tracking

No signals, no observables, no automatic `notify`. Three reasons, and the first
is the one that matters: GPUI is an explicit `cx.notify()` model, and two mental
models coexisting in one application interfere with each other. Automatic
tracking would also mean wrapping every view instance in a `Proxy`, a permanent
cost on the render path that QuickJS has no JIT to amortize — the measured price
of one diagnostic `Proxy` is in §13.2. And a forgotten `notify` has a definite
symptom (the interface does not update) that costs far less to diagnose than
over-triggering does.

This has to be said louder in JavaScript than it would in Lua, because the
entire front-end ecosystem assumes the opposite: **there are no signals here, no
`useState`, and no dependency arrays. Change state, then call `cx.notify()`.**

### 11.3 View definition

```js
import { View } from "gpui";

export default class Counter extends View {
  init(props = {}) {            // once, at construction; phase = Event
    this.count = props.start ?? 0;
  }

  render(cx) {                  // phase = Render; returns exactly one element
    return v_flex().gap(12).child(text(`${this.count}`));
  }
}
```

`View`'s constructor does one thing: if the subclass defines `init`, it calls
it. Authors do not write `constructor` directly because a `constructor` must
call `super(props)` before touching `this`, and every forgotten `super` is a red
exception; `init` has no such trap, and it also matches the Lua engine's
`init` convention.

A view constructed for a dialog or a sheet is built with `new Class(props)`
directly, and `View`'s constructor forwards the argument to `init` — so the same
protocol covers both the root view (constructed with no properties) and an
overlay body (§16).

---

## 12. Asynchrony

### 12.1 Executors

This workspace does not depend on tokio. GPUI supplies a foreground executor
(main thread, same thread as the UI, can reach `App`) and a background executor
(a thread pool for `Send` computation and IO). Script runs only on the
foreground and never enters the background.

### 12.2 Promises, `await`, and the job queue

Script code is asynchronous in the ordinary JavaScript way. What
`engine/quickjs/scheduler.rs` supplies is the half a bare QuickJS runtime does
not have: a clock, an owner for pending work, and somebody to pump the queue.

```js
gpui.spawn(async (cx) => {
  await gpui.sleep(200);
  gpui.with_cx((cx) => {      // obtain a context that belongs to this call
    this.ready = true;
    cx.notify();
  });
});
```

**Nothing after an `await` runs until the host drains the queue.** QuickJS keeps
promise reactions in a job queue that only runs on request, so every script
entry point ends with `drain_jobs`: render, click dispatch, change dispatch,
input events, and every resumption the scheduler drives. Two placement rules are
load-bearing and are written into the function's own documentation. The drain
must happen *outside* `Context::with`, because `execute_pending_job` takes the
runtime lock that `Context::with` already holds. And it must happen *inside* the
entry point's scope guard, because a resumed continuation is script code that
will ask for a `cx` of its own — which is why a render pass opens a fresh
`ScopePhase::Task` scope around its drain rather than draining under `Render`.

A job that throws is reported and the drain continues; one broken continuation
must not stop the others. The drain is bounded at 100,000 jobs so that
`for(;;) Promise.resolve().then(f)` cannot wedge the frame loop, and hitting the
bound is itself an error log.

**A `cx` must not be held across an `await`.** After an `await` the generation
has moved and the old token produces the §9.2 error. The correct form is
`gpui.with_cx(...)`, or taking `cx` from the callback arguments. This is easier
to get wrong in JavaScript than it was in Lua, because the code before and after
an `await` shares one lexical scope and the old `cx` is simply in reach.

**An unhandled rejection must be visible.** A failed promise with no `catch` is
silent by default in JavaScript. `gpui.spawn` adopts the promise it is given and
attaches reporting handlers, so a rejection reaches `tracing::error!` with the
script's own stack rather than vanishing. A body that throws synchronously is
absorbed the same way.

**Top-level `await` is not supported.** Module evaluation must complete
synchronously; anything needing asynchronous start-up does it from `init` with
`gpui.spawn`.

### 12.3 Ownership and cancellation

```js
const task = gpui.timer.every(1000, (cx) => { /* ... */ });
task.cancel();
task.is_done();
```

Every task belongs to a view: `opts.owner`, or the view whose call is in
progress. The task holds a `WeakEntity<ScriptView>`, so when the panel that
started the work closes, the callback is skipped instead of writing into state
nothing will ever render again. That failure mode is worse in script than in
Rust, because it does not panic — it silently mutates an object nobody will look
at.

`opts.owner: null` is the deliberate opt-out for work that must outlive every
view. Any *other* view is refused rather than silently ignored, because the
engine can only resolve the current view's script instance back to its entity,
and a task that quietly took the wrong owner is exactly the bug ownership
exists to prevent.

There is no language-level way to interrupt a JavaScript function that is
already inside an `await`, so cancellation means the runtime stops resuming it:
a cancelled timer does not fire again, a cancelled `sleep` leaves its promise
pending forever, and a cancelled `spawn` stops having its outcome adopted. A
cancelled task reports itself done and its registry entry is reaped immediately,
so a long-running application does not accumulate one entry per elapsed timer.

### 12.4 No background script

Running script on the background executor is deliberately not offered. Only
host-implemented Rust work can be dispatched there, and its arguments and
results must be plain, thread-transferable data. There is no `Worker`.

### 12.5 Timers

`gpui.timer.after(ms, fn, opts?)` and `gpui.timer.every(ms, fn, opts?)`, both
owner-bound. The interval on `every` is measured from the end of one callback to
the start of the next wait, so a slow callback delays the next tick rather than
piling ticks up behind it.

There is no global `setTimeout` or `setInterval`. They are not part of the
JavaScript language — a host provides them — and they have no owner, so a
`setInterval` keeps running after the panel that started it is closed, which is
exactly what §12.3 exists to prevent. The names are present as throwing stubs
that point at the replacement (§19.1); a name that errors usefully beats a name
that is simply missing.

---

## 13. Styling and Theme

Presentation authority sits in script, so this is a core chapter rather than a
supporting one. All of it is above the seam: both engines share `style.rs` and
`theme.rs`.

### 13.1 No-argument styles: reflected, zero maintenance

`crates/ui/src/inspector.rs` already does something this runtime needs:

```rust,ignore
let table: Vec<_> = [
    gpui_base::styled_ext_reflection_methods::<StyleRefinement>(),
    gpui::styled_reflection::methods::<StyleRefinement>(),
]
.into_iter()
.flatten()
.collect();
```

That yields a name → style-method table at runtime. The shell uses the same pair
of APIs — one from `gpui-base`, one from `gpui`, neither requiring
`gpui-component` — and exposes **3,148 no-argument style methods** to script:
`flex`, `flex_col`, `items_center`, `gap_2`, `rounded_md`, `text_sm`,
`size_full`, and the rest. When upstream GPUI adds one, script gets it with no
change here. They are addressed by a `u16` index, so recording a style call
costs the arena two bytes rather than a string.

Three constraints follow.

`FunctionReflection::invoke` takes only a receiver, so reflection covers exactly
the `fn(self) -> Self` shape. Anything taking an argument has to be bound by
hand (§13.2).

Reflection is behind `#[cfg(any(feature = "inspector", debug_assertions))]` in
both crates, so `crates/shell` enables `gpui-base/inspector` unconditionally and
`style.rs` carries a test asserting the table has more than a hundred entries —
a test that is meaningful only when CI runs it in release.

Nine names reflection cannot see are added by hand: `gpui-base` generates its
font-weight helpers with a macro, and the reflection pass does not see
macro-expanded trait methods, so `font_thin` through `font_black` would
otherwise be missing entirely. They are appended after the reflected table and
addressed by the same `u16`.

The QuickJS engine does one more thing at start-up: it hands the name list to a
JavaScript prelude, which loops over it and installs one small function per name
on the element prototype, each forwarding to a single Rust entry point:

```js
const define = (name) => {
  methods[name] = function (...args) {
    __apply(this.__id, name, args);
    return this;
  };
};

for (const name of __styleNames) define(name);
```

Three thousand small JavaScript closures, not three thousand registered Rust
closures — which would cost both memory and one cross-language registration
each.

### 13.2 Styles that take arguments

Fifty-seven methods are bound by hand in `style.rs`, and they are indistinguishable
from the reflected ones at the call site:

```js
v_flex()
  .size_full().items_center()               // reflected
  .bg("surface").p(12).rounded(8).gap(8);   // hand-bound
```

They divide as 9 size, 7 padding, 7 margin, 5 position, 6 flex, 6 paint, 8
border, and 9 radius. Five families are deliberately unbound, with the reason
recorded at the head of the array: `shadow` takes a `Vec<BoxShadow>` and belongs
with the animation and token work rather than as a positional argument list;
`cursor`, `text_align`, `text_overflow`, and `font_weight` take GPUI enums and
would each need a name mapping when every variant already has a nullary form
(`cursor_pointer`, `text_center`, `font_bold`); and `scrollbar_width` is
meaningless without overflow configuration the shell does not yet expose.

There is no `.class("flex gap_2")` string form. It would be exactly equivalent
to the chain while becoming a second style syntax that examples, declarations,
editor completion, and generated code would all have to support — and the string
form is the one with the weakest completion and the weakest static checking.

#### The cost of a good diagnostic

Prototype dispatch gives no diagnostic by itself: a name that is not on the
prototype never reaches `__apply`, and QuickJS reports it as
`TypeError: not a function` without naming the property. A mistyped style name
would arrive with no clue at all — and giving the call site a real diagnostic is
the entire reason the style surface is methods rather than a string of class
names.

A `Proxy` prototype solves it, and the M0 benchmark measured what it costs:
**1.09 ms → 1.42 ms for 443 nodes, about 30% of the whole description pass.**

So the implementation keeps two prototypes. The plain one is the default. A
render that fails with "not a function" is re-run once against a `Proxy`
prototype whose `get` trap returns a function that throws with a "did you mean"
suggestion, purely to produce the message; the arena is reset between the two
passes and the flag is cleared afterwards. Errors are rare; a 30% tax on every
frame is not.

The cost of *that* is a string match — `error.to_string().contains("not a
function")` — deciding whether to re-run. It is a fragile hinge, and a QuickJS
wording change would silently degrade the diagnostic to the bare `TypeError`
rather than break anything visibly.

The suggestion itself is Levenshtein distance over the full name list, with a
budget of two edits relaxed to a third of the name for longer identifiers,
because a wrong suggestion is worse than none:

```text
unknown element method `items_centre` (did you mean `items_center`?)
```

Two further guarantees hold regardless of engine. `gpui.d.ts` plus `// @ts-check`
catches the same class of mistake in the editor, before anything runs (§14.4).
And nothing is ever silently ignored: any name reaching `__apply` that the
dispatcher does not recognize throws.

### 13.3 Semantic tokens and the default palette

`gpui_base::Theme`'s `ColorTokens` derives `Default`, so its colors are all
zero — fully transparent. `RadiusTokens` and `SpacingTokens` have real defaults.
Calling `gpui_base::init(cx)` without supplying colors paints an invisible
window.

The shell therefore does two things. It ships `theme/default-tokens.json`,
embedded with `include_str!` so a binary is self-contained, and deserializes it
into `Theme::global_mut(cx).tokens` at `init`. There is no schema cost:
`SemanticThemeTokens` and its sub-structures already derive `Serialize`,
`Deserialize`, and `JsonSchema`, and the shipped file is the reference document
for that format. Two palettes are defined, light and dark; the scales are omitted
from the file and fall back to base's defaults, which is asserted by a test.

And it resolves token names for script. Seventeen colors — `background`,
`foreground`, `surface`, `primary`, `muted`, `accent`, `destructive`, `border`,
`input`, `ring`, and the `*_foreground` pairs — plus seven spacing steps and six
radius steps. A test compares each name list against the serialized field names
of the corresponding token struct, so adding a token upstream without adding its
name here fails.

Token lookup is cached in a thread-local rather than read from the `App` on every
access, and that is a bug fix rather than an optimization. Lookups happen in two
places with different access to the host: while a script records a style (inside
a call scope, `App` reachable) and again while the description is materialized
(outside any scope, `App` *not* reachable). Reading only through the scope made
every color silently resolve to `None` during materialization — a window that
painted nothing but a black rectangle. The palette changes at most once per theme
switch, so caching it is both correct and cheaper.

Rules for script:

- Prefer a token name: `.bg("surface")`. Hex literals (`#rgb`, `#rrggbb`,
  `#rrggbbaa`) are accepted for one-off tools, and the documentation says
  plainly that a literal bypasses the theme and will not follow a theme switch.
- An unknown token is an error listing the valid set, never a transparent
  fallback — that would reproduce the exact failure this module exists to
  prevent.
- This matches `CLAUDE.md`: the theme API exposes semantic tokens, not a growing
  set of component-specific fields.

`gpui.theme()` and `gpui.set_theme(...)` are not bound yet; `theme::set_mode` is
the Rust-side entry point, and it returns whether anything changed so a caller
mirroring an OS appearance change can skip the refresh.

### 13.4 The preset module

There is no `gpui/preset` module. `examples/js_todolist/ui.js` is what a preset
would be — `button`, `iconButton`, `checkbox`, `field`, `label`, `muted`,
`surface`, `emptyState` — written as ordinary JavaScript in the application, and
it is instructive that it stayed there. Three disciplines still apply to
anything that does ship:

1. it must be script source, replaceable or forkable wholesale;
2. the Rust side installs no visual decision (§5.6), or the shell becomes a
   third, uncontrolled visual system on top of base;
3. it is not a reproduction of `gpui-component` and promises no visual parity
   with it.

The seam's real cost surfaces here: Rust above the seam is written once, but
script shipped with the runtime has to be written per engine. The only control
is keeping any preset thin.

### 13.5 Animation

Not bound. Base's `motion` (`Transition`, `Spring`, `Interpolate`) and
`animation` are the intended target, with the script describing target and
timing while interpolation stays in Rust.

---

## 14. Bindings and Generated Declarations

### 14.1 The surface, measured

The bound surface today is small and deliberately so: `div`, `h_flex`, `v_flex`,
`text`, `svg`, `Button`, `Checkbox`, `Switch`, `Input`, and `InputState`, plus
`child`, `children`, `when`, `on_click`, `on_change`, `disabled`, `selected`,
`checked`, `accessibility_label`, and the three state styles — over 3,148
no-argument and 57 parametric style methods.

That is a small prefix of `gpui-base`. What makes completing it plausible is the
size difference measured in §4.2: base's `Button` has 13 public functions to
`gpui-component`'s 52.

### 14.2 What will and will not be bound

Base's semantic elements, compound behavior roots, and stateful systems are all
in scope: Checkbox, Radio, Switch, Toggle, Link, Input/Textarea, Select,
Combobox, Tabs, Dialog, Sheet, Popover, Tooltip, Scrollbar, Tree, Table,
VirtualList, Dock, and the rest of the [module
families](ARCHITECTURE.md#module-families).

`input::Editor`'s LSP, folding, diagnostics, and highlighting interfaces are
not, and will not be. They are built from Rust traits and generics —
`InputHighlighter`, `CompletionProvider`, `HighlightStyleResolver` — where
cross-language mapping is both expensive and lossy, and they are exactly the
part that belongs in Rust (§3). An editor should be exposed through a narrow
"here is the text, here is the language, here is the read-only flag" interface
instead.

### 14.3 There is no binding table

The design called for bindings declared as data in `crates/shell/src/bindings/`
and expanded by a macro. It does not exist. Component methods are matched by name
in each engine's `apply`, and the behavior name list is a literal array in
`install_globals`. With ten components that is smaller than the macro would be;
it will not stay that way, and the argument for a table — one source of truth for
the runtime registry, the declarations, and the documentation — is unchanged.

The style surface already works the way the whole binding layer should: it is a
table, it is generated, and nothing about it is written by hand.

### 14.4 `gpui.d.ts`

`typings.rs` generates TypeScript declarations for the script API.
`gpui-shell types <directory>` writes `gpui.d.ts` next to an application, and the
output is deterministic — no timestamps, no reflection order — so regenerating
after a runtime upgrade produces a reviewable diff.

What makes the declarations trustworthy is that they are generated from **the
tables the runtime dispatches through**, not transcribed from documentation:

- style methods come from `style::known_names()`, the same list the prelude
  loops over, so a name that type-checks is a name the dispatcher accepts;
- a parametric method's argument type is *probed*: `argument_of` hands
  `style::apply_param` one literal of each shape and sees which are refused, so
  the difference between `Length`, `DefiniteLength`, `AbsoluteLength`, a color,
  and a bare number is decided by the code that enforces it. `.p("auto")` and
  `.rounded("50%")` are type errors for the same reason they throw;
- the color union comes from `theme::color_token_names()`, so a mistyped token
  is a compile error;
- the phase union comes from `ScopePhase` itself.

An unrecognized argument shape is emitted as `never` rather than `any`, so a
style method added without a matching probe literal fails loudly at the first
call site rather than silently accepting anything.

Four things the declarations deliberately do not express, each stated in the
generated file's own preamble. Capability grants: every `fs`, `store`,
`clipboard`, and `process` call type-checks, and whether it is *granted* is a
runtime question types cannot carry. Element and `cx` lifetimes: TypeScript has
no affine types, so reusing an element still type-checks and still throws.
Which methods suit which component: every element shares one prototype, so
`.checked(true)` is declared on all of them and is simply inert on a `div`;
narrowing it would mean inventing a type hierarchy the runtime does not have.
And retained entities.

The application stays plain JavaScript with no compile step; `.d.ts` is an
annotation for the editor and for `// @ts-check`. It is also the form in which
the API is handed to a model, which is an explicit audience.

### 14.5 Drift

There is no automated drift check. The intended one — read `crates/base`'s
public API with `cargo rustdoc --output-format json` and compare it against the
binding table — needs the table of §14.3 to exist first.

Drift within the crate is caught: `typings.rs` has tests asserting that the
declared element methods and the runtime's style table have not diverged, that
no style method collides with an element method, that every parametric method is
classified, that every color token is in the union, and that no internal name
(`__id`, `__apply`, `__gpui`) leaks into the declarations.

Those tests do not catch a *missing* declaration, and there are several: `svg`,
`Input`, `InputState`, `accessibility_label`, and every overlay method on `cx`
are bound at runtime and absent from `gpui.d.ts`. Running `// @ts-check` against
the generated declarations would flag the shipped example.

### 14.6 A `gpui-component` module, later

The natural second step is a `gpui-component` binding as a *second registry*
sharing the same render protocol, call scope, event model, and arena:

```js
import { v_flex, text } from "gpui";              // base: the script owns presentation
import { Button } from "gpui-component";          // product visuals, ready-made
```

Four points decide it now rather than then. The protocol is one thing and the
registries are two, which is exactly what separating the render protocol from
component bindings bought. The crate dependency stays optional, behind a feature
or in its own crate, so not enabling it keeps `gpui-component` out of the tree.
The two module names must be distinct from the start, because both export
`Button` with overlapping method names and different semantics, and in JavaScript
they can be imported into the same file — the module name is the only thing that
can distinguish them. And migration is then a change of import rather than a
rewrite: the functions that build the interface change, the business logic and
the state do not.

---

## 15. Dock and Panels

Not implemented. No renderer is supplied, no `ScriptPanel` exists, and no script
panel can be registered with `PanelRegistry`.

The substrate is ready and is the reason this is worth doing at all. Base owns
the layout tree, persistence, drag hit-testing, resize arithmetic, zoom, focus,
and the panel registry; `crates/ui/src/dock` is a skin over it that supplies tab
bar, toolbar, drop indicators, and dock-toggle appearance through three renderer
traits. A shell skin would forward those three traits to script, which makes the
dock's appearance script-decided for the first time, while base keeps the drag
source, drop-target hit testing, keyboard actions, and focus — a script would see
only resolved state through `TabGroupContext`, `DockContext`, and `TileContext`,
never a drag event.

Two constraints are already known and do not change. `Panel::panel_name`
returns `&'static str` while a plugin's name is only known at runtime, so
registration needs a process-wide intern table with a one-time `Box::leak`;
the bound is loaded plugins × panels each, which is in the hundreds, and an
unloaded plugin's string is not reclaimed. And the name must be prefixed
`script:<plugin_id>/<panel_id>` — with `script:` rather than a language name, so
one layout file still restores after an engine change.

The property that makes this worth building is one base already guarantees: when
a panel's name is not in the registry, `DockArea` substitutes a draw-nothing
placeholder and preserves the original `PanelState`, writing it back on the next
save. A user can uninstall a plugin and reinstall it, and its panel returns to
where it was.

---

## 16. ShellRoot: the Window and its Overlays

### 16.1 Base ships the parts, not the host

`Root` belongs to `gpui-component`. Base ships the pieces: `Dialog` and `Sheet`
each build their own viewport-sized host, `ToastManager` and `ToastStackState`
own stacking geometry and lifecycle, `FocusTrapElement` owns focus trapping,
`Popup` and `Positioner` own placement and collision. What base does not decide
is what happens when two of them are open at once.

`ShellRoot` is that decision, and it is the only reason `root.rs` exists: a
stacking order plus a dismissal order, with the smallest presentation that makes
them visible. The first view of a shell window is always a `ShellRoot`, the same
way the first view of a `gpui-component` window is always a `Root`, and a script
reaches it only through `ShellRoot::update` — never by constructing overlays
itself.

**Stacking**, painted back to front: the script's content; at most one sheet,
anchored to a viewport edge; the dialog stack in open order, each deferred at
`10 + index` so a later dialog always paints over an earlier one regardless of
element build order; and toasts above everything at `POPUP_PRIORITY + 1`. A
sheet sits *below* the dialog stack because a sheet is a place in the window: a
dialog raised from inside a sheet must be readable, and a sheet raised under a
dialog must not cover it. Only the topmost dialog draws a backdrop, so a stack
of three dims the window once rather than three times, and that single backdrop
is what separates the live dialog from the inert ones behind it.

**Dismissal** is always one layer, never a cascade. Escape closes the topmost
dialog only; lower dialogs render with keyboard handling disabled, so repeated
presses unwind the stack one dialog at a time and never reach the sheet while a
dialog is open. `escape_dismissable: false` withdraws the *key binding*, not the
underlying cancel action, so a close control the script put inside the dialog
still works — which is what makes an undismissable dialog one the user must
answer rather than one they cannot leave. A backdrop press closes the topmost
dialog only if it was opened `backdrop_dismissable`. Enter does nothing at this
layer: base's dialog host treats it as "confirm and close", which belongs to the
dialog's own primary button, so the root vetoes the built-in confirmation rather
than guessing what the content wants.

**Focus** is recorded on open and restored on close, so a stack restores through
its own history: closing the second dialog returns focus to the first, and
closing the first returns it to wherever the window was. `close_all_dialogs`
restores to where the *first* dialog took focus from, because restoring through
each in turn would flicker focus across views about to be dropped. Tab and
Shift-Tab honor base's focus trap, with a wrap-around loop bounded at 100 steps
so a trap with no focusable child cannot spin.

**Toasts** are data, not views — a title, an optional description, a level, a
timeout, and an optional id — which is what lets the root own the geometry and
lifecycle without asking the script to render anything. Pushing the same id
twice replaces rather than stacks, so a repeated "Saved" reads as one event.
Three are mounted at a time and the rest wait, so a burst is throttled rather
than lost. A 50 ms clock advances the lifecycle, paused while the stack is
expanded or the window is inactive — a toast that expired unseen behind another
window was never delivered.

### 16.2 The script surface

```js
const depth = cx.open_dialog(ConfirmClear, {
  props: { count, onConfirm },
  escape_dismissable: false,
});
cx.close_dialog();       // -> was anything open?
cx.close_all_dialogs();  // -> how many closed

cx.open_sheet("right", FiltersPanel, { props: { filters } });
cx.close_sheet();

cx.toast({ title: "Saved", description: "3 files", level: "success",
           timeout: 4000, id: "save" });
cx.dismiss_toast("save");
cx.dismiss_all_toasts();
```

Four details are worth stating because each was a decision.

`open_dialog` and `open_sheet` take the view *class*, not an instance and not an
element. The runtime constructs it, passing `props` to the constructor, which
`View` forwards to `init`.

`open_dialog` returns the new stack depth rather than a handle. The root
addresses dialogs by position and never by identity, so a handle would have to
promise "close *this* dialog", which is not an operation that exists. The depth
is what a script can actually use.

A misspelled option is refused, not ignored. `{ escapeDismissable: false }`
throws and names both the offending key and the valid set — a silently ignored
option would leave the dialog dismissable while the call looked like it worked.
This applies to `props` too, which is a named key rather than a bare object:
passing `{ count: 3 }` where `{ props: { count: 3 } }` was meant is an error at
the call site.

An absent `timeout` keeps the default; an explicit `null` asks for a toast that
stays until dismissed. The two cannot be collapsed into one optional.

Every entry point checks the phase before doing anything, and refuses `Render`
and `Layout` (§9.3). The check exists in both `overlay.rs` and `ShellRoot`
because the two refusals are different: the root logs and ignores, which is right
for host code that got it wrong, while a script gets a thrown `TypeError` naming
the phase it called from — the only shape an author can act on.

### 16.3 Windows and window decoration

`gpui.open_window` is not bound; the host opens exactly one window (§23). There
is no `TitleBar` or `window_border` component, so a script that wants one draws
it; the behavior bindings for drag regions, double-click maximize, and window
buttons are not built either.

---

## 17. System Capabilities

Everything here is denied by default and gated on the capability set in force
(§19.2). Capability decisions and path resolution live above the seam in
`capability.rs`; the engine holds only the argument shuffling.

```js
import { fs, store, clipboard, log, process } from "gpui";
```

Two rules keep this honest. **There is one path resolver:** every filesystem
path goes through `Capabilities::resolve`, never through `std::fs` directly, so
`gpui.fs` and every later path-taking entry point share one policy and there is
no second place for a traversal bug to hide. **A denial names its manifest
key:** the error a script sees is the instruction for fixing it.

```text
`/etc/passwd` is outside every granted read root; add its directory to
capabilities.fs.read in the manifest
```

```text
running `curl` is not granted; add it to capabilities.fs.execute in the manifest
```

### 17.1 Filesystem

`read_text`, `write_text`, `read_dir`, `exists`, `remove`, `create_dir_all`.
Paths resolve against the granted roots; traversal is rejected by lexical
normalization plus a `starts_with` check, and symbolic links are re-checked at
the system call.

Three shapes are deliberate. `read_dir` sorts by name, so a script rendering a
listing does not inherit the filesystem's arbitrary order and does not have to
sort. `exists` *throws* on a denied path rather than answering `false`, because
"you may not look" and "it is not there" are different facts and collapsing them
would let a script probe outside its roots one boolean at a time. `remove` is
not recursive: write access is granted per root, so a recursive remove would turn
one mistyped path into the loss of an application's whole data directory.

**These calls are synchronous.** The design requires them to be asynchronous —
blocking IO on the render thread stalls the frame — and they are not yet. Every
body is deliberately a capability check plus one `std::fs` call, so the move is
mechanical: hand the closure body to `gpui.spawn` and return its promise,
changing nothing about the checks. The generated declarations say so at the type.

### 17.2 Network

Not implemented. `Capabilities` carries a host allowlist and `may_reach`, and
nothing calls it. The `fetch` stub's message points at a `gpui.http` that does
not exist yet.

When it lands, the HTTP client must be **injected by the host** rather than
hard-wired: a desktop host can pass Zed's `reqwest_client` (which `crates/story`
already uses) and a WebAssembly host a fetch adapter, which is what keeps the
dependency restraint of §4.2. And it must not be called `fetch` or imitate its
signature. A name that matches while the semantics only mostly match is worse
than a different name: a model will generate against `fetch`'s full contract —
`Response.json()`, a streaming body, `AbortController` — and fail on the third
line.

### 17.3 Storage

`store.get`, `set`, `remove`, `keys`, `flush`. One flat JSON object per
application, cached in memory because `get` is called from `render` and a file
read per frame would be absurd.

Every mutation writes through immediately, to a temporary file that is renamed
over the target, so a crash mid-write leaves the previous settings intact rather
than a truncated file. The store holds small configuration data, and losing a
setting because a script forgot to call `flush` is a worse failure than one extra
rename; `flush` stays in the API as the durability barrier for when the write
becomes an awaitable promise.

A missing file is an empty store — a first run is not an error. A *malformed*
file is an error, because silently discarding a user's settings is worse than
refusing to start.

Values cross as JSON, depth-capped at 64 so a reference cycle cannot recurse
forever. Functions and `undefined` properties are dropped exactly as
`JSON.stringify` drops them, so a script author's mental model transfers, and
`NaN` and `Infinity` are refused by name because they have no JSON form.

### 17.4 Clipboard and logging

`clipboard.read_text` and `write_text`, with read and write as separate grants so
a denial names the half that was missing. Both need a live host call, and a
clipboard call from, say, a module's top level reports that rather than panicking.

`log.debug/info/warn/error` need no capability: a script that can run can already
say something, and denying it would only cost the author their diagnostics.
Output goes to `tracing` under the `gpui_shell::script` target, so script output
is separable from host output in a filter. Extra arguments are appended
space-separated the way `console.log` behaves, and any value prints — structured
values as JSON, an unprintable one as a placeholder rather than aborting the call
it was describing.

There is no `console`. The design called for `console.log` as an alias for
`log.info`, on the argument that its semantics are simple enough to map exactly
and that it is a JavaScript author's first reflex. That argument still holds and
the alias is simply not there, so `console.log` is a bare `ReferenceError` —
which is the outcome §19.1 exists to avoid.

### 17.5 Processes

`process.run(command, args?)` returns an exit code and requires the command to be
on the `capabilities.fs.execute` allowlist. `process.exit(code?)` requires a
filesystem grant and is a **request**: it records a code the host polls for and
decides what to do with — close the panel, close the window, or ignore it. It is
never `exit(2)`, because one plugin must not be able to take down an application
the user is working in.

`process` is installed as a global as well as a `gpui` module member, because
`process` is the name a JavaScript author, or a model writing JavaScript, will
reach for.

There is no streaming subprocess: pipe semantics conflict with the asynchronous
model, and a case that needs streaming output belongs behind a host-registered
native module that can return a structured result and a timeout.

### 17.6 Native modules

Not implemented, and the shape is settled: a host registers Rust modules at
compile time. There is no `dlopen`. Rust has no stable ABI, and native code
loaded into the process has every permission the process has, which makes the
sandbox meaningless. The cost — a third party who needs native capability must
fork the host or send a patch — is deliberately retained.

---

## 18. The Plugin Model

Not implemented. There is no manifest parser, no contribution registry, no
plugin loader, and no authorization UI. What exists is the type the manifest
would populate — `Capabilities`, with private fields, a builder, and an empty
default — and the host-side entry points that install one
(`gpui_shell::set_capabilities`, `set_store_path`).

Today the grant comes from the host directly. Running a directory from the
command line is an explicit act of trust, the same as `node app.js`, and
`gpui-shell` grants exactly this: read access to the application root and its own
storage directory, write access to the storage directory, and `store`. No
network, no process execution, no clipboard, and no filesystem access outside
those two directories (§23).

Three decisions from the design still stand and should shape whatever is built.

**Capability is permission; contribution is behavior.** A manifest answers two
questions — who is this, and what does it want permission to do — and nothing
else. Commands, panels, key bindings, settings, and themes are registered in
script, not declared a second time in the manifest. Declaring them twice
guarantees they will disagree, and the alignment produces no information.

**Lazy loading belongs at the module level, not the plugin level.** `main.js`
runs at start-up and its only job is registration; the real implementation lives
in other modules that a handler pulls in with a dynamic `import()` when it is
triggered. That is why dynamic `import()` is deliberately left callable by the
sandbox (§19.1): it is the mechanism, not a hole. It also gives every plugin a
non-zero start-up cost that has to be budgeted (§20.5) and makes "do real work in
`main.js`" a convention violation that tooling can flag.

**Identity is a namespace.** The manifest `id` prefixes the panel name
(`script:<id>/<panel>`), the store namespace, the log field, and the
authorization record — which is also what should replace the path digest that
identifies an application's storage today (§23).

---

## 19. The Sandbox

The language surface is engine-specific and this chapter describes QuickJS; a
Lua engine's trimming (`ffi`, `io.*`, `package.loadlib`, `debug.*`) is a
different list because it is a different attack surface. But capability decisions
and path resolution exist once, above the seam, so there is no room for one
engine's sandbox to be looser than the other's.

### 19.1 Language trimming

JavaScript's advantage here is that its standard library has no IO: apart from
`eval`, the language itself cannot reach a file, a process, or the network. The
exposure is therefore concentrated in four places — what the host injected, paths
from a string to executable code, module resolution, and the shared built-in
prototypes.

| Treatment | Target | Notes |
| --- | --- | --- |
| **Never added** | quickjs-libc's `std` and `os` | These provide `open`, `exec`, `getenv`, and `popen`; registering either is full access. `rquickjs` does not inject them and the shell never registers them. This is "never added" rather than "removed", which is an order of magnitude more reliable — and `rquickjs-sys` does not compile that file at all, so a test asserts their absence as a guard on the build |
| **Withheld** | `eval` and every function constructor | `globalThis.eval` is deleted outright; the `Function`, `AsyncFunction`, `GeneratorFunction`, and `AsyncGeneratorFunction` constructors are replaced with throwing stubs |
| **Replaced** | The module resolver (static and dynamic `import` alike) | Resolves only the built-in `gpui` module and paths inside the application root; anything else is refused before it reaches the filesystem. Dynamic `import()` stays callable — it is how §18 does lazy loading |
| **Frozen** | `Object`, `Array`, `Function`, `String`, and `Number` prototypes | One VM hosts several plugins, so the built-ins are shared mutable state |
| **Capability-gated** | `gpui.fs.*`, `process.run`, `process.exit` | §17 |
| **Throwing stub** | `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`, `fetch`, `require` | Present, and throwing a message that names the replacement |

Three of these are worth more than a table row.

**The `Function` constructor is replaced, not deleted, and all four of them are
swapped.** Deleting `globalThis.Function` would achieve nothing:
`(function(){}).constructor` is the same object, and each of the async,
generator, and async-generator function prototypes carries its own constructor
which is an independent compiler. The replacement also keeps the real
`Function.prototype` as its `.prototype`, because `x instanceof Function` and
`Function.prototype.{call,apply,bind}` are ordinary, legitimate JavaScript that
has nothing to do with `eval`. `eval` itself is deleted rather than stubbed,
because a `ReferenceError` cannot be mistaken for a working `eval` by feature
detection while a throwing stub can.

**This is the weaker of the two available layers, deliberately.** QuickJS makes
evaluation an *optional intrinsic*: a context assembled with
`Context::custom::<(Date, RegExpCompiler, RegExp, Json, Proxy, MapSet,
TypedArrays, Promise, Performance, WeakRef)>` — that is, `intrinsic::All` minus
`intrinsic::Eval` — has no `eval` and no compiler to reach in the first place.
The runtime uses `Context::full` instead, because `Ctx::eval` *is* `JS_Eval` and
the same intrinsic gates it: dropping it also disables the engine's own
`ctx.eval`, which is how the JavaScript prelude and the two policy snippets are
installed. Reaching intrinsic level means converting the prelude to
`Module::evaluate` or precompiled bytecode first. Until that happens, the
withholding layer above is what is actually in force.

**The DOM names are absent rather than stubbed.** `window`, `document`, and
`localStorage` are deliberately *not* given throwing stubs, even though
`setTimeout` and `fetch` are. Every bundle that does environment detection reads
them through `typeof`, and `typeof window === "undefined"` is the answer that
makes such a bundle take its non-browser branch; a throwing getter would turn a
working feature test into a crash.

Ordering is load-bearing and stated in the module header. The policy is
installed **after** the runtime's own globals — the prelude, the host API, the
scheduler — and **before** any application module is evaluated. Earlier, and the
prelude's own writes would land on prototypes meant to be frozen, and a later
subsystem could re-add a global this module means to withhold. Later, and
application code would already have had its turn with `eval` and a mutable
`Object.prototype`.

The freeze is switchable, and the trade is stated: a library that patches
`Array.prototype` — a polyfill, an older utility bundle — stops working, at
import time, with a `TypeError` that points at the library rather than at this
policy. A host that knowingly runs one can turn the freeze off and keep every
other part of the sandbox. Turning it off does not hand back a compiler, which
is asserted.

Freezing also does not make a sloppy-mode write throw; ECMAScript discards it
silently. That is the language's rule, not a hole, and the test asserts the
outcome that matters: the property never appears.

The module resolver is the one piece that had to be written rather than
configured. `rquickjs`'s `FileResolver` is unusable here because it tests
candidate paths against the process working directory, so an absolute
application path never matches. Owning the resolver also puts the module policy
in one place: a module must live inside the canonicalized application root, which
is what stops `import "../../../etc/passwd"` before the filesystem is touched.

### 19.2 Capability grants

`Capabilities::default()` is the empty set, every field is private, and
construction goes through a builder — so "no capability by default" is a fact
about the type rather than a promise in prose. The grant lives in a thread-local
and is re-read at every call, so revoking a capability takes effect on the next
call rather than on the next restart.

The three-state `granted` / `denied` / `prompt` model, the authorization UI, and
persisting a decision in host configuration are all part of §18 and not built.

### 19.3 Resource limits

| Limit | Mechanism | Value |
| --- | --- | --- |
| Runaway execution | `Runtime::set_interrupt_handler`, on a per-call deadline | 50 ms in `Render`/`Layout`, 500 ms in `Event`/`Task`, 5 s outside any scope |
| Memory | `Runtime::set_memory_limit` | 256 MiB — a leak reports as a catchable exception on the offending allocation rather than an OOM kill of the host |
| Stack | `Runtime::set_max_stack_size` | 1 MiB against QuickJS's 256 KiB default, so deep recursion is a `RangeError` a script can report rather than a native stack overflow, which is a process abort |
| Microtask storms | Bounded drain (§12.2) | 100,000 jobs per drain |

The budget is per host call, not global: every `scope::enter` mints a fresh
generation, and a change of generation is the signal that a new call has begun
and the clock restarts. That is what lets the render path have a tighter budget
than an event handler without reinstalling the handler between calls.

**An interrupt is not catchable from script.** The design flagged this as an
assumption that had to be measured rather than assumed: if a script could
swallow the interrupt with `try { while (true) {} } catch {}`, the interrupt
would not be a defence at all and the policy would have to escalate to discarding
the plugin's entire execution context. It was measured, and it cannot —
`an_interrupt_cannot_be_swallowed_by_a_catch_block` in `sandbox.rs` asserts it.
The interrupt is a real boundary, and the policy stays as it is.

### 19.4 Development mode

`--dev` restores `eval` and leaves the built-in prototypes writable, which a
REPL needs and a shipped application must not have. Capability gating is *not*
relaxed: development mode makes the language easier to poke at and never hands
out access the manifest did not declare, because a grant nobody wrote down is a
grant that will be missing in production.

It is not wired up. `gpui_shell::set_development_mode` exists and the binary does
not call it — a stale `TODO` in `bin/gpui-shell.rs` still describes the wrapper
as missing — so `--dev` today enables source watching and nothing else, and says
so with a warning at start-up. The visible development-mode marker the design
requires does not exist either.

Signature verification, an installation-time capability listing, and re-prompting
when an update adds a capability all belong to §18.

---

## 20. Performance

This chapter is language-independent and engine-sensitive, which is exactly why
the seam of §6.5 exists.

### 20.1 When rendering happens

GPUI does **not** call `Render::render` every frame. A view rebuilds its element
tree when it is notified, when an entity it depends on changes, or when the
window is invalidated. Script cost is therefore a function of interaction
frequency, not frame rate.

The case that matters is continuous interaction — dragging, scrolling, typing,
animating — which notifies at close to frame rate.

### 20.2 The cost model

```text
T_render ≈ N_nodes × (C_new + K_ops × C_op) + N_nodes × C_materialize + C_scope
```

`C_op` is one script → Rust method call including argument conversion;
`C_materialize` is pure-Rust element construction.

Under QuickJS one `C_op` is: a prototype property lookup (an ordinary lookup,
not a proxy trap — §13.2 is what buys that); one JavaScript function call; **one
rest-parameter array allocation**, because the prelude's forwarder is
`function (...args)`; one host call into Rust; and a `Value` → `Bridged`
conversion plus a `SmallVec` push.

The third item is JavaScript-specific and was not in the original cost model:
`...args` allocates an array on every call, and no-argument style methods — the
most common kind by a wide margin — pay for it and use nothing. Specialized
zero- and one-argument forwarders are the most direct optimization available and
are not implemented.

Base-first has its own cost that must be counted: presentation authority in
script means more operations per node than a `gpui-component` binding would
need, where one `.primary()` replaces five or six style calls. And because style
has exactly one expression (§13.2), there is no batching escape hatch. That
leaves three levers: reduce `C_op` itself, memoize, and virtualize.

### 20.3 Measured

| Metric | Target | Measured |
| --- | --- | --- |
| 120 Hz frame budget | 8.3 ms | Everything, including layout and paint |
| Script description under continuous interaction | **< 1.5 ms** | **QuickJS 1.14 ms, LuaJIT 1.36 ms** for 443 nodes |
| Typical panel node count | 200 – 800 | 443 in the benchmark (40 rows × 5 cells plus wrappers) |
| Operations per node, base-first | 6 – 12 | ~10 |
| Implied `C_op` ceiling | ≈ 150 ns | ~250 ns measured under QuickJS |

`tests/benchmark.rs` is the gate, and it runs on both engines from the same
description shape:

```bash
cargo test -p gpui-shell --release --test benchmark -- --nocapture
cargo test -p gpui-shell --release --no-default-features --features luajit \
    --test benchmark -- --nocapture
```

The headline result is that **QuickJS came out ahead of LuaJIT**, which is not
what §6.3's table predicts. Describing an element tree is almost entirely
cross-boundary calls and argument conversion, not the arithmetic a trace JIT is
good at, and QuickJS's prototype dispatch is competitive with an `__index`
metamethod plus a method cache. The two numbers together are what the seam was
for: they turned an engine choice from a bet into a measurement.

Two caveats. The figures are one machine's, and the *shape* — both engines
within a factor of 1.5, both under budget — is what should be read from them
rather than the digits. And the measured `C_op` is above the ceiling the budget
implies while the total is under it, which means the budget is being met with
fewer operations per node than the model assumed, not with a cheaper operation.

The in-test assertion is deliberately loose (200 ms), because the real budget is
a release-build figure and the assertion must also hold in a debug build. The
gate is the printed number, read by a person.

### 20.4 What is left

**Virtualization stays in Rust.** `VirtualList`, `Tree`, and `Table` call back
only for visible items, so a ten-thousand-row list costs the same in script as a
hundred-row one. None of them is bound yet, which makes this the largest
unrealized win on the list.

**Reduce `C_op` itself.** On the Rust side a no-argument style pushes a `u16`
into a `SmallVec`; the cost is on the JavaScript side, in the call shape and the
conversion. Specializing zero- and one-argument forwarders to avoid the rest
array, and making the `__apply` entry allocation-free, are the most valuable
changes available. **QuickJS has no JIT, so all of this is manual** — on LuaJIT
one could hope a trace would remove part of the dispatch; here one cannot.

**`gpui.memo`** would skip script construction for a subtree whose data has not
changed, reusing the previous pass's arena fragment while materialization still
runs every frame (it is the cheap, pure-Rust half). The absence of a JIT makes
its relative value higher here than it would have been on LuaJIT. Not
implemented (§8.6).

**Reuse argument objects.** The context objects handed to item renderers and
dock renderers should be pre-allocated and reused rather than built per row.

**Never let script participate** in layout, text shaping, scroll offsets,
animation interpolation, or hit testing.

### 20.5 Start-up

The start-up budget — under 2 ms for VM creation and sandbox trimming, under
5 ms for global registration including the reflection table, under 1 ms for the
palette — has not been measured, and the one line that most needs a number is
the prelude: building roughly 3,200 JavaScript closures is a one-time cost, but
it happens on the start-up path. If it turns out to be expensive, the
alternatives are caching the prototype as QuickJS bytecode or defining methods on
first use — the latter moving the cost back into `C_op`, which makes it worth
doing only if the measurement supports it.

---

## 21. Errors, Diagnostics, and Hot Reload

### 21.1 Failure is recoverable

Every Rust → script call catches at the boundary and carries the script's own
stack. `describe` flattens a QuickJS `Exception` into `message + stack`; the Lua
engine does the same with a traceback; both end as an ordinary `anyhow::Error`,
because nothing above the seam should recognize a VM's error type.

A failure during render becomes a **failure surface** where the interface should
have been. `runtime.rs` renders it: one heading, the message and stack, one
recovery line, and a copy control, on the same semantic tokens as every other
screen. Three details are deliberate. It takes its colors from tokens rather than
hardcoding red, because a failure surface that hardcodes red is unreadable in
half the themes it will be seen in; `destructive` appears once, as a hairline
rule, because emphasis is a budget and the message is already the focal point.
It has square corners, because it is not a card floating in the window — it *is*
the window's content for as long as the failure lasts. And a stack trace exists
to be pasted somewhere else, so copying it is a first-class action rather than
something the reader retypes.

The same surface serves an application that fails to load, so a failed start-up
still opens a window with the reason in it rather than only a line in a terminal
the user may not be watching.

A failure during an event is logged and the state is left alone. An unhandled
promise rejection is reported as an event-time failure (§12.2) — the
JavaScript-specific case, because without the adoption hook it would be entirely
silent.

Because there is no compile step, a reported line number is a source line
number. No source map is needed, which is one concrete benefit of refusing JSX
and TypeScript compilation.

What is missing is a toast on an event-time failure: today it is a `tracing`
line, and a host without a subscriber installed sees nothing at all — which is
why `bin/gpui-shell.rs` installs one before doing anything else.

### 21.2 Hot reload

```text
a source file changes → debounce 200 ms → re-evaluate the module →
construct a new view instance → swap the object in and notify
```

`watch::reload` does **all** of its fallible work before it touches the live
entity: re-evaluating the module can throw, and constructing the view can throw,
so both happen first and the swap is a single statement at the end. A save that
does not compile returns an error and changes nothing on screen — the previous
working view keeps running, with the error reported to the caller.

The entity survives, and so do the window, the focus, and the element
identities; only the script object behind the view is replaced. That is what
makes a reload invisible to the host.

**A reload re-reads every module, entry point included.** This is the one thing
about hot reload that had to be discovered rather than designed. QuickJS caches
an evaluated module by name and an ES module cannot be unloaded, so re-evaluating
`main.js` alone left every module it imports at the version that was on disk the
first time — a hot reload that silently ignores every file except the entry
point, which is worse than no hot reload because it looks like it worked. The
fix is a generation counter, incremented on every `load_app` and appended to
every resolved module name as `?v=N`, which makes each reload a different module
as far as the cache is concerned. The entry carries the tag too, because a
reload that re-read every import but served a stale `main.js` is the same bug one
level up. `tests/render.rs` covers exactly this.

The cost is that the previous generation stays in the cache until the runtime
shuts down. That is a development-only leak, and it is a grade coarser than the
clean form, which is to discard and rebuild the whole context — that belongs
behind the seam and is not built.

State does not survive a reload. The design routes preservation through the same
`serialize()` / `deserialize()` round trip as layout persistence, which both
saves a mechanism and continuously tests the serialization; that path does not
exist, so `watch.rs` does not invent a second one. The new instance starts from
its constructor, and the swap is one statement precisely so the restore can be
inserted before it.

`SourceWatcher` polls rather than subscribing, because the crate deliberately
takes no dependency on `notify` — the host injects a watcher. Polling is honest
for the job: a 250 ms tick, one `stat` per source file in a directory that holds
a handful, bounded at depth 8 and 4,096 files so a symlink farm or a vendored
tree cannot turn one poll into an unbounded walk. The stamp is three aggregates —
newest modification time, file count, total bytes — each covering a case the
others miss, and what it cannot see is a change that preserves all three, such as
swapping two files' names. A `notify`-based watcher would cut latency from one
poll interval to milliseconds, stop scaling with file count, and see renames;
feeding its events into `SourceWatcher::notice` is a smaller change than
replacing the type.

### 21.3 Checking, declarations, and DevTools

`gpui-shell check <directory>` is what a compiler would be for a language that
had one. The script surface is dynamic — an unknown style method, a wrongly typed
argument, a reused element are all runtime facts — so the only honest check is to
build the application and render one frame. The window is real and never shown,
because rendering is where those facts surface. It exits 0 or 1, reports syntax
errors, unresolved imports, a missing or malformed default export, unknown style
methods with a suggestion, wrongly typed style arguments, and an element used
twice. `--print-spec` additionally prints the description tree.

`gpui-shell types <directory>` writes `gpui.d.ts` (§14.4), which moves a second
class of mistake earlier still, into the editor.

There is no DevTools panel. The intended form — a debug panel written in the
script language itself, showing VM memory, live views, persistent handle count,
last frame's node count and duration, style table hits, error history, and a REPL
— is also the best available dogfood, and the REPL depends on the development-mode
`Eval` intrinsic of §19.4.

---

## 22. Testing

### 22.1 Description snapshots, with no GPU

What a script produces is a plain-data `SpecArena`, so interface structure can be
asserted without a window:

```rust,ignore
let tree = runtime.render_to_spec(&object, None, window, cx)?;
assert!(tree.contains("Button \"increment\""));
```

This is the extra return on choosing descriptions over a retained tree (§8.2),
the main regression defence for the script layer, and the vehicle for
cross-engine comparison.

### 22.2 Sandbox escape tests

`sandbox.rs` carries a set of scripts that must fail: every path back to a
compiler (seven of them, including each function-prototype constructor), writing
to a frozen prototype, `process.run` without a grant, `process.exit` without a
grant, and the interrupt-swallowing case of §19.3. `host.rs` covers the path
resolver: a read outside the granted root, a read with no grant at all, a store
call without the capability, and a clipboard denial naming the half that was
missing. Every one of these asserts on the *message*, because the message is the
instruction for fixing it.

These are security assertions and are not subject to the "avoid trivial tests"
exemption in `.claude/COMPONENT_TEST_RULES.md`.

Two of them are regression guards on the build rather than on this code:
quickjs-libc's `std` and `os` are asserted absent because `rquickjs-sys` does not
compile that file at all, and dynamic `import()` is asserted to stay callable
because confining it is the resolver's job and closing it would remove lazy
loading.

### 22.3 The shared suite across engines

Two engines cannot run the same script, so what must be equal is not the fixture
but the behavior. `tests/render.rs` carries per-engine fixtures for the same
cases and asserts the same outcomes: the counter produces the same description
shape, an element added to two parents fails with the same wording, a mistyped
style name suggests the same correction, and a real paint does not panic.
`tests/benchmark.rs` runs the same grid on both.

The rule the suite is meant to enforce is that a case exists for both engines or
CI fails, because that is the only executable definition of "the fallback is
real." It is **not** enforced, and the consequence is visible: six of the ten
tests in `tests/render.rs` are `#[cfg(feature = "quickjs")]`, covering the
bundled example, state styles, input events, and reload — every one of which is a
QuickJS-only capability. The shared core is genuinely shared; everything added
since is not.

### 22.4 Interaction tests

`overlay.rs` and `root.rs` drive `TestAppContext` and `VisualTestContext`
directly: opening and stacking dialogs, Escape unwinding one layer, focus
restoration through a stack, sheet replacement, every layer drawing at once,
toast timeout and dismissal, and phase refusal from a render pass. These are the
tests that catch a duplicated element id or a missing hitbox.

### 22.5 Relation to the repository's testing rules

Following `.claude/COMPONENT_TEST_RULES.md`: no tests assert presentation
dimensions, and coverage concentrates on complex logic — call-scope validity,
arena reuse errors, callback lifetime, value conversion, style table
non-emptiness, sandbox boundaries, overlay ordering and focus, task ownership
and cancellation, and cross-engine agreement.

---

## 23. Running an Application

```text
gpui-shell <directory> [--watch] [--dev]
gpui-shell check <directory> [--print-spec]
gpui-shell types <directory>
gpui-shell --help | --version
```

The binary is a thin host: it parses a command line, installs a log sink, builds
one runtime, opens one window, and drives the source watcher when asked.
Everything that outlives one invocation lives in the library.

The directory argument may name the application root or the `main.js` inside it,
and the root resolver handles both — along with being pointed at the *parent* of
the real application directory, which is the other common way to start. Its error
names what was expected and, where it can tell, where the application actually
is.

An unknown flag is reported rather than taken as a path, because silently
treating a mistyped `--watch` as a directory would report a missing `main.js`
instead of the typo. `--help` and `--version` are answered before anything else
can fail, since a caller who mistyped a flag is exactly the caller who needs
`--help` to work.
Usage errors exit 2; a runtime that fails to start exits 1.

**Storage is per application, under the user's data directory, keyed by the
canonical path of the application root.** The path is
`<data home>/gpui-shell/apps/<directory name>-<16 hex digits>/store.json`, where
the digits are an FNV-1a hash of the canonical root — not a security boundary,
just enough to keep two directories from sharing a folder. Keeping the directory
name in the path makes the folder recognizable; the digest disambiguates it, so
two checkouts of the same application are genuinely different installations.
`<data home>` honors `XDG_DATA_HOME`, and otherwise follows the platform
convention — `~/Library/Application Support` on macOS, `%APPDATA%` on Windows,
`~/.local/share` elsewhere.

Storage lives outside the application directory because that directory may be
read-only, is often a git checkout, and is not where a user expects their data.
When the plugin model lands, a manifest `id` should replace the digest, so an
installed plugin keeps its data across an upgrade that moves it.

Assets — the files `svg(path)` names — are served from the application directory
and nowhere else, with the same traversal check as the module resolver. Note the
asymmetry, because it surprises people: `import "./counter.js"` resolves against
the *importing file*, the way every JavaScript module system does, while
`svg("icons/check.svg")` resolves against the *application root*, the way a web
application's public directory does. The runtime cannot tell which module called
`svg`, so per-file asset paths are not available to it. A missing asset is not an
error — GPUI asks for assets it may not need — but it is warned about once per
path, saying exactly where it was looked for, because an icon that silently does
not appear is among the hardest mistakes to find.

The API version scheme — a semver for the script API, independent of the crate
version, declared by an application and checked at load — is not implemented, and
neither is any packaging or distribution format. When they are: the same version
number must offer the same capabilities and the same behavior on both engines,
and the engine belongs in neither the version number nor the manifest.

---

## 24. Alternatives, and Why They Stay Rejected

This is a record, not a debate. Each of these was decided and none should be
reopened without new information.

| Alternative | Why it is not used |
| --- | --- |
| **LuaJIT / Lua** | Not rejected — retained as a compilable fallback behind `lua`/`luajit`. It is smaller, faster in hot loops, and cheaper per call. It loses on corpus coverage and type tooling, cannot build for WebAssembly, and is restricted on W^X platforms. §20.3 measured it slower than QuickJS for the one workload that matters here |
| **The WASM component model** | Every call crosses a serialization boundary, which is the worst possible fit for high-frequency fine-grained UI calls. Heavy toolchain, poor debugging |
| **Embedding Node.js or Deno** | The process model and the size do not match an in-process, main-thread, embedded runtime. Bringing npm in brings native dependencies and a supply chain with it. VS Code's approach requires a separate extension process to work at all |
| **A pure-Rust scripting language** (Rhai, Steel, Koto) | Almost no ecosystem, a new language for every author, and a thin corpus — which is disqualifying for generated interfaces |
| **Rust dylib plugins** | No stable ABI, no sandbox, and the compile cost remains, so it solves none of §2.1 |
| **Rust hot reload** | Solves only the compile time. It does not address plugin distribution or third-party extension, and state preservation is fragile |
| **A UI DSL or JSX** | A DSL is a second language with its own parser, diagnostics, editor support, and versioning. JSX needs a compile step, which returns the "edit, save, see it" property this runtime exists for (§5.3) |
| **Object-literal element descriptions** | Exactly equivalent to the builder chain and therefore a second dialect of the same thing (§8.2) |
| **A `class("...")` style string** | Equivalent to the chain, with the weakest completion and the weakest static checking of any form available (§13.2) |
| **Automatic dependency tracking** | A second mental model beside GPUI's explicit `notify`, plus a permanent `Proxy` cost on the render path with no JIT to amortize it (§11.2) |

---

## 25. Standing Risks

| Risk | Impact | State |
| --- | --- | --- |
| **Cross-boundary call cost exceeds the budget.** Base-first raises operations per node and style has no batching form | Fatal | Measured under budget at 443 nodes (§20.3); the levers if it regresses are specialized call forms, `gpui.memo`, virtualization, and finer view granularity — and, failing all of those, the fallback engine |
| **The fallback engine rots.** A "fallback" nobody exercises silently stops compiling, and a fallback in the documentation only is worse than none | High | **Already happening.** The Lua engine has no `svg`, `Input`, `InputState`, state styles, `accessibility_label`, scheduler, host API, sandbox, or overlays. It still compiles and passes the shared core (§22.3), but the gap is widening every milestone |
| **Presentation authority in script means uneven interface quality** | High | Mitigated by the default palette and by `examples/js_todolist/ui.js` as a worked example; a shipped preset (§13.4) and a `gpui-component` module (§14.6) are the real answers |
| **Bindings drift from upstream** | High | The style surface is immune by construction; component bindings have no drift check at all (§14.5) |
| **Cycles across two collectors leak** | Medium | Per-frame callbacks are released per pass and long-lived ones are owner-bound (§7.4); there is no `gc_stats`, so a slow leak would be found by watching memory |
| **Sandbox escape**, with `Eval`, quickjs-libc, and prototype pollution as the largest surfaces | High | quickjs-libc is not compiled in; prototypes are frozen; every compiler path is closed at the JavaScript level, but the stronger intrinsic-level fix is not done (§19.1). The escape suite is real and asserts on messages |
| **Generated code assumes Node or a browser** | Medium | Named stubs point at replacements, `gpui.d.ts` moves the error into the editor. Two stub messages are wrong: `fetch` points at a `gpui.http` that does not exist, and `setTimeout` points at `gpui.timer(ms, callback)` when the API is `gpui.timer.after(ms, fn)` |
| **Interned `&'static str` accumulates** for script-registered names | Low | Bounded by loaded plugins × names each; not yet reachable, since neither actions nor panels are bound |

---

## 26. What Is Built and What Is Not

### Built

The engine seam with QuickJS as default and a compilable Lua fallback, enforced
by `compile_error!`. The render protocol: descriptions into `SpecArena`,
materialization in pure Rust, single-use enforcement, and the text-color
inheritance the description walk resolves. `CallScope` with four phases,
generation checks, and the crate's only `unsafe`. Retained state by handle, with
store-owned subscriptions, for `InputState`. The full style surface — 3,148
reflected no-argument methods, 57 hand-bound parametric ones, 9 hand-added font
weights — with Levenshtein suggestions and a two-prototype diagnostic strategy.
The default semantic palette in light and dark. Callbacks with per-pass lifetime
and generation-checked dispatch. State styles for hover, active, and focus.
Asynchrony: promises bridged to GPUI tasks, job-queue draining, `spawn`,
`sleep`, `timer.after`/`every`, `with_cx`, owner-bound cancellation, and
unhandled-rejection reporting. `ShellRoot` with the dialog stack, one sheet, the
toast stack, focus restoration, and Tab navigation. System capabilities for
`fs`, `store`, `clipboard`, `log`, and `process`, all default-denied through one
path resolver. The sandbox: module confinement, compiler withholding, frozen
prototypes, absent-global stubs, interrupt and memory limits. Hot reload with
per-generation module invalidation. `gpui.d.ts` generation from the dispatch
tables. The CLI, with `check` and `types`. The measured benchmark on both
engines.

### Not built

`gpui.memo` and every other memoization. Component bindings beyond `div`,
`text`, `svg`, `Button`, `Checkbox`, `Switch`, and `Input` — no Select, Tabs,
Tree, Table, VirtualList, Radio, Toggle, Popover, Tooltip, or Textarea, and
therefore no virtualization, which is the largest unrealized performance win.
Semantic state styles (checked, selected, disabled) with base's precedence rules.
Actions and key bindings. Animation. `gpui.theme()` and `gpui.set_theme()`.
`gpui.open_window` and multi-window applications. `gpui.http`. Native modules.
The binding table and the rustdoc-JSON drift check. Every part of the dock and
panel integration. Every part of the plugin model: manifest, contribution
registry, loader, authorization UI. Distribution, packaging, and API versioning.
`--dev`'s sandbox relaxations and the development-mode marker. The
intrinsic-level `Eval` withholding. DevTools and `gc_stats`. State preservation
across a reload. A shipped preset module. The `gpui-component` binding registry.
`console`.

---

## 27. Open Questions

1. **How thick should a preset module be?** Too thin and every author writes
   button styling from scratch; too thick and it becomes a third visual system
   in practice (§13.4). `examples/js_todolist/ui.js` — button, icon button,
   checkbox, field, label, surface, empty state — is the current answer and a
   reasonable starting scope. Whatever ships also has to be written per engine.

2. **Do `ShellRoot` and `Root` eventually merge?** Once `gpui-component` is
   bound, `ShellRoot` could delegate to `Root` and reuse its dialog, sheet, and
   notification stacks, or keep its own. `ShellRoot` has since grown decisions
   `Root` does not make — per-dialog dismissal options, only-the-topmost
   backdrop, vetoing Enter — so the merge is less obviously free than it looked.

3. **Can a script define modules other plugins can import?** Cross-plugin
   dependency brings version resolution, load ordering, and cycles. Reuse within
   one plugin only, until there is evidence otherwise.

4. **VM granularity across windows.** One VM for all windows (shared state,
   simple) or one per window (isolated, but state synchronization becomes the
   problem)? One VM is the working assumption, and it is the premise of freezing
   the built-in prototypes (§19.1). The host opens exactly one window today, so
   this is untested.

5. **What does a narrow Editor interface look like?** The full LSP, folding, and
   highlighting surface is explicitly out of scope (§14.2), but "here is the
   text, here is the language, here is the read-only flag" is worth prototyping.

6. **Where do plugin settings live?** A host settings interface driven by a
   script-declared schema (consistent) or drawn by the plugin (flexible)? The
   former, with `gpui.register_settings(schema)`, is the working preference.

7. **Where is the compatibility-stub boundary?** `setTimeout` errors and points
   at `gpui.timer`; `fetch` errors and points at a capability. What about
   `structuredClone`, `TextEncoder`, `URL`, `crypto.randomUUID`? The draft
   criterion is that anything mapping exactly may be provided and anything
   mapping approximately may not — the same rule that refuses to name the HTTP
   API `fetch` (§17.2). `console` is the live case: it maps exactly, it is a
   JavaScript author's first reflex, and it is currently a `ReferenceError`.

8. **When may the Lua engine be removed?** The proposed criterion was two
   consecutive milestones with no CI failure caught by a Lua fixture and no
   platform requiring it. The question has changed shape: the engine has fallen
   far enough behind (§25) that the real decision is whether to invest in
   restoring parity or to admit the seam's value was the measurement in §20.3 and
   retire the fallback deliberately rather than by neglect.

9. **Does the seam still pay for itself?** It was built because the engine
   choice could not be settled on paper. It has been settled, on measurement,
   in QuickJS's favor. What it still buys is discipline — 90% of the crate cannot
   name a VM — and that discipline is the thing worth keeping even if the second
   engine is not.

---

## 28. Appendices

### Appendix A: A worked example

`examples/js_todolist` is the reference application, and it exists to exercise
the whole runtime rather than to be minimal: retained input state, controlled
checkboxes, a dialog, a toast, capability-gated storage, and a filter that must
survive every mutation. It is four files — `main.js` for the view, `ui.js` for
the presentation layer, `storage.js` for persistence, `confirm.js` for the
dialog body — and a test loads and renders it, because if it stops rendering the
quickstart is wrong.

```js
import { View, h_flex, v_flex, text, InputState } from "gpui";
import { store, log } from "gpui";

export default class TodoList extends View {
  init() {
    this.draft = InputState.new({ placeholder: "What needs doing?" });
    // Enter is how a list like this is actually used; the Add button is for
    // the pointer, not the primary path.
    this.draft.on("submit", (_event, cx) => this.add(cx));
    this.items = [];
    this.filter = "all";
  }

  add(cx) {
    const caption = this.draft.value().trim();
    if (caption === "") return;
    this.items = [...this.items, { caption, done: false }];
    this.draft.set_value("");
    cx.notify();
  }

  render() {
    return v_flex()
      .size_full()
      .bg("background")
      .p(24)
      .gap(16)
      .child(this.composer())
      .children(this.items.map((item) => this.row(item)));
  }
}
```

Five things in that code are the shapes this document has been describing.

**Event handlers are always arrow functions**, because they need `this` to be
the view (§10.1).

**`children` takes an array**, so `map` is the natural list form. The Lua engine
keeps a `children(list)` shape too, but for a different reason — Lua has no
array methods — which is one example of the two engines agreeing on behavior
while differing in expression.

**`when(condition, fn)` keeps the chain in one piece**, matching the GPUI
builder style `CLAUDE.md` requires, instead of splitting into a temporary and a
sequence of `if`s:

```js
label(item.caption).when(item.done, (el) =>
  el.text_color("muted_foreground").line_through(),
)
```

**Bound methods are snake_case and the author's own are camelCase** —
`visible()`, `setFilter`, `clearCompleted` against `.items_center()`,
`.on_change()`. That contrast is §6.4's trade in real code.

**A capability that was not granted is absorbed where it is used, not checked
at every call site.** `storage.js` wraps `store` in try/catch and the interface
says "Not saved" rather than failing:

```js
export function save(items) {
  try {
    store.set(KEY, items);
    return true;
  } catch (error) {
    log.warn(`todolist: could not save (${error.message})`);
    return false;
  }
}
```

One correction to the shipped example: `main.js` opens its confirmation dialog
with `cx.open_dialog(ConfirmClear, { count, onConfirm })`, which throws, because
those are not dialog options. The correct call passes them through `props`:

```js
cx.open_dialog(ConfirmClear, { props: { count, onConfirm } });
```

### Appendix B: Crate layout

```text
crates/shell/                 # gpui-shell — depends on gpui-base + gpui only
  Cargo.toml                  # features: quickjs (default) / lua / luajit
  src/
    lib.rs                    # init, capability and storage entry points, re-exports
    engine/                   # ← the seam
      mod.rs                  #   contract, compile_error! guards, cfg forwarding
      quickjs/
        mod.rs                #   prelude, dispatch, module resolver, callbacks
        host.rs               #   fs · store · clipboard · log
        scheduler.rs          #   promises · timers · task ownership · job draining
        sandbox.rs            #   language trimming · process · limits
        overlay.rs            #   dialog · sheet · toast on the script-side cx
        entity_api.rs         #   the script face of retained state
      lua.rs                  #   the fallback engine (mlua)
    scope.rs                  # CallScope — the crate's only unsafe module
    spec.rs                   # SpecArena / SpecNode / SpecOp
    materialize.rs            # descriptions → real elements, pure Rust
    style.rs                  # reflection table + 57 parametric styles + suggestions
    theme.rs                  # the default palette and token resolution
    value.rs                  # Bridged, plus color and length coercion
    error.rs                  # ShellError
    capability.rs             # Capabilities / path resolution / denials
    entities.rs               # retained state by handle
    runtime.rs                # CallbackArena<T> · root resolution · failure surface
    root.rs                   # ShellRoot
    view.rs                   # ScriptView
    assets.rs                 # application-directory asset source
    watch.rs                  # source watching and in-place reload
    typings.rs                # gpui.d.ts generation
    bin/gpui-shell.rs         # run / check / types
  theme/default-tokens.json   # the default semantic palette, light and dark
  tests/
    render.rs                 # end-to-end description tests
    benchmark.rs              # the viability benchmark, both engines
examples/js_todolist/         # the reference application
```

### Appendix C: Naming

Following `CLAUDE.md`:

- No `Kind` suffix: `ScopePhase` rather than `ScopeKind`, `ExecuteGrant` rather
  than `CapabilityKind`, `SpecOp` rather than `SpecOpKind`.
- Public types crossing the seam keep private fields and are built with a
  builder, so adding a field is not a breaking change: `Capabilities`,
  `DialogOptions`, `ToastRequest`. An all-boolean type names its setters after
  the field and reads with `is_`/`has_` (`DialogOptions::escape_dismissable` and
  `is_escape_dismissable`); a type with non-boolean fields prefixes setters with
  `with_` (`ToastRequest::with_description`). `Capabilities` is inconsistent
  here — `with_read_roots` and `with_execute` beside a bare `store(bool)` and
  `clipboard_read(bool)` — and should be brought in line.
- `Context` is spelled out: `PanelBuildContext`, `TabGroupContext`, never
  `…Ctx`. `cx` is reserved for GPUI's `App`, `Context<T>`, and `AsyncApp`, and
  for the script-side object of the same name.
- **Rust type names above the seam carry no language**: `ScriptView`,
  `ScriptPanel`, `ScriptDockSkin` — never `JsView` or `LuaView`. They do not know
  what the language is. Types inside an engine may, because there the language is
  singular.
- Bound script method names match Rust exactly, in snake_case, with no camelCase
  renaming (§6.4). Names an author writes follow the host language's convention.

A batch of module documentation in `crates/shell/src` still describes the
runtime as a Lua one — `lib.rs`, `spec.rs`, `scope.rs`, `value.rs`,
`materialize.rs`, `style.rs`, `theme.rs`, and `capability.rs` all say "Lua" where
they mean "script". That predates the engine change and should be corrected in
one pass.

---
title: Capabilities
description: The default-deny model, the fs / store / clipboard / log / process surface, where storage lives, and what the sandbox withholds.
order: 7
---

# Capabilities

A script gets **nothing** by default. No file access, no storage, no clipboard, no process execution, no network. `Capabilities::default()` is the empty set, and that is asserted in the code rather than described in a comment.

The host grants what it grants, because only the host knows how far it trusts the code it is about to run. Every entry point re-reads the grant at call time, so revoking a capability takes effect on the next call rather than on the next restart.

```rust
gpui_shell::set_capabilities(
    Capabilities::new()
        .with_read_roots([application_root.clone()])
        .with_write_roots([data_directory.clone()])
        .store(true),
);
```

## What a locally run application is granted

Running a directory from the command line is an explicit act of trust — the same as `node app.js` — so `gpui-shell <directory>` grants a specific, narrow set:

| | |
| --- | --- |
| Read | The application directory, and its own storage directory |
| Write | Its own storage directory |
| Storage | Granted |
| Clipboard | **Not** granted |
| Process execution | **Not** granted |
| Network | **Not** granted |

An application can therefore read its own sources and assets and use its own storage, and nothing else. It is deliberately narrower than "everything", because an installed plugin will one day run through the same code path with a manifest deciding instead — and a grant that is generous for a local run would be the wrong default to inherit.

## Refusals name the fix

Every denial ends in the thing to declare, not just the fact of the refusal:

```text
filesystem read is not granted; declare capabilities.fs.read in the manifest
```

```text
`/etc/passwd` is outside every granted read root;
add its directory to capabilities.fs.read in the manifest
```

```text
storage is not granted; set capabilities.store to true
```

```text
running `git` is not granted; add it to capabilities.fs.execute in the manifest
```

::: warning There is no manifest yet
The messages name manifest keys because that is where grants will come from when the plugin model lands. Today the host calls `gpui_shell::set_capabilities` directly, and the key names are the vocabulary that API will keep.
:::

## `fs`

```js
import { fs } from "gpui";
```

| Call | Returns |
| --- | --- |
| `fs.read_text(path)` | The file's contents |
| `fs.write_text(path, contents)` | — |
| `fs.read_dir(path)` | `[{ name, is_dir }]`, sorted by name |
| `fs.exists(path)` | `true` / `false` |
| `fs.remove(path)` | — |
| `fs.create_dir_all(path)` | — |

A relative path resolves against a granted root; an absolute one must already be inside one. Every path in the surface goes through **one resolver**, which normalizes it and then requires the result to still be under a root — so `../../etc/passwd` is rejected before it reaches the filesystem, and there is no second place for a traversal bug to hide.

Three of these behave in a way worth stating, each for the same reason:

**A denied path throws rather than answering `false`.** "You may not look" and "it is not there" are different facts, and collapsing them would let a script map the filesystem outside its roots one boolean at a time.

**`remove` is not recursive.** Write access is granted per root, so a recursive remove would turn one mistyped path into the loss of an application's whole data directory. A script that means it can walk the tree itself.

**`read_dir` is sorted.** A script that renders a listing should not have to sort it, and should not inherit the filesystem's arbitrary order.

::: warning These calls block
The filesystem surface is **synchronous** today: it returns a value rather than a promise, and it blocks the thread that renders. Making it asynchronous is planned, and it will change these signatures. Keep the amount of work small, and do not read a file from `render`.
:::

## `store`

Key–value storage that survives a restart.

```js
import { store } from "gpui";

store.set("todolist.items", items);
const saved = store.get("todolist.items");   // null when the key is unset
store.remove("todolist.items");
store.keys();
store.flush();
```

Values are JSON: `null`, booleans, numbers, strings, arrays and plain objects. Functions and `undefined` properties are dropped exactly as `JSON.stringify` drops them, so the mental model transfers. `NaN` and `Infinity` have no JSON form and are refused rather than silently becoming `null`. Nesting is capped at 64 levels, which no real configuration reaches and a reference cycle exceeds immediately.

Values are cached in memory, because `get` is reachable from `render` and a file read per render would be absurd. **Every mutation persists immediately**, written to a temporary file and renamed over the target — so a crash mid-write leaves the previous settings intact rather than a truncated file. `flush` therefore does not need to be called; it stays in the API as the durability barrier for when the write becomes a promise you can await.

### Where storage lives

Storage is per application, and the host chooses the location — an application cannot name its own, or two applications could collide on purpose.

For a local run the identity is the **canonical path of the application directory**, so the same directory always reaches the same data and two directories never collide, including two checkouts of the same application, which are genuinely different installations. The path is:

| Platform | Location |
| --- | --- |
| Linux and other Unix | `$XDG_DATA_HOME/gpui-shell/apps/<name>-<digest>/store.json`, defaulting to `~/.local/share` |
| macOS | `~/Library/Application Support/gpui-shell/apps/<name>-<digest>/store.json` |
| Windows | `%APPDATA%\gpui-shell\apps\<name>-<digest>\store.json` |

`<name>` is the application directory's name, kept so the folder is recognizable; `<digest>` is a short hash of the full path, there only to disambiguate. It lives under the user's data directory rather than inside the application, because an application directory may be read-only, is often a git checkout, and is not where a user expects their data to be.

### Degrading when it is not granted

Storage that has not been granted throws, and a well-written application treats that as a fact about its host rather than an error:

```js
// storage.js — from the bundled example
import { store, log } from "gpui";

export function load() {
  try {
    const saved = store.get(KEY);
    return Array.isArray(saved) ? saved : [];
  } catch (error) {
    log.warn(`todolist: storage unavailable, starting empty (${error.message})`);
    return [];
  }
}
```

The example's footer then says so on screen — "Not saved — this host did not grant storage, so the list lasts for this run only" — which is the right shape: absorb the refusal at the boundary, and tell the user the truth.

## `clipboard`

```js
import { clipboard } from "gpui";

clipboard.write_text("copied");
const text = clipboard.read_text();   // undefined when the clipboard holds no text
```

Read and write are **separate grants**, and a denial names the half that is missing:

```text
writing the clipboard is not granted; declare capabilities.clipboard.write in the manifest
```

The clipboard needs a live host call — GPUI's `App` only exists for the duration of one — so calling it from a module's top level reports that plainly instead of panicking:

```text
clipboard.read_text() needs a live host call; call it from render, an event handler or a task
```

## `log`

```js
import { log } from "gpui";

log.info("loaded", count, { source: "disk" });
log.warn("could not save");
```

`debug`, `info`, `warn` and `error`. **No capability is required**: a script that can run can already say something, and denying it would cost the author their diagnostics and nothing else.

Extra arguments are appended space-separated, the way `console.log` behaves. Structured values print as JSON, because that is what an author reading a log wants to see.

Output goes through `tracing` with the target `gpui_shell::script`, so script output is separable from host output in a log filter. **A host with no `tracing` subscriber installed discards all of it** — along with the runtime's own reports of throwing handlers, unhandled rejections and illegal-phase calls. The `gpui-shell` binary installs a stderr sink at `INFO`, or `DEBUG` under `--dev`.

## `process`

```js
import { process } from "gpui";   // also available as a bare global

const code = process.run("git", ["status"]);
process.exit(0);
```

`process.run` is gated on an execute grant, which is one of three: denied (the default), an allowlist of command names, or unrestricted.

`process.exit` is **a request, never `exit(2)`**. It records the code; the host polls for it after a script call returns and decides what to do — close the plugin's panel, close the window, or ignore it. One plugin must not be able to take the host process down, and the host may have unsaved state.

The name is a deliberate collision. `process` is what a JavaScript author — or a model generating JavaScript — reaches for, so the runtime puts its own capability-gated surface there rather than leaving the name free to look like Node's and behave differently.

::: warning `process.exit` is gated on the wrong key
`process.exit` currently requires a filesystem grant (`capabilities.fs`) rather than a grant of its own, and its refusal says so. That is an artefact of the capability set not yet having an entry for it.
:::

## The sandbox

Beyond the capability grants, the runtime trims the language itself. All of it applies **unless development mode is on**.

**No dynamic code.** `globalThis.eval` is deleted outright — a `ReferenceError` cannot be mistaken for a working `eval` by feature detection, which a throwing stub could be. All four function compilers are replaced: `Function`, and the constructors reachable through `(async function(){}).constructor`, `(function*(){}).constructor` and the async-generator equivalent. `Function` is *replaced* rather than deleted, keeping the real `Function.prototype`, so `x instanceof Function` and `.call` / `.apply` / `.bind` keep working and only construction throws.

**Frozen built-in prototypes.** `Object`, `Array`, `Function`, `String` and `Number` prototypes are frozen. One VM will host several plugins, which makes those prototypes shared mutable state: one plugin adding an enumerable property to `Object.prototype` changes `for...in` for every other plugin and for the runtime's own prelude. The cost is real — a library that patches `Array.prototype` stops working, at import time — so a host that knowingly runs one can turn the freeze off and keep every other part of the sandbox.

**Module resolution is confined to the application root.** `import "./ui.js"` resolves relative to the importing file; anything that resolves outside the application directory is refused. Dynamic `import()` stays callable on purpose — it is how lazy loading will work — and is confined by the same resolver.

**Resource limits**, so a runaway script reports rather than taking the window with it:

| Limit | Value |
| --- | --- |
| Heap | 256 MiB — a leak becomes a catchable JavaScript exception, not an OOM kill |
| Interpreter stack | 1 MiB — deep recursion becomes a `RangeError`, not a native stack overflow |
| Time in one call: render and layout | 50 ms |
| Time in one call: event and task | 500 ms |
| Time in one call: outside any call, such as module evaluation | 5 s |

The clock restarts on every host call, which is what lets the render path have a tighter budget than an event handler. **The interrupt cannot be swallowed by a `catch` block** — that is measured by a test, because if it could be, the interrupt would not be a defence at all.

There is no `std` and no `os`: quickjs-libc is not compiled into the build in the first place.

::: warning Development mode is not wired up
`--dev` currently enables source watching only. The relaxations it is meant to turn on — restoring `eval` and leaving the built-in prototypes writable, which a REPL needs — are not reachable from the binary yet; it prints a warning saying so. The library function exists (`gpui_shell::set_development_mode`) and must be called before the runtime is constructed, because the policy is read when the context is created.

Development mode never relaxes capability gating. It makes the language easier to poke at; it does not hand out access nobody declared, because a grant the author never wrote down is a grant that will be missing in production.
:::

## Not there yet

- **`gpui.http`.** The capability model has `capabilities.network.hosts` and `fetch`'s refusal message names it, but there is no HTTP surface.
- **The manifest, and the plugin model it belongs to.** Grants come from the host today.
- **Asynchronous `fs` and `store.flush`.** Both block.
- **A capability of its own for `process.exit`.**
- **Prompting the user.** Grants are decided before the application loads; nothing asks at the moment of use.

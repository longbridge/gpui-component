# GPUI Shell LLRT Standard Runtime Design

## Status

Approved direction; implementation has not started.

Both GPUI Shell and the LLRT modules integrated by this design are experimental.
Until GPUI Shell reaches a stable release, its script interfaces, module
compatibility, capability model, and module behavior may change between minor
versions.

## Objective

Add the following LLRT-backed JavaScript facilities to GPUI Shell without
weakening its existing render isolation, per-runtime lifecycle, or capability
model:

- `llrt_buffer`
- `llrt_path`
- `llrt_url`
- `llrt_zlib`
- `llrt_crypto`
- `llrt_console`
- `llrt_os`
- `llrt_process`
- `llrt_fetch`
- `llrt_net`
- `llrt_fs`

The result is a Standard Runtime installed into each existing `ShellRuntime`.
It is not a second LLRT VM and does not move script execution away from GPUI
Shell's QuickJS runtime or scheduler.

## Constraints

1. GPUI Shell Core remains responsible for UI, VM lifecycle, scheduling,
   errors, module loading, and capability enforcement.
2. Standard Runtime modules execute in the same QuickJS context as `gpui`.
3. No LLRT module may obtain ambient filesystem, network, process, environment,
   or host-exit authority.
4. Authority is selected from the active Shell call scope. Process-wide static
   allowlists are not sufficient because several runtimes and policies may
   coexist.
5. Async work is owned by the runtime and, unless explicitly detached, by the
   view that started it. Runtime/view teardown cancels physical I/O where the
   underlying operation supports cancellation.
6. The handwritten JavaScript implementations of `fs`, `process`, and
   `console` are replaced rather than retained beside LLRT. Their authority and
   side effects continue to use Shell-owned adapters.
7. LLRT is pinned to an exact upstream Git revision whose crates use
   `rquickjs 0.12`. The published `0.8.1-beta` crates use `rquickjs 0.11` and
   cannot share GPUI Shell's `Ctx`, `Value`, or `ModuleDef` types.

## Architecture

```text
JavaScript
    |
    +-- gpui ----------------------------------+
    |                                         |
    +-- Web / node-compatible modules         |
              |                               |
              v                               v
       Standard Runtime                   Shell Core
       + pure LLRT modules                + VM/context
       + Shell adapters                   + GPUI scheduler
       + privileged adapters              + lifecycle
              |                           + error surface
              +--------------+----------------+
                             |
                             v
                       active Policy
                    fs / network / process
```

The Standard Runtime is a deep module. Its external interface is the set of
JavaScript globals and import specifiers it installs. LLRT-specific types,
initialization order, aliases, and dependency modules stay inside its
implementation.

The module loader composes three resolvers/loaders in this order:

1. Standard Runtime built-ins, including canonical and `node:` names;
2. the existing `gpui` built-in;
3. application modules confined to the application root.

An application file can never shadow a built-in module. Unknown bare imports
remain errors; this design does not add npm or Node package resolution.

## Module Groups

### Pure and data modules

The first group is installed from LLRT with only compatibility wrappers:

- `buffer` / `node:buffer`, including global `Buffer`;
- `path` / `node:path`;
- `url` / `node:url`, including the required URL globals;
- `zlib` / `node:zlib`, using the pure-Rust compression backend;
- `crypto` / `node:crypto` and Web Crypto, using one explicitly selected
  provider.

Their transitive LLRT support crates are implementation details. Each module is
tested through JavaScript imports and observable results rather than through
LLRT internals.

### Host compatibility adapters

`console`, `os`, and `process` use LLRT-compatible JavaScript shapes while Shell
owns authority and side effects. The old handwritten `console` and `process`
JavaScript bindings are removed; they do not survive as aliases with subtly
different behavior.

- Console output routes to the existing `gpui_shell::script` tracing target.
  LLRT must not create a second logging destination.
- OS exposes an explicit safe subset of read-only platform data. Home and temp
  directories resolve to policy-selected application locations, not ambient
  host locations. Resource-heavy system statistics are not enabled initially.
- Process exposes safe metadata, a filtered environment, virtual working
  directory, `nextTick`, and `run`. Although `run` keeps the existing Shell
  adapter's async bounds and physical cancellation, its JavaScript-facing
  implementation moves into the LLRT-compatible process module. `exit` remains
  a host request. LLRT's direct `std::process::exit`, signal delivery, uid/gid
  mutation, and unrestricted environment access are never registered.

The standard `process` global and `node:process` module are authoritative.
`gpui.process` is removed from the generated interface rather than maintaining
a second process surface. Applications must migrate to the standard interface.

### Privileged adapters

LLRT's JavaScript-compatible shapes for FS, Fetch, and Net sit over Shell-owned
adapters. Their upstream ambient implementations are not registered. The old
handwritten `gpui.fs` JavaScript binding is removed once the capability-backed
LLRT FS subset covers its existing operations; there is no long-lived dual
surface.

#### Filesystem

Every operation resolves against the active Policy and uses capability-scoped
directory handles. This applies to synchronous and asynchronous calls,
`FileHandle`s, metadata, links, rename, recursive mutation, and any future
streaming interface. Cross-capability rename is rejected unless the adapter can
prove both endpoints are granted. Symlink containment remains mandatory.

The initial interface may implement only the LLRT/Node subset for which the
adapter can preserve these invariants. Unimplemented exports fail explicitly;
they never fall through to `std::fs` or `tokio::fs`.

#### Fetch

The active Policy authorizes every initial URL and redirect target. Requests
have bounded connect/overall timeouts, bounded response bodies for buffering
methods, and runtime/view cancellation. Network errors and limits reject the
JavaScript promise without crashing the host. The client must not rely on
LLRT's process-wide `OnceLock` allow/deny lists.

#### Net

Raw sockets require a distinct capability from HTTP fetch. Connect and listen
are separately authorized. Socket ownership, listener lifetime, queued bytes,
and pending accepts are bounded and tied to the Shell task registry. No network
capability means the module may be importable for compatibility, but every
authority-bearing operation is denied.

## Dependency Strategy

Use one exact LLRT Git revision across every LLRT crate. Start from
`llrt_modules` with default features disabled and an explicit feature list for
pure modules, but do not enable upstream `fs`, `fetch`, `net`, or `process`
implementations when they bypass Shell adapters.

If LLRT's public interfaces do not permit an adapter, maintain a minimal patch
set under Cargo `[patch]` or contribute the seam upstream. Do not copy entire
modules into GPUI Shell. Every patch must be documented with its upstream issue
or pull request and covered by a black-box regression test.

The integration must compile on macOS, Linux, and Windows. Crypto, TLS, and
compression providers must be selected explicitly so platform defaults cannot
silently change the dependency or linking model.

## Async Integration

LLRT modules commonly assume Tokio and `rquickjs` future spawning, while GPUI
Shell owns promise adoption, bounded job draining, cancellation, and view
invalidation. Standard Runtime async operations therefore enter through the
existing Shell scheduler rather than creating an independent Tokio runtime on
the UI thread.

A background adapter may use Tokio-compatible primitives internally, but it
must return completion through Shell task ownership. No QuickJS value crosses
threads. Shutdown drops the completion route, cancels physical work when
possible, and never resumes a destroyed runtime.

## Errors and Compatibility

- Invalid arguments, denied capabilities, unavailable exports, I/O failures,
  timeouts, and cancellation become JavaScript errors or promise rejections.
- Script failures never panic or terminate the host.
- Standard names follow the LLRT-supported subset and are documented as partial
  compatibility, not Node.js compatibility as a whole.
- `gpui` remains the host namespace for GPUI-specific capabilities. Standard
  modules are authoritative for FS, Process, and Console, so this change
  intentionally removes the corresponding handwritten `gpui` exports.
- Generated typings describe only the installed subset and mark unsupported
  members absent rather than declaring behavior that throws at runtime.

## Delivery Stages

### Stage 1: Pure modules

Install Buffer, Path, URL, Zlib, and Crypto; compose the loader; add aliases,
globals, typings, black-box JavaScript tests, and startup/binary-size baselines.

### Stage 2: Host adapters

Install Console, safe OS, and safe Process interfaces over Shell adapters;
remove the handwritten Console and Process JavaScript bindings and migrate the
examples and typings. Add tests proving host exit, unrestricted environment
access, signals, and identity mutation remain unavailable.

### Stage 3: Privileged adapters

Install capability-gated FS, Fetch, and Net subsets; remove the handwritten FS
JavaScript binding and migrate examples and typings. Add policy isolation,
redirect, traversal/symlink, limit, cancellation, teardown, and multi-runtime
tests before enabling each capability in the standalone host.

Stages are implemented sequentially, but all three are in scope. A stage does
not weaken an invariant merely to unblock the next one.

## Verification

Each stage must pass:

1. JavaScript black-box tests that import and exercise every exposed module;
2. denied-capability tests as well as allowed-capability tests;
3. two-runtime tests with different policies;
4. runtime and owner teardown tests with pending async work;
5. existing render, snapshot, process, filesystem, plugin, and host-interface
   tests;
6. formatting, clippy for `gpui-shell`, and release tests;
7. startup-time, memory, and release-binary-size comparison against the current
   baseline.

Security tests must demonstrate absence of ambient access, not only successful
access through the adapter.

## Acceptance Criteria

- All eleven requested module families have a supported, documented JavaScript
  surface.
- No privileged LLRT implementation bypasses the active Shell Policy.
- Existing async process bounds and physical cancellation remain intact.
- Several Shell runtimes can use different permissions in one process.
- Dropping a view/runtime cannot leave resumable QuickJS work or owned OS
  resources behind.
- Existing examples and first-party scripts are migrated to the standard
  interfaces. Removing `gpui.fs` and `gpui.process` is an accepted experimental
  compatibility break and is called out in release notes.
- GPUI Shell is visibly marked Experimental in architecture and crate-level
  documentation.
- The exact LLRT revision, enabled features, compatibility subset, and local
  patches are documented.

## Explicit Non-goals

- full Node.js compatibility;
- npm or Node package resolution;
- LLRT's Lambda runtime, AWS SDK bundles, or test runner;
- unrestricted `child_process`, signals, identity changes, raw environment, or
  ambient filesystem/network access;
- a second JavaScript VM or scheduler;
- claiming process-level sandboxing for untrusted code.

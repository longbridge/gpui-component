# GPUI Shell LLRT Standard Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the handwritten FS, Process, and Console JavaScript surfaces and add the requested LLRT-backed Standard Runtime without bypassing Shell policy or lifecycle ownership.

**Architecture:** One Standard Runtime module composes LLRT built-ins into each existing `ShellRuntime`. Pure LLRT modules register directly; Console, Process, OS, FS, Fetch, and Net expose LLRT-compatible shapes backed by Shell-owned adapters and the active `Policy`.

**Tech Stack:** Rust, GPUI, rquickjs 0.12, LLRT 0.9.0-beta pinned by Git revision, cap-std, GPUI executors, Hyper/Rustls through LLRT.

**Spec:** `docs/superpowers/specs/2026-08-25-shell-llrt-standard-runtime-design.md`

## Global Constraints

- Keep one QuickJS context and the existing GPUI scheduler per `ShellRuntime`.
- Pin every LLRT dependency to one exact Git revision using `rquickjs 0.12`.
- Never register upstream ambient FS or Process implementations.
- Never use LLRT process-wide network allow/deny lists as the Shell permission model.
- No QuickJS value crosses threads; all async completion returns through Shell task ownership.
- Preserve the 30-second process timeout, 8 MiB per-stream limits, kill/reap cancellation, FS read bound, symlink containment, and per-call active Policy lookup.
- Treat removal of `gpui.fs` and `gpui.process` as an accepted experimental compatibility break and migrate first-party scripts.
- Support macOS, Linux, and Windows; Unix-only process tests remain cfg-gated.

---

### Task 1: Pin LLRT and compose Standard Runtime modules

**Files:**
- Modify: `crates/shell/Cargo.toml`
- Create: `crates/shell/src/engine/quickjs/standard/mod.rs`
- Create: `crates/shell/src/tests/standard_runtime.rs`
- Modify: `crates/shell/src/tests.rs`

**Interfaces:**
- Produces: `standard::builtins() -> (impl Resolver, impl Loader)` and `standard::install(&Ctx<'_>) -> rquickjs::Result<()>`.
- Consumes: existing `ShellRuntime`, `GpuiModule`, and `AppModules` loader composition.

- [ ] **Step 1: Add a failing JavaScript black-box test**

Load a script importing `Buffer`, `node:path`, `node:url`, `node:zlib`, and `node:crypto`; render exact round-trip results for UTF-8 bytes, path joining, URL parsing, deflate/inflate, and SHA-256.

- [ ] **Step 2: Verify the imports fail before integration**

Run: `cargo test -p gpui-shell --test standard_runtime --release -- --nocapture`

Expected: failure resolving the first LLRT module.

- [ ] **Step 3: Add pinned selective dependencies**

Use `llrt_modules` with default features disabled and only `buffer,path,url,zlib,crypto,compression-rust,crypto-rust`; record the exact revision in a Cargo comment. Enable the `rquickjs` features required by the selected LLRT modules without switching to `AsyncRuntime`.

- [ ] **Step 4: Implement loader composition and global initialization**

Build LLRT's resolver/loader once per `ShellRuntime`, compose it ahead of `gpui` and `AppModules`, and attach LLRT globals during `ShellRuntime::new`. Preserve the same composition when `load_app` replaces the application resolver.

- [ ] **Step 5: Run the focused and existing loader tests**

Run: `cargo test -p gpui-shell --release standard_runtime -- --nocapture`

Expected: pure-module test passes and application reload tests remain green.

- [ ] **Step 6: Commit**

```bash
git add crates/shell/Cargo.toml Cargo.lock crates/shell/src/engine/quickjs/standard crates/shell/src/tests.rs crates/shell/src/tests/standard_runtime.rs
git commit -m "feat(shell): add LLRT standard modules"
```

### Task 2: Replace Console with the LLRT-compatible surface

**Files:**
- Create: `crates/shell/src/engine/quickjs/standard/console.rs`
- Modify: `crates/shell/src/engine/quickjs/host.rs`
- Modify: `crates/shell/src/engine/quickjs/sandbox.rs`
- Modify: `crates/shell/src/tests/host_api.rs`

**Interfaces:**
- Produces: `console::install(&Ctx<'_>) -> rquickjs::Result<()>` routing `debug/log/info/warn/error` to `gpui_shell::script`.
- Consumes: the existing value formatter and tracing target currently used by `gpui.log`.

- [ ] **Step 1: Add tests for the replacement contract**

Assert `console` exists globally, `console.log` accepts mixed values without throwing, and `gpui` no longer exports a second console implementation.

- [ ] **Step 2: Run the focused tests and observe the old binding**

Run: `cargo test -p gpui-shell --release host_api -- --nocapture`

- [ ] **Step 3: Move formatting and tracing behind the Standard Console installer**

Register the LLRT-compatible Console global while retaining the existing tracing target and non-panicking value formatting. Delete the handwritten global aliases from sandbox installation.

- [ ] **Step 4: Verify**

Run: `cargo test -p gpui-shell --release host_api -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add crates/shell/src/engine/quickjs/standard/console.rs crates/shell/src/engine/quickjs/host.rs crates/shell/src/engine/quickjs/sandbox.rs crates/shell/src/tests/host_api.rs
git commit -m "refactor(shell): replace console with standard runtime"
```

### Task 3: Replace Process and add the safe OS subset

**Files:**
- Create: `crates/shell/src/engine/quickjs/standard/process.rs`
- Create: `crates/shell/src/engine/quickjs/standard/os.rs`
- Modify: `crates/shell/src/engine/quickjs/sandbox.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Modify: `crates/shell/src/tests/process.rs`
- Create: `crates/shell/src/tests/os.rs`

**Interfaces:**
- Produces: global `process`, `node:process`, `node:os`; `process.run(command, args?) -> Promise<{code,stdout,stderr}>`.
- Consumes: existing `crate::process::run`, `ExitRequest`, scheduler cancellation hooks, active Policy and application identity paths.

- [ ] **Step 1: Migrate tests to standard imports and add denial tests**

Change probes to use global `process` or `import process from "node:process"`. Assert `process.run` preserves limits; `process.exit` produces a host request; raw host environment, `kill`, `setuid`, and `setgid` are absent. Assert OS temp/home paths do not reveal ambient host paths.

- [ ] **Step 2: Verify the migrated tests fail**

Run: `cargo test -p gpui-shell --release process os -- --nocapture`

- [ ] **Step 3: Install the safe standard Process and OS adapters**

Move the current bounded Process functions out of `sandbox.rs` into the Standard Runtime. Construct LLRT-compatible metadata from explicit Shell values, expose a filtered environment object, implement `cwd` as the application virtual root, and expose only safe OS read methods.

- [ ] **Step 4: Remove `process` from the `gpui` module**

Delete it from `MODULE_EXPORTS`, `GpuiModule::evaluate`, generated type declarations, and sandbox unavailable-name handling. Do not remove the Rust process execution adapter.

- [ ] **Step 5: Verify process mechanics and JS behavior**

Run: `cargo test -p gpui-shell --release process -- --nocapture`

Run: `cargo test -p gpui-shell --release os -- --nocapture`

- [ ] **Step 6: Commit**

```bash
git add crates/shell/src/engine/quickjs/standard/process.rs crates/shell/src/engine/quickjs/standard/os.rs crates/shell/src/engine/quickjs/sandbox.rs crates/shell/src/engine/quickjs/mod.rs crates/shell/src/tests/process.rs crates/shell/src/tests/os.rs crates/shell/src/typings.rs
git commit -m "refactor(shell): replace process with safe standard adapter"
```

### Task 4: Replace FS with a capability-backed standard module

**Files:**
- Create: `crates/shell/src/engine/quickjs/standard/fs.rs`
- Modify: `crates/shell/src/engine/quickjs/host.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Modify: `crates/shell/src/tests/fs.rs`
- Modify: `crates/shell/src/tests/host_api.rs`

**Interfaces:**
- Produces: `fs`, `node:fs`, `fs/promises`, and `node:fs/promises` built-ins backed exclusively by `Capabilities::resolve` and `cap_std::fs::Dir`.
- Consumes: the existing read/write/list/exists/mkdir/remove operations and `scheduler::blocking`.

- [ ] **Step 1: Migrate FS probes to standard module names**

Use `import * as fs from "node:fs/promises"` with `readFile`, `writeFile`, `readdir`, `access`, `mkdir`, `unlink`, and `rmdir`. Add tests showing an ungranted absolute path and a symlink escape are rejected.

- [ ] **Step 2: Verify imports fail**

Run: `cargo test -p gpui-shell --release fs -- --nocapture`

- [ ] **Step 3: Implement the capability-backed module**

Adapt Node/LLRT option and result shapes to the existing bounded operations. Export only implemented members. Keep the 64 MiB read ceiling, denial-at-call behavior, empty-directory deletion rule, background syscall execution, and scoped directory handles.

- [ ] **Step 4: Remove `gpui.fs`**

Delete the export and object installation after all existing operations are reachable through `node:fs/promises`. Keep shared Rust helpers private to the Standard FS adapter.

- [ ] **Step 5: Verify FS and capability regression suites**

Run: `cargo test -p gpui-shell --release fs -- --nocapture`

Run: `cargo test -p gpui-shell --release capability -- --nocapture`

- [ ] **Step 6: Commit**

```bash
git add crates/shell/src/engine/quickjs/standard/fs.rs crates/shell/src/engine/quickjs/host.rs crates/shell/src/engine/quickjs/mod.rs crates/shell/src/tests/fs.rs crates/shell/src/tests/host_api.rs crates/shell/src/typings.rs
git commit -m "refactor(shell): replace fs with capability-backed module"
```

### Task 5: Add per-Policy network capabilities

**Files:**
- Modify: `crates/shell/src/capability.rs`
- Modify: `crates/shell/src/policy.rs`
- Modify: `crates/shell/src/plugin.rs`
- Modify: `crates/shell/src/typings.rs`
- Create: `crates/shell/src/tests/network_policy.rs`

**Interfaces:**
- Produces: immutable `NetworkGrant` values for HTTP requests, TCP connect, and TCP listen; exact `authorize_url`, `authorize_connect`, and `authorize_listen` checks.
- Consumes: active `scope::policy()` and manifest capability parsing.

- [ ] **Step 1: Add policy tests**

Cover denied-by-default, exact host/port allow, subdomain behavior, redirect re-check input, connect/listen separation, and two policies with different grants.

- [ ] **Step 2: Verify the types do not exist**

Run: `cargo test -p gpui-shell --release network_policy -- --nocapture`

- [ ] **Step 3: Implement immutable grants and manifest decoding**

Normalize scheme, host, and effective port once when constructing Policy. Reject malformed entries. Perform authorization from the active call scope, never process globals.

- [ ] **Step 4: Verify**

Run: `cargo test -p gpui-shell --release network_policy -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add crates/shell/src/capability.rs crates/shell/src/policy.rs crates/shell/src/plugin.rs crates/shell/src/typings.rs crates/shell/src/tests/network_policy.rs
git commit -m "feat(shell): add scoped network capabilities"
```

### Task 6: Add capability-gated Fetch

**Files:**
- Create: `crates/shell/src/engine/quickjs/standard/fetch.rs`
- Modify: `crates/shell/src/engine/quickjs/standard/mod.rs`
- Create: `crates/shell/src/tests/fetch.rs`

**Interfaces:**
- Produces: global `fetch`, `Request`, `Response`, `Headers`, and `FormData` using LLRT data classes and a Shell-owned transport entry point.
- Consumes: `NetworkGrant::authorize_url`, scheduler task ownership, runtime shutdown and body/timeout constants.

- [ ] **Step 1: Add local-server black-box tests**

Test GET and response text, denied host, redirect from an allowed URL to a denied URL, request timeout, oversized buffered body, owner teardown, and two runtimes with different policies.

- [ ] **Step 2: Verify fetch is absent**

Run: `cargo test -p gpui-shell --release fetch -- --nocapture`

- [ ] **Step 3: Implement transport integration**

Reuse LLRT Request/Response/Headers/FormData parsing where it does not perform I/O. Route every request and redirect through current Policy. Execute transport off the UI thread; resolve/reject through the Shell scheduler; cap buffered bodies and attach physical cancellation.

- [ ] **Step 4: Verify**

Run: `cargo test -p gpui-shell --release fetch -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add crates/shell/src/engine/quickjs/standard/fetch.rs crates/shell/src/engine/quickjs/standard/mod.rs crates/shell/src/tests/fetch.rs
git commit -m "feat(shell): add capability-gated fetch"
```

### Task 7: Add capability-gated Net

**Files:**
- Create: `crates/shell/src/engine/quickjs/standard/net.rs`
- Modify: `crates/shell/src/engine/quickjs/standard/mod.rs`
- Create: `crates/shell/src/tests/net.rs`

**Interfaces:**
- Produces: `net` and `node:net` module subset with policy-gated connect/listen and bounded socket ownership.
- Consumes: `NetworkGrant`, Shell scheduler cancellation, LLRT Buffer/Event/Stream support modules.

- [ ] **Step 1: Add loopback integration tests**

Cover allowed connect, denied connect, connect timeout, allowed listener, denied listener, queued-byte ceiling, socket close, runtime teardown, and policy isolation across two runtimes.

- [ ] **Step 2: Verify the module is absent**

Run: `cargo test -p gpui-shell --release net -- --nocapture`

- [ ] **Step 3: Implement the bounded socket adapter**

Authorize before DNS/connect or bind, assign every socket/listener to its runtime and view, cap queued writes and unread bytes, and make close/teardown idempotently release OS handles. Do not call LLRT global allow/deny setters.

- [ ] **Step 4: Verify**

Run: `cargo test -p gpui-shell --release net -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add crates/shell/src/engine/quickjs/standard/net.rs crates/shell/src/engine/quickjs/standard/mod.rs crates/shell/src/tests/net.rs
git commit -m "feat(shell): add capability-gated net"
```

### Task 8: Migrate typings, examples, docs, and mark Experimental

**Files:**
- Modify: `crates/shell/src/lib.rs`
- Modify: `crates/shell/src/typings.rs`
- Modify: `docs/gpui-shell.md`
- Modify: `examples/js_todolist/main.js`
- Modify: every first-party script found by `rg 'gpui.*(fs|process)|import .*fs.*from "gpui"|import .*process.*from "gpui"' examples crates`

**Interfaces:**
- Produces: generated declarations matching installed Standard Runtime exports and visible Experimental status.
- Consumes: final module compatibility surfaces from Tasks 1–7.

- [ ] **Step 1: Add typing snapshot assertions**

Assert declarations include supported globals/modules and omit `gpui.fs` and `gpui.process`.

- [ ] **Step 2: Migrate first-party JavaScript**

Use `node:fs/promises`, global or `node:process`, and global Console. Keep GPUI-only imports in `gpui`.

- [ ] **Step 3: Update architecture documentation**

Mark GPUI Shell Experimental at the top of the architecture and crate docs. Document the pinned LLRT revision, module matrix, partial compatibility, capability adapters, limits, migration, and unsupported Node behavior.

- [ ] **Step 4: Regenerate and verify declarations**

Run: `cargo test -p gpui-shell --release typings -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add crates/shell/src/lib.rs crates/shell/src/typings.rs docs/gpui-shell.md examples crates
git commit -m "docs(shell): publish experimental standard runtime contract"
```

### Task 9: Full verification and dependency audit

**Files:**
- Modify only files required by failures found below.

**Interfaces:**
- Consumes: all preceding tasks.
- Produces: verified three-platform-compatible source and recorded LLRT baseline.

- [ ] **Step 1: Format and lint**

Run: `cargo fmt --all -- --check`

Run: `cargo clippy -p gpui-shell --all-targets --all-features -- -D warnings`

- [ ] **Step 2: Run the complete Shell test suite**

Run: `cargo test -p gpui-shell --release -- --nocapture`

- [ ] **Step 3: Audit the dependency graph**

Run: `cargo tree -p gpui-shell -d`

Expected: exactly one `rquickjs`/`rquickjs-core` version and no enabled upstream ambient FS/Process adapter.

- [ ] **Step 4: Verify compilation surfaces**

Run: `cargo check -p gpui-shell --no-default-features`

Run: `cargo check -p gpui-shell --all-targets --all-features`

- [ ] **Step 5: Record release baseline**

Build `gpui-shell --release`, record executable size and startup benchmark beside the existing metrics documentation, and compare with the pre-LLRT baseline. Any regression is reported numerically rather than hidden.

- [ ] **Step 6: Check the final diff and commit fixes**

Run: `git diff --check`

```bash
git add -u crates/shell
git commit -m "test(shell): verify LLRT standard runtime"
```

# Shell Dock Review Fixes

## Scope

This change closes the five actionable findings from the review of the shell
keyboard, pointer, window, and dock API pull request:

1. every retained entity, including `DockArea`, obeys the runtime-wide live
   entity limit;
2. a programmatic dock-size change emits the layout event used by persistence;
3. the public lock contract matches base's existing rearrangement-only lock;
4. JavaScript dock arguments are rejected instead of silently truncated or
   converted into invalid geometry; and
5. unchanged dock chrome does not execute JavaScript on every frame.

Unrelated API or dock refactoring is out of scope.

## Resource-limit design

The entity limit belongs to `EntityStore`, because that is the single boundary
through which every retained record enters the store. `push` will become a
fallible operation and reject insertion at `MAX_LIVE_ENTITIES`. JavaScript
constructors will translate this failure into the existing retained-entity
`RangeError`. This removes the need to remember a separate capacity check in
each future constructor.

Tests will exercise the store boundary and the public `DockArea.new` path.

## Layout events and lock contract

`DockArea::set_dock_size` will compare the effective old and new sizes. When
they differ it will notify and emit `DockEvent::LayoutChanged`; a no-op write
will emit nothing. This keeps persistence subscribers complete without adding
duplicate saves.

Base already defines `locked` as preventing rearrangement while preserving
resize. The generated declarations and English and Chinese documentation will
state that contract. No resize behavior will change.

## JavaScript validation

The dock host boundary will validate values before narrowing them:

- versions and panel ids must be finite, non-negative safe integers;
- sizes and every bounds coordinate must be finite;
- widths and heights must also be non-negative;
- required names must be non-empty; and
- panel classes must be `View` subclasses, using the same JavaScript-side
  predicate as `cx.new`.

Invalid input throws synchronously at the API call with the argument name in
the message. It is never clamped, defaulted, or queued.

## Dock chrome cache

The cache stores descriptions, not `AnyElement`s. A cached entry contains the
temporary `SpecArena` and root `SpecId` produced by a chrome callback. Its key
identifies the hook and native container; its validity tuple contains the
callback id and the resolved JSON payload.

On a cache hit, the runtime materializes the stored description without
entering QuickJS. On a miss, it calls the handler once, records the description,
replaces that key's entry, and materializes it. A new parent snapshot naturally
invalidates entries because it supplies new callback ids. Changed native dock
state invalidates an entry because its payload differs.

The dock-content placeholder remains in the cached description. Materializing
the description on each frame consumes that frame's native dock content from
the existing scoped slot, so caching does not retain or reuse consumed GPUI
elements. Native command resolution likewise continues to use the contexts
recorded for the current frame.

Entries are bounded by the live dock structure and are cleared when chrome
slots are cleared or the owning dock/runtime is released. Errors and null
results are not cached, allowing a later frame to recover after logging the
failure.

## Verification

Regression tests will prove:

- the entity store and `DockArea.new` refuse the first entity beyond the cap;
- a changed dock size emits one layout event and a repeated value emits none;
- public declarations and documentation describe rearrangement-only locking;
- malformed numeric, bounds, name, and class arguments throw;
- drawing unchanged dock chrome repeatedly invokes the JS callback once, while
  changing its payload or callback invokes it again; and
- cached `dock_content()` still renders the current native content.

The intended verification commands are formatting, focused shell/base tests,
the full `gpui-shell` library test suite, and `git diff --check`. If the local
environment still lacks Cargo, the handoff will state exactly which checks
could and could not run rather than claiming unverified success.

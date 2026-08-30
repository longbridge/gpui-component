# JavaScript Story gallery

This is the JavaScript Story gallery scaffold: an auditable, reviewable route
catalog for the component-shell work.

The gallery imports the registered public `gpui-component` surface through the
completed public component-shell host. Deferred and infrastructure routes remain
explicit status panels rather than fabricated constructors.

## Coverage audit

The independent coverage check derives all tracked component-shell surfaces
from `crates/component-shell/component-inventory.json` and checks them against
the explicit imports, routes, and `coveredBy` metadata in `stories/coverage.js`:

```bash
node examples/js_story/fixtures/verify-coverage.mjs
```

The gallery imports only public `gpui`, `gpui-base`, and `gpui-component`
script modules. `catalog.js` explicitly imports each family module and every
route records its Rust Story source. The inventory currently supplies 63 mirrored Story
entries and 64 tracked catalog surfaces. The check fails if either side changes
without matching catalog coverage and status.

## Registration status

Registered inventory surfaces render a real public constructor and invoke a
descriptor-backed method when that descriptor exposes one. `Breadcrumb` and
`StatusBar` are constructor-only public descriptors. Deferred surfaces render
every checked surface covered by their route, with the inventory category and
reason from `stories/status.js`; the verifier checks that projection against the
inventory. This keeps a missing binding visible without adding a private Rust
host module.

`NativeMenuTrigger` provides the registered native-menu surface used by the
JavaScript story under the inventory's `platform-integration` category.

Two Rust Stories are deliberately not mirrored, and `verify-coverage.mjs` holds
the list with a reason for each: `ShellStory`, which embeds a script view inside
a Rust story and would demonstrate this gallery to itself, and
`ThemeColorsStory`, since every route already renders through the active theme.
The verifier refuses an exclusion that is not `infrastructure` in the inventory,
so the list cannot quietly hide a component that has something to show.

## Editor checking

`gpui.d.ts` is generated from the public component-shell host's declaration
API and is not hand-authored by this example:

```bash
cargo run -p gpui-component-shell --bin gpui-component-shell -- types examples/js_story
```

`jsconfig.json` enables strict JSDoc checking for the gallery and all family
modules against that generated surface.

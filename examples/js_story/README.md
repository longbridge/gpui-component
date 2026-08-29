# JavaScript Story gallery

This is the JavaScript Story gallery scaffold: an auditable, reviewable route
catalog for the component-shell work.

The gallery is not yet runnable in this checkout: its `gpui-component` adapter
bindings and executable composition are still in progress. Its routes render
explicit availability panels until a public constructor is registered.

## Coverage audit

The independent coverage check derives all renderable/platform registrations
from `crates/component-shell/component-inventory.json` and checks them against
the explicit imports, routes, and `coveredBy` metadata in `catalog.js`:

```bash
node examples/js_story/fixtures/verify-coverage.mjs
```

The gallery intentionally imports only `gpui` and `gpui-base`, both public
script modules. `catalog.js` explicitly imports each family module and every
route records its Rust Story source. The inventory currently supplies 65 Story
entries and 71 renderable/platform registrations. The check fails if either
side changes without matching catalog coverage.

## Registration status

The adapter and its generated `gpui-component` module are implemented outside
this directory. Until a component constructor is registered, its route renders
an availability panel naming the expected public export and the interactive
states that the eventual story must exercise. This is deliberate: importing a
future constructor would make every route fail to load today, and adding a
private Rust host module would hide an API-boundary violation.

`Shell` and `NativeMenu` are platform-aware entries. Their panels explain the
availability constraint instead of pretending that a platform-only control can
render everywhere.

## Editor checking

When the component-shell build and registry are available, declarations will be
generated from the host with:

```bash
cargo run -p gpui-shell -- types examples/js_story
```

That command writes `gpui.d.ts`, which is intentionally generated and not
hand-authored by this example. It cannot succeed until the adapter composition
builds. `jsconfig.json` enables strict JSDoc checking for the gallery and all
family modules once those declarations exist.

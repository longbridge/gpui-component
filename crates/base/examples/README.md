# gpui-base examples

`base_components.rs` is the single Cargo example entrypoint for every documented `gpui-base`
component. It selects one component from the shared `showcase` implementation, so native and
WebAssembly previews exercise the same Rust code without producing one binary per component.

Run an individual component natively:

```bash
cargo run -p gpui-base --example base_components -- button
cargo run -p gpui-base --example base_components -- alert-dialog
cargo run -p gpui-base --example base_components -- virtual-list
```

Run without a component slug to show the overview:

```bash
cargo run -p gpui-base --example base_components
```

The website builds `examples/wasm`, which imports the same `showcase/mod.rs` and selects the
component using the `?component=<slug>` query parameter.

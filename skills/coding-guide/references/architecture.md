# Application Architecture and Layering

Dependencies point downward. Higher layers own domain meaning and orchestration; lower layers own reusable presentation or behavior. Do not make a reusable component depend on an application screen, or make `gpui-base` depend on a theme from GPUI Component.

## Application Layers

- **app shell:** compose windows and feature crates while keeping feature logic out;
- **feature crate:** keep one capability's model, services, views, commands, dialogs, and workflow behind one public boundary;
- **app component:** a repeated domain-aware pattern;
- **gpui-component:** themed, general-purpose UI;
- **gpui-base:** reusable behavior and geometry without product presentation.

## Organize Large Applications by Capability

In a large Rust application, a feature should usually be a crate, not another file in a global `views`, `models`, or `modals` directory. Keep the model, views, commands, dialogs, and workflow for one capability together. A dialog that edits a workspace belongs to the workspace feature; only the reusable dialog primitive belongs to the UI library.

```text
crates/
├── app/
│   └── src/main.rs             # Compose windows and features
├── workspace/
│   └── src/
│       ├── lib.rs              # The feature's public boundary
│       ├── model.rs
│       ├── commands.rs
│       ├── workspace_view.rs
│       └── rename_dialog.rs
├── search/
│   └── src/
│       ├── lib.rs
│       ├── model.rs
│       ├── commands.rs
│       ├── search_view.rs
│       └── filters.rs
├── settings/
│   └── src/
│       ├── lib.rs
│       ├── model.rs
│       ├── settings_view.rs
│       └── account_dialog.rs
└── shared/
    └── src/
        ├── lib.rs
        └── recent_items.rs     # A stable capability with multiple owners
```

Do not invert this into global `models/`, `views/`, `modals/`, and `commands/` directories. Those folders classify files by implementation role while scattering every feature across the application.

## Inter-feature Communication

The application shell composes feature crates but contains little feature logic. A feature may depend on stable shared capabilities and UI foundations; it must not depend on the shell or reach into a sibling feature's internals.

When two features need to communicate:
- Prefer an explicit command, event, data type, or small shared service over a dependency between their views.
- Extract a shared crate only after the capability has a coherent name and more than one real owner.

## Crate Boundary Rules

- Crate boundaries let Cargo rebuild and test a smaller dependency subgraph, make ownership visible in `Cargo.toml`, and limit the review and regression surface of a change.
- Do not create a crate for every screen or helper. Split where a capability has its own state and lifecycle, a stable public seam, or enough implementation to benefit from independent compilation and tests.
- Keep dependencies acyclic and pointing toward smaller, more stable crates.

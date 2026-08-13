# Input module layout

The public Rust module remains `gpui_base::input`; the folders below organize
its implementation without adding public module-path segments.

- `controls/` contains the purpose-specific `Input`, `Textarea`, and `Editor`
  elements and their state facades.
- `engine/` contains the shared text-editing engine: state, layout, cursor,
  selection, movement, masking, display mapping, search, native integration,
  and painting.
- `editor/` contains capabilities specific to rich/code editing: highlighting,
  diagnostics, decorations, indentation, and LSP integration.

`mod.rs` is the external seam. Keep public re-exports there so reorganizing the
implementation does not change callers' imports.

The root uses explicit `#[path]` declarations so this organization remains an
implementation detail and existing internal module relationships stay intact.

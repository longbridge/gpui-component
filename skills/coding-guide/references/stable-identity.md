# Stable Identity and ElementId

An `ElementId` is part of behavior. It gives an element stable identity and keys element-local or component state. A component may also use it as one input to its own focus, measurement, or animation identity; focus and scrolling are otherwise owned by their dedicated handles.

## Rules for ElementId

- **Use stable domain IDs** for rows, tabs, tree nodes, and repeated controls.
- **Namespace child IDs** with their owning object when the same control repeats.
- **Never derive identity from a translated label or a mutable list index** when items can be inserted or reordered.
- **Do not generate a fresh random ID during `render`**.

```rust
Button::new(("delete-project", project.id))
    .danger()
    .label("Delete")
```

A changed ID means a changed UI identity. Treat that reset as deliberate.

## Shared Keys & Channels

The same rule applies to transition channels, overlay tokens, scroll handles, and persistence IDs.
- If two independently retained behaviors share a key, they can overwrite each other's state;
- If one behavior changes keys every frame, it never accumulates state.

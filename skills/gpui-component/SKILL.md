---
name: gpui-component
description: gpui-component project patterns and workflows. Use when creating new UI components in the gpui-component project, writing component stories/examples, generating component documentation, or following project style conventions. Covers component structure (stateless/stateful/composite), trait implementations (Sizable/Selectable/Disableable/Styled), builder pattern, callbacks, story creation, and documentation. Replaces individual new-component, gpui-style-guide, generate-component-story, generate-component-documentation skills.
---

## Navigation

Load the relevant reference file based on the task:

| Topic | File | When to load |
|-------|------|--------------|
| Creating a new component | [new-component.md](references/new-component.md) | Building new UI components, component file structure, registration |
| Component style guide | [style-guide.md](references/style-guide.md) | Code conventions, trait implementations, field organization, patterns |
| Writing stories | [story.md](references/story.md) | Creating story examples for the gallery app |
| Writing documentation | [documentation.md](references/documentation.md) | Component docs in the `docs/` folder |

## Key Concepts

- **Stateless**: `RenderOnce` + `#[derive(IntoElement)]`, no internal state (e.g. `Button`)
- **Stateful**: holds `Entity<State>`, state managed separately (e.g. `Select` + `SelectState`)
- **Composite**: built on top of other components (e.g. `AlertDialog` based on `Dialog`)
- **Callbacks**: use `Rc<dyn Fn(...)>` for multi-call handlers
- **Styling**: implement `Styled` with `StyleRefinement` + `refine_style()` in render
- **Sizes**: implement `Sizable` trait for `xs/sm/md/lg` size variants

---
name: generate-component-documentation
description: Generate documentation for new components. Use when writing docs, documenting components, or creating component documentation.
---

## Instructions

When generating documentation for a new component:

1. **Follow existing patterns**: Use the documentation styles found in the `docs` folder (examples: `button.md`, `accordion.md`, etc.)
2. **Reference implementations**: Base the documentation on the same-named story implementation in `crates/story/src/stories`
3. **API references**: Use markdown `code` blocks with links to docs.rs for component API references when applicable

## Examples

The generated documentation should include:
- Component description and purpose
- Props/API documentation
- Usage examples
- Visual examples (if applicable)

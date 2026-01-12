---
name: new-component
description: Create new GPUI components. Use when building components, writing UI elements, or creating new component implementations.
---

## Instructions

When creating new GPUI components:

1. **Follow existing patterns**: Base implementation on components in `crates/ui/src` (examples: `Button`, `Select`)
2. **Style consistency**: Follow existing component styles and Shadcn UI patterns
3. **Component type decision**:
   - Use stateless elements for simple components (like `Button`)
   - Use stateful elements for complex components with data (like `Select` and `SelectState`)
4. **API consistency**: Maintain the same API style as other elements
5. **Documentation**: Create component documentation
6. **Stories**: Write component stories in the story folder

## Component Types

- **Stateless**: Pure presentation components without internal state
- **Stateful**: Components that manage their own state and data

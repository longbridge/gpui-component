---
name: generate-component-story
description: Create story examples for components. Use when writing stories, creating examples, or demonstrating component usage.
---

## Instructions

When creating component stories:

1. **Follow existing patterns**: Base stories on the styles found in `crates/story/src/stories` (examples: `tabs_story.rs`, `group_box_story.rs`, etc.)
2. **Use sections**: Organize the story with `section!` calls for each major part
3. **Comprehensive coverage**: Include all options, variants, and usage examples of the component

## Examples

A typical story structure includes:
- Basic usage examples
- Different variants and states
- Interactive examples
- Edge cases and error states

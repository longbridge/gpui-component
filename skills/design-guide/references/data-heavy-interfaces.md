# Designing Data-Heavy Interfaces

Dense does not mean cramped. In tables, trees, command palettes, editors, and docks:

## Data Design Principles

- Keep headers and primary row identity visually stable.
- Align comparable values and use tabular numerals where appropriate.
- Clearly distinguish focus, hover, active row, and multi-selection.
- Keep sorting and filtering visible and reversible.
- Preserve selection by domain identity across filtering and reordering.
- Virtualize large collections without changing keyboard semantics.
- Use progressive disclosure for secondary columns and inspectors.
- Provide a useful empty state that explains the next action.

## Component Selection for Data

- **Table**: Comparison across consistent, structured fields.
- **List**: Scanning heterogeneous items.
- **Tree**: True hierarchical data.
- **Dock**: Arranging long-lived tools or documents (preserve container chrome and zoom state).

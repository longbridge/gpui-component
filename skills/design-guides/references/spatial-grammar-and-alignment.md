# Spatial Grammar and Alignment Spines

## Spatial Grammar

Spacing expresses relationship. Choose a gap from the semantic scale by asking what the two elements mean to each other:

| Relationship | Typical token | Current scale | Examples |
| --- | --- | --- | --- |
| Optical correction | `xxs` | 2 px | icon baseline, compact separator |
| Parts of one control | `xs` | 4 px | menu icon/label, title/description |
| Closely related controls | `sm` | 8 px | button icon/label, dialog actions |
| One content group | `md` | 12 px | notification columns, compact form rows |
| Separate groups in one section | `lg` | 16 px | panel padding, form groups |
| Separate sections | `xl` | 24 px | major blocks in a page or inspector |
| Major region boundary | `xxl` | 32 px | empty-state breathing room, page bands |

### Rules for Spacing

1. **Inside before outside.** A component's padding belongs to the component; the gap between components belongs to their parent.
2. **Vertical rhythm shows grouping.** The gap between a title and its description is smaller than the gap to the next section.
3. **Horizontal space supports scanning.** Repeated rows keep icons, labels, values, badges, and trailing actions on stable columns.
4. **Do not double padding.** A card placed in an already padded panel should not automatically add another full panel inset.
5. **Use optical alignment sparingly.** A 1–2 px correction is valid for icon/glyph geometry, but document why.

---

## Alignment Spines & Exact Tolerance

Alignment is a structural system. Establish a small set of alignment spines for each surface: shared leading/trailing edges, text baselines, center lines, and fixed functional lanes.

- **Shared content inset**: Heading, toolbar, list row, empty state, and footer at the same level should share one leading edge.
- **Repeat column geometry**: Headers, rows, summaries, and loading states reserve the same lanes for identity, metadata, status, numbers, and actions.
- **Align text by baselines** (not bounding-box centers) when mixed sizes share a row.
- **Center icons in fixed slots** so labels do not jitter when icons differ in width.
- **Right-align numbers**, left-align prose.
- **Scrollbar belongs on the boundary owner**: The scrollbar sits against the panel/editor edge. Content padding must not pull the scrollbar into the middle of the surface.
- **Rendered-pixel tolerance**: When two edges or spaces are intended to be equal, a one-rendered-pixel difference is a defect. Inspect resolved bounds with measurement tools rather than casual screenshots.

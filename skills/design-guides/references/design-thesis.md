# Design Thesis & Desktop vs Web Habits

Build interfaces that feel native, quiet, and precise. Let content, hierarchy, and interaction carry the experience; decoration should support them rather than compete with them.

## 5 Principles

1. **Clarity before personality.** Make the primary task and next action clear before adding brand expression.
2. **Composition before invention.** Start with established components and compose them into product-specific workflows. Create a new primitive only when its behavior is genuinely new.
3. **Tokens before values.** Colors, radii, typography, and spacing should form a system. Avoid isolated literals that cannot respond to themes.
4. **Desktop before web convention.** Preserve keyboard access, window chrome, menus, dense data views, resizable regions, and persistent navigation where the task benefits from them.
5. **State must be visible.** Hover, focus, selection, disabled, loading, validation, and destructive states need distinct and consistent treatment.

---

## Learning from Shadcn & Native Desktop Differences

Shadcn provides useful patterns—open code, composition, dependable defaults, and separating behavior primitives from styled layers. But do not copy web assumptions blindly:

| Web habit | Native GPUI default |
| --- | --- |
| Pointing-hand cursor on every button | Default arrow cursor; pointing hand for links |
| Page navigation as the main structure | Persistent windows, panes, sidebars, tabs, and menus |
| Browser focus and scrolling as a fallback | Explicit focus ownership and region-owned scrolling |
| Mobile-first single column | Resizable desktop shell with a defined minimum window size |
| Hover-revealed critical actions | Keyboard- and pointer-reachable actions that do not depend on hover |
| A row of hover-only icon buttons | A visible primary action plus `DropdownMenu` or `ContextMenu` for secondary commands |
| Link-styled text for application commands | `Button`, `outline`, or `ghost`; Link only for URLs, web resources, or email addresses |
| Large touch density everywhere | Medium density by default; compact only where information work benefits |
| CSS overrides across descendants | Typed builders, semantic parts, and application composition |

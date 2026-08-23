# Interaction States, Pointer Rules, and Buttons vs Links

A control should predict its result. Its label names the action and object, its state shows availability, and its feedback confirms the outcome.

## 9 Interactive States

| State | Design requirement |
| --- | --- |
| **Rest** | Clear affordance without visual noise |
| **Hover** | Subtle pointer feedback, never the only cue |
| **Pressed** | Immediate press feedback |
| **Open / pressed** | Persistent feedback while an attached popup is open |
| **Focus visible** | High-contrast keyboard focus ring |
| **Selected / checked** | Persistent state distinct from hover |
| **Disabled** | Lower emphasis and no misleading hover/pressed response |
| **Loading** | Preserve context, prevent duplicate action, explain long waits |
| **Error** | State what happened and how to recover |

---

## Pointer Conventions

- Default arrow cursor for buttons, checkboxes, menu items, tabs, and native controls.
- Pointing hand for links and content that behaves as an external link.
- Keep hover effects modest. Do not reveal the only copy of an essential action on hover.

---

## Desktop Command Surfaces vs Hover Toolbars

- Keep primary actions visible as labeled Buttons;
- Put secondary actions behind a visible `DropdownMenu` trigger;
- Put object-under-pointer commands in a `ContextMenu`;
- Expose commands via Actions/key bindings;
- Use hover icons only as shortcuts to commands reachable elsewhere.

---

## Button vs Link Semantics

- **Button means application action**:
  - `primary`: Default commit in decision area;
  - `default`: Ordinary visible actions;
  - `outline`: Clear boundary with less emphasis;
  - `ghost`: Quiet toolbar / inline actions;
  - `icon`: Well-known symbols with tooltip + accessible name.
- **Link means external resource**:
  - Reserved exclusively for external URL, web page, documentation, or email address.
  - Never use Link styling to make an in-app Delete, Save, Refresh, Add, or navigation action look quiet.
  - In-app destinations (sidebars, tabs, breadcrumbs, detail views) must use native navigation components or Button/Action.

# Events, Actions, and Focus

Use pointer callbacks for pointer-specific behavior. Use GPUI Actions for commands that should support key bindings, menus, or dispatch from multiple inputs. Keep action handlers close to the view that owns the command.

## Command Unification

Model one logical desktop command once. A toolbar Button, `DropdownMenu` item, `ContextMenu` item, menu-bar item, and key binding should dispatch the same Action or call the same owner method instead of copying five mutations. Derive their label, icon, shortcut, and enabled state from one command policy.

## Button vs Link Semantics

- Use `Button` for commands even when the desired treatment is quiet—select `outline`, `ghost`, or an icon presentation instead of replacing it with `Link`.
- GPUI Component applications reserve `Link` for targets opened by a browser or mail client (URL, web document, email address).
- Use navigation components for in-app destinations and `Button`/`Action` for commands.

## Event Propagation & Focus Rules

- Only stop propagation when a nested interaction must prevent its parent from handling the same event. Blanket propagation stops break menus, selection, dragging, and window-level commands.
- **Explicit focus ownership**:
  - Retain a `FocusHandle` in the entity that owns keyboard interaction;
  - Register `key_context` and `on_action` handlers on the same focused region;
  - Transfer focus when opening an overlay and restore it on dismissal;
  - Render a visible `focus_visible` state;
  - Do not request focus unconditionally from `render`.
- Modal surfaces must trap focus and restore the previous valid focus target on dismissal. Nested overlays dismiss from the top.

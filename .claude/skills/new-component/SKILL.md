---
name: new-component
description: How to write a new component of GPUI Component.
---

## Rules

- Based on existing components in `crates/ui/src` folder, e.g.: `Button`, `Select`.
- The UI and API follow the existing components style, we also based on Shadcn UI style.
- If the new component are simple, we'd like to use stateless element like the Button.
- If the new component are complex with data, we'd like to use stateful element like the Select and SelectState.
- Keep same API style like other elements.
- Write a new component story in story folder.
- Write document.

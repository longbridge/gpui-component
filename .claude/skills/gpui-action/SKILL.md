---
name: gpui-action
description: Describes Action definition and keyboard shortcut binding in GPUI.
---

## Overview

Actions in GPUI provide a declarative way to handle keyboard-driven UI interactions. They decouple user input (key presses) from application logic, enabling customizable keybindings and consistent behavior across different UI contexts. Actions can be simple unit structs or complex types with data, and they integrate with GPUI's focus system for context-aware behavior.

## Action Definition

### Simple Actions

Use the `actions!(:namespace, [Action1, Action2 ...])` macro for simple actions without data:

```rust
use gpui::actions;

// Define actions in a namespace
actions!(editor, [MoveUp, MoveDown, MoveLeft, MoveRight, Newline, Save]);

// Or without namespace
actions!([Quit, OpenFile, CloseWindow]);
```

This generates:
- Unit structs for each action (`MoveUp`, `MoveDown`, etc.)
- Registration with GPUI's action system
- Automatic `Clone`, `PartialEq`, `Default`, and `Debug` implementations
- The `namespace` argument used for find Keyings for example `editor::MoveUp`, `editor::Save`, etc. It not a namespace of Rust module just a logical grouping for actions.

Use simple actions for:

- Perfer to named by use verb-noun pattern (e.g., OpenFile, CloseWindow)  
- Basic commands (Save, Copy, Undo)
- Navigation actions (MoveUp, MoveDown)
- Toggle actions (ToggleFullscreen, ToggleSidebar)

### Complex Actions with Parameters

For actions that need to carry data we can define action like this:

```rust
use gpui::{Action, actions};

#[derive(Clone, PartialEq, Action, Deserialize)]
#[action(namespace = editor)]
pub struct SelectRange {
    pub start: usize,
    pub end: usize,
}
```

The `#[derive(Clone, PartialEq, Action, Deserialize)]\n#[action(namespace = ...)]` is required for complex actions.

## Keybinding

We can use `cx.bind_keys([...])` to bind keys to actions.

```rust
actions!(editor, [Clear, Backspace]);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = editor, no_json)]
pub struct DigitAction(pub u8);

const CONTEXT: &'static str = "MyComponent";

// Initialize in your init function
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("c", Clear, Some(CONTEXT)),
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("0", DigitAction(0), Some(CONTEXT)),
        KeyBinding::new("1", DigitAction(1), Some(CONTEXT)),
        // ... more digit bindings
    ]);
}


impl MyComponent {
    // We perfer to named `on_action_<action_name>` pattern for action handlers
    pub fn on_action_clear(&mut self, _: &Clear, _: &mut Window, cx: &mut Context<Self>) {
        // Handle clear action
        cx.notify();
    }

    pub fn on_action_digit(&mut self, action: &DigitAction, _: &mut Window, cx: &mut Context<Self>) {
        // Handle digit input, action.0 contains the digit value
        cx.notify();
    }
}

// In your component's render method
impl Render for MyComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context(CONTEXT)  // Activate contextual bindings
            .on_action(cx.listener(Self::on_action_clear))
            .on_action(cx.listener(Self::on_action_backspace))
            .on_action(cx.listener(Self::on_action_digit))
            // ... rest of your UI
    }
}
```

### Key Format

Keys are specified as strings with optional modifiers:

```
Modifiers: cmd, ctrl, alt, shift, cmd-ctrl, etc.
Keys: a-z, 0-9, f1-f12, up, down, left, right, enter, escape, space, tab, backspace, delete, etc.
Special: -, =, [, ], \, ;, ', ,, ., /, `, etc.
```

### Element-Level Handlers

Handle actions on specific elements:

```rust
impl Render for MyComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_action(cx.listener(|this, action: &Save, window, cx| {
                this.save(cx);
            }))
            .on_action(cx.listener(|this, action: &MoveUp, window, cx| {
                this.move_cursor_up(cx);
            }))
            .child("Editor content")
    }
}
```

### Focus-Based Dispatching

Actions route to focused elements:

```rust
// Dispatch to currently focused element
window.dispatch_action(MoveUp.boxed_clone(), cx);

// Dispatch to specific focus handle
focus_handle.dispatch_action(&Save, window, cx);
```

## Best Practices

### Action Naming

- Use clear, descriptive names
- Follow namespace conventions
- Use consistent casing (PascalCase for action names)

### Keybinding Choices

- Follow platform conventions (Cmd on macOS, Ctrl on Windows/Linux)
- Provide alternatives for common actions
- Document custom keybindings

### Handler Organization

- Keep handlers focused and single-purpose
- Use match statements for action routing
- Handle errors gracefully

### Performance Considerations

- Actions are lightweight
- Avoid expensive operations in handlers
- Cache keymap lookups when possible
- Minimize action dispatches in tight loops

### Accessibility

- Ensure all functionality is keyboard accessible
- Provide clear action names for screen readers
- Test with keyboard-only navigation

Actions provide the foundation for keyboard-driven interfaces in GPUI, enabling rich, customizable user interactions while maintaining clean separation between input handling and application logic.</content>
<parameter name="filePath">.claude/skills/action/SKILL.md

---
name: gpui-action
description: Describes Action definition and keyboard shortcut binding in GPUI.
---

## Overview

Actions in GPUI provide a declarative way to handle keyboard-driven UI interactions. They decouple user input (key presses) from application logic, enabling customizable keybindings and consistent behavior across different UI contexts. Actions can be simple unit structs or complex types with data, and they integrate with GPUI's focus system for context-aware behavior.

## Action Definition

### Simple Actions

Use the `actions!` macro for simple actions without data:

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

### Complex Actions with Data

Use the `Action` derive macro for actions with parameters:

```rust
use gpui::{Action, actions};

#[derive(Clone, PartialEq, Action)]
#[action(namespace = editor)]
pub struct SelectRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = editor)]
pub struct InsertText {
    pub text: String,
    pub replace: bool,
}
```

### Action Traits

Actions automatically implement several traits:

- `Clone` - Actions can be cloned for dispatching
- `PartialEq` - Actions can be compared for deduplication
- `Action` - Core action trait for GPUI integration

### Action Metadata

Configure action behavior with attributes:

```rust
#[derive(Clone, PartialEq, Action)]
#[action(
    namespace = editor,
    name = "custom_name",           // Override action name
    no_json,                        // Skip JSON serialization
    deprecated_aliases = ["old_name"], // Old names still work
    deprecated = "Use new_action instead" // Deprecation warning
)]
pub struct MyAction;
```

## Action Registration

### Automatic Registration

Actions defined with the macros are automatically registered at startup. For manual control:

```rust
#[derive(Clone, PartialEq)]
pub struct CustomAction {
    value: i32,
}

impl Action for CustomAction {
    fn build(value: serde_json::Value, _: &App) -> Result<Self> {
        // Custom deserialization
        Ok(Self { value: value.as_i64().unwrap_or(0) as i32 })
    }

    fn name(&self) -> &str {
        "custom_action"
    }

    fn namespace(&self) -> &str {
        "my_app"
    }

    fn boxed_clone(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }
}
```

## Keybinding

### Keymap Structure

Keymaps bind keys to actions:

```rust
// Keymap entry format: "modifiers-key" -> action
let keymap = Keymap::new(vec![
    // Basic bindings
    ("cmd-n", NewFile),
    ("cmd-o", OpenFile),
    ("cmd-s", Save),
    ("cmd-w", CloseWindow),

    // With modifiers
    ("shift-cmd-s", SaveAs),
    ("cmd-shift-[", PreviousTab),
    ("cmd-shift-]", NextTab),

    // Function keys
    ("f11", ToggleFullscreen),

    // Special keys
    ("escape", Cancel),
    ("enter", Confirm),
    ("space", ToggleSelection),

    // Arrow keys
    ("up", MoveUp),
    ("down", MoveDown),
    ("left", MoveLeft),
    ("right", MoveRight),
]);
```

### Key Format

Keys are specified as strings with optional modifiers:

```
Modifiers: cmd, ctrl, alt, shift, cmd-ctrl, etc.
Keys: a-z, 0-9, f1-f12, up, down, left, right, enter, escape, space, tab, backspace, delete, etc.
Special: -, =, [, ], \, ;, ', ,, ., /, `, etc.
```

### Context-Aware Bindings

Bindings can be conditional based on context:

```rust
let keymap = Keymap::new(vec![
    Binding::new("cmd-c", Copy, "when: editor_focused"),
    Binding::new("cmd-v", Paste, "when: editor_focused"),
    Binding::new("cmd-x", Cut, "when: editor_focused"),
    Binding::new("escape", CloseModal, "when: modal_open"),
]);
```

### Keymap Loading

Load keymaps from JSON:

```rust
// keymap.json
{
  "editor": {
    "cmd-n": "editor::NewFile",
    "cmd-s": "editor::Save",
    "up": "editor::MoveUp"
  },
  "global": {
    "cmd-q": "Quit",
    "cmd-,": "OpenPreferences"
  }
}
```

```rust
// Load keymap
let keymap = Keymap::load(fs::read_to_string("keymap.json")?)?;
```

## Action Handling

### Global Action Handlers

Register handlers for actions anywhere in the app:

```rust
impl App {
    fn setup_actions(&mut self) {
        self.set_action_handler(move |action: &Quit, window, cx| {
            cx.quit();
        });

        self.set_action_handler(move |action: &NewFile, window, cx| {
            // Create new file
            workspace.new_file(cx);
        });
    }
}
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

## Advanced Action Patterns

### Action with State

```rust
#[derive(Clone, PartialEq, Action)]
#[action(namespace = editor)]
pub struct InsertMode {
    pub mode: String,
}

impl InsertMode {
    pub const INSERT: &str = "insert";
    pub const REPLACE: &str = "replace";
    pub const APPEND: &str = "append";
}
```

### Conditional Actions

```rust
impl MyEditor {
    fn handle_action(&mut self, action: &Action, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            Action::MoveUp => {
                if self.can_move_up() {
                    self.move_cursor(-1, 0);
                    cx.notify();
                }
            }
            Action::InsertChar(ch) => {
                if !self.readonly {
                    self.insert_char(*ch);
                    cx.notify();
                }
            }
            _ => {}
        }
    }
}
```

### Action Sequences

```rust
struct MacroRecorder {
    recording: bool,
    actions: Vec<Box<dyn Any>>,
}

impl MacroRecorder {
    fn record_action(&mut self, action: &dyn Any) {
        if self.recording {
            self.actions.push(action.boxed_clone());
        }
    }

    fn play_macro(&self, window: &mut Window, cx: &mut App) {
        for action in &self.actions {
            window.dispatch_action(action.boxed_clone(), cx);
        }
    }
}
```

## Keymap Management

### Multiple Keymaps

GPUI supports layered keymaps:

```rust
// Base keymap
let base_keymap = Keymap::new(vec![
    ("cmd-c", Copy),
    ("cmd-v", Paste),
]);

// Mode-specific keymap
let insert_keymap = Keymap::new(vec![
    ("escape", ExitInsertMode),
    ("enter", Newline),
]);

// Combine keymaps
let combined = base_keymap.merge(&insert_keymap);
```

### Keymap Switching

```rust
struct Editor {
    normal_mode: Keymap,
    insert_mode: Keymap,
    current_mode: EditorMode,
}

impl Editor {
    fn switch_mode(&mut self, mode: EditorMode, cx: &mut Context<Self>) {
        self.current_mode = mode;
        let keymap = match mode {
            EditorMode::Normal => &self.normal_mode,
            EditorMode::Insert => &self.insert_mode,
        };
        cx.set_keymap(keymap.clone());
    }
}
```

## Testing Actions

```rust
#[cfg(test)]
impl MyComponent {
    fn test_action_handling(&mut self, cx: &mut TestAppContext) {
        // Dispatch action
        cx.dispatch_action(MoveUp, cx.window);

        // Assert state changed
        assert_eq!(self.cursor_position.row, 0);
    }

    fn test_keybinding(&mut self, cx: &mut TestAppContext) {
        // Simulate key press
        cx.simulate_key_press("up", cx.window);

        // Assert action was triggered
        assert_eq!(self.cursor_position.row, 0);
    }
}
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

---
name: gpui-focus-handle
description: Describes the role of FocusHandle in GPUI and its relationship with GPUI Actions.
---

## Overview

FocusHandle in GPUI manages keyboard focus within the UI hierarchy. It provides a way to programmatically control which element receives keyboard input and coordinates with GPUI's action system to dispatch keyboard-driven commands. Focus handles create a focus tree that mirrors the UI element hierarchy, enabling predictable keyboard navigation and action routing.

## Core Concepts

### FocusHandle

A `FocusHandle` represents a focusable element in the UI tree:

```rust
// Create focus handle
let focus_handle = cx.focus_handle();

// Check focus state
let is_focused = focus_handle.is_focused(window);

// Set focus
focus_handle.focus(window);

// Get containing focus handle
let parent_handle = focus_handle.parent(window);
```

### Focus Tree

GPUI maintains a focus tree that mirrors the element hierarchy:

- **Root**: The window itself
- **Branches**: Container elements that can contain focusable children
- **Leaves**: Individual focusable elements (buttons, inputs, etc.)

```rust
// Focus tree structure
Window (root)
├── Container A
│   ├── Focusable Element 1
│   └── Focusable Element 2
└── Container B
    └── Focusable Element 3
```

## Focus Management

### Creating Focusable Elements

Elements become focusable by requesting a focus handle:

```rust
struct FocusableButton {
    focus_handle: FocusHandle,
    label: String,
}

impl FocusableButton {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            label: "Click me".to_string(),
        }
    }
}

impl Render for FocusableButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .bg(if self.focus_handle.is_focused(cx.window) {
                rgb(0x007acc)
            } else {
                rgb(0xcccccc)
            })
            .child(&self.label)
    }
}
```

### Focus Navigation

#### Programmatic Focus Control

```rust
impl MyComponent {
    fn focus_next_element(&mut self, cx: &mut Context<Self>) {
        // Focus next sibling
        if let Some(next_focus) = self.focus_handle.next_focusable(cx.window) {
            next_focus.focus(cx.window);
        }
    }

    fn focus_previous_element(&mut self, cx: &mut Context<Self>) {
        // Focus previous sibling
        if let Some(prev_focus) = self.focus_handle.previous_focusable(cx.window) {
            prev_focus.focus(cx.window);
        }
    }
}
```

#### Keyboard Navigation

GPUI provides default keyboard navigation (Tab/Shift+Tab):

```rust
// Elements automatically participate in tab order
// unless explicitly configured otherwise

div()
    .track_focus(&focus_handle)
    .tab_index(0) // Default tab index
    .child("Focusable content")
```

### Focus Events

Respond to focus changes:

```rust
impl FocusableButton {
    fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Observe focus changes
        cx.on_focus(&focus_handle, window, |this, cx| {
            // Gained focus
            this.on_focus_gained(cx);
        });

        cx.on_blur(&focus_handle, window, |this, cx| {
            // Lost focus
            this.on_focus_lost(cx);
        });

        Self {
            focus_handle,
            label: "Click me".to_string(),
        }
    }

    fn on_focus_gained(&mut self, cx: &mut Context<Self>) {
        self.is_focused = true;
        cx.notify();
    }

    fn on_focus_lost(&mut self, cx: &mut Context<Self>) {
        self.is_focused = false;
        cx.notify();
    }
}
```

## Relationship with Actions

### Action Dispatching

Focus handles route actions to focused elements:

```rust
// Dispatch action to focused element
focus_handle.dispatch_action(&MyAction::default(), window, cx);

// Global action dispatch (routes to focused element)
window.dispatch_action(MyAction.boxed_clone(), cx);
```

### Action Context

Actions receive focus context:

```rust
#[derive(Clone, PartialEq)]
struct MoveCursor {
    direction: Direction,
}

impl_actions!(editor, [MoveCursor]);

// Action handler receives focus information
fn move_cursor(action: &MoveCursor, window: &mut Window, cx: &mut App) {
    // Action dispatched to focused element
    if let Some(focused) = window.focused_element() {
        // Handle action based on focused element
    }
}
```

### Focus-Aware Actions

Actions can be conditional based on focus:

```rust
impl MyComponent {
    fn handle_action(&mut self, action: &MyAction, window: &mut Window, cx: &mut Context<Self>) {
        // Only handle if this element is focused
        if self.focus_handle.is_focused(window) {
            // Handle action
            match action {
                MyAction::Activate => self.activate(cx),
                MyAction::Deactivate => self.deactivate(cx),
            }
        }
    }
}
```

### Action Registration

Register action handlers on elements:

```rust
impl Render for MyComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_action))
            .child("Focusable element")
    }
}
```

## Focus Scopes

### Modal Focus

Create focus scopes for modal dialogs:

```rust
struct Modal {
    focus_handle: FocusHandle,
    content: Entity<ModalContent>,
}

impl Modal {
    fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Create focus scope
        focus_handle.set_focus_scope(window, cx);

        Self {
            focus_handle,
            content: cx.new(|_| ModalContent::new()),
        }
    }
}

impl Render for Modal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_escape))
            .child(self.content.clone())
    }
}
```

### Focus Trapping

Prevent focus from escaping a component:

```rust
impl Modal {
    fn handle_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        // Keep focus within modal
        let focusables = self.focus_handle.focusable_children(window);
        if focusables.is_empty() {
            return;
        }

        let current_index = focusables
            .iter()
            .position(|fh| fh.is_focused(window))
            .unwrap_or(0);

        let next_index = if window.modifiers().shift {
            // Shift+Tab: previous
            if current_index == 0 {
                focusables.len() - 1
            } else {
                current_index - 1
            }
        } else {
            // Tab: next
            (current_index + 1) % focusables.len()
        };

        focusables[next_index].focus(window);
    }
}
```

## Best Practices

### Focus Indicator Visibility

Always provide visible focus indicators:

```rust
div()
    .track_focus(&focus_handle)
    .when(focus_handle.is_focused(cx.window), |el| {
        el.border_2()
            .border_color(rgb(0x007acc))
            .outline_2()
            .outline_color(rgb(0x007acc))
    })
    .child("Focusable content")
```

### Logical Tab Order

Ensure tab order matches visual layout:

```rust
// Use tab_index for custom ordering
div()
    .track_focus(&focus_handle)
    .tab_index(1)
    .child("First in tab order")

div()
    .track_focus(&other_handle)
    .tab_index(2)
    .child("Second in tab order")
```

### Focus Management in Lists

```rust
struct ListItem {
    focus_handle: FocusHandle,
    index: usize,
}

impl ListItem {
    fn handle_arrow_keys(&mut self, action: &ArrowKey, window: &mut Window, cx: &mut Context<Self>) {
        match action.direction {
            Direction::Up => {
                if self.index > 0 {
                    // Focus previous item
                    self.parent_list.focus_item(self.index - 1, window);
                }
            }
            Direction::Down => {
                // Focus next item
                self.parent_list.focus_item(self.index + 1, window);
            }
            _ => {}
        }
    }
}
```

### Testing Focus Behavior

```rust
#[cfg(test)]
impl MyComponent {
    fn test_focus_navigation(&mut self, cx: &mut TestAppContext) {
        // Set initial focus
        self.focus_handle.focus(cx.window);

        // Simulate tab
        cx.dispatch_action(Tab, cx.window);

        // Assert focus moved to expected element
        assert!(self.next_element.focus_handle.is_focused(cx.window));
    }
}
```

### Accessibility Considerations

- Provide clear focus indicators
- Ensure keyboard navigation works without mouse
- Test with screen readers
- Avoid focus traps unless intentional (modals)
- Maintain logical tab order

### Performance Considerations

- Focus handles are lightweight
- Focus queries are fast
- Avoid excessive focus observers
- Cache focus state when possible

FocusHandle coordinates with GPUI's action system to create a cohesive keyboard-driven interface. Proper focus management ensures accessibility and provides users with predictable keyboard navigation patterns.</content>
<parameter name="filePath">.claude/skills/focus-handle/SKILL.md

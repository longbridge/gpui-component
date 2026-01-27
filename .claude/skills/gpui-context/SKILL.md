---
name: gpui-context
description: Context management in GPUI including App, Window, and AsyncApp. Use when working with contexts, entity updates, or window operations. Different context types provide different capabilities for UI rendering, entity management, and async operations.
---

## Overview

GPUI uses different context types for different scenarios:

**Context Types:**
- **`App`**: Global app state, entity creation
- **`Window`**: Window-specific operations, painting, layout
- **`Context<T>`**: Entity-specific context for component `T`
- **`AsyncApp`**: Async context for foreground tasks
- **`AsyncWindowContext`**: Async context with window access

## Quick Start

### Context<T> - Component Context

```rust
impl MyComponent {
    fn update_state(&mut self, cx: &mut Context<Self>) {
        self.value = 42;
        cx.notify(); // Trigger re-render

        // Spawn async task
        cx.spawn(async move |cx| {
            // Async work
        }).detach();

        // Get current entity
        let entity = cx.entity();
    }
}
```

### App - Global Context

```rust
fn main() {
    let app = Application::new();
    app.run(|cx: &mut App| {
        // Create entities
        let entity = cx.new(|cx| MyState::default());

        // Open windows
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| Root::new(view, window, cx))
        });
    });
}
```

### Window - Window Context

```rust
impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Window operations
        let is_focused = window.is_window_focused();
        let bounds = window.bounds();

        div().child("Content")
    }
}
```

### AsyncApp - Async Context

```rust
cx.spawn(async move |cx: &mut AsyncApp| {
    let data = fetch_data().await;

    entity.update(cx, |state, inner_cx| {
        state.data = data;
        inner_cx.notify();
    }).ok();
}).detach();
```

## Common Operations

### Entity Operations

```rust
// Create entity
let entity = cx.new(|cx| MyState::default());

// Update entity
entity.update(cx, |state, cx| {
    state.value = 42;
    cx.notify();
});

// Read entity
let value = entity.read(cx).value;
```

### Notifications and Events

```rust
// Trigger re-render
cx.notify();

// Emit event
cx.emit(MyEvent::Updated);

// Observe entity
cx.observe(&entity, |this, observed, cx| {
    // React to changes
}).detach();

// Subscribe to events
cx.subscribe(&entity, |this, source, event, cx| {
    // Handle event
}).detach();
```

### Window Operations

```rust
// Window state
let focused = window.is_window_focused();
let bounds = window.bounds();
let scale = window.scale_factor();

// Close window
window.remove_window();
```

### Async Operations

```rust
// Spawn foreground task
cx.spawn(async move |cx| {
    // Async work with entity access
}).detach();

// Spawn background task
cx.background_spawn(async move {
    // Heavy computation
}).detach();
```

## Context Hierarchy

```
App (Global)
  └─ Window (Per-window)
       └─ Context<T> (Per-component)
            └─ AsyncApp (In async tasks)
                 └─ AsyncWindowContext (Async + Window)
```

## Reference Documentation

- **API Reference**: See [api-reference.md](references/api-reference.md)
  - Complete context API, methods, conversions
  - Entity operations, window operations
  - Async contexts, best practices

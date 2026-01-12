---
name: gpui-context
description: Describes context types in GPUI including Window, App, and AsyncApp, covering update, update_in, and Window/App related content.
---

## Overview

Context (`App`, `Context<T>`) types allow interaction with global state, windows, entities, and system services. They are typically passed to functions as the argument named `cx`. When a function takes callbacks they come after the `cx` parameter.

## Context Type Hierarchy

### App

`App` is the root context type, providing access to global state and read and update of entities.

```rust
// App provides access to:
// - Global state and configuration
// - Entity management (create, read, update)
// - Background task spawning
// - Global observers and subscriptions
// - Asset loading
// - Platform services

fn some_function(cx: &mut App) {
    // Create new entities
    let entity = cx.new(|cx| MyState::default());
    
    // Spawn a async tasks in main thread
    cx.spawn(async move |cx| {
        // Async work with access to App context
    }).detach();

    // Spawn background tasks
    cx.background_spawn(async move {
        // Background work
    }).detach();
}
```

### Context<T> (Entity Context)

`Context<T>` is provided when updating an `Entity<T>`. This context dereferences into `App`, so functions which take `&App` can also take `&Context<T>`.

```rust
impl MyComponent {
    fn update_count(&mut self, cx: &mut Context<Self>) {
        self.count += 1;

        // Context<T> has all App methods
        cx.notify(); // Trigger re-render

        // Access to entity-specific operations
        let entity_id = cx.entity_id();

        // Can create other entities
        let child_entity = cx.new(|_| ChildState::new());
    }
}
```

### AsyncApp

`AsyncApp` and `AsyncWindowContext` are provided by `cx.spawn` and `cx.spawn_in`. These can be held across await points.

### Window

`Window` provides access to the state of an application window. It is passed to functions as an argument named `window` and comes before `cx` when present. It is used for managing focus, dispatching actions, directly drawing, getting user input state, etc.

```rust
fn handle_click(window: &mut Window, cx: &mut App) {
    // Window provides:
    // - Window bounds and focus state
    // - Mouse position and keyboard state
    // - Focus management
    // - Action dispatching
    // - Direct drawing operations

    let bounds = window.bounds();
    let viewport_size = window.viewport_size()l
    let is_focused = window.focused(cx);
    let mouse_position = window.mouse_position();

    // Dispatch actions
    window.dispatch_action(ToggleAction.boxed_clone(), cx);

    // Manage focus
    focus_handle.focus(window);
}
```

### Async Contexts

#### AsyncApp

`AsyncApp` is returned by `cx.spawn()` and allows async operations with access to the application:

```rust
cx.spawn(async move |cx: &mut AsyncApp| {
    // AsyncApp provides:
    // - Entity read/update operations
    // - Background task spawning
    // - Timer operations
    // - Global state access

    // Can update entities asynchronously
    entity.update(cx, |state, cx| {
        state.loading = true;
        cx.notify();
    }).await;

    // Perform async work
    let result = some_async_operation().await;

    // Update entity with result
    entity.update(cx, |state, cx| {
        state.data = result;
        state.loading = false;
        cx.notify();
    }).await;
}).detach();
```

#### AsyncWindowContext

`AsyncWindowContext` combines async capabilities with window access:

```rust
cx.spawn_in(window, async move |cx: &mut AsyncWindowContext| {
    // AsyncWindowContext provides:
    // - All AsyncApp functionality
    // - Window operations (bounds, focus, etc.)
    // - UI operations that require window context

    // Access window state
    let window_bounds = cx.bounds();

    // Update entity with window context
    entity.update_in(cx, |state, window, cx| {
        state.window_size = window.bounds().size;
        cx.notify();
    }).await;
}).detach();
```

## Context Operations

### Entity Operations

#### Creating Entities

```rust
// Create new entity
let entity = cx.new(|cx| MyState {
    value: 42,
});

// Create entity from existing value
let entity = cx.new(|cx| existing_struct);
```

#### Reading Entities

```rust
// Read entity state
let value = entity.read(cx, |state, _cx| state.value);

// Read with context access
let (value, theme) = entity.read(cx, |state, cx| {
    (state.value, cx.theme().clone())
});
```

#### Updating Entities

```rust
// Update entity
entity.update(cx, |state, cx| {
    state.value += 1;
    cx.notify(); // Trigger re-render
});

// Update with window context
entity.update_in(cx, |state, window, cx| {
    state.is_focused = window.is_window_focused();
    cx.notify();
});
```

### Task Management

#### Spawning Tasks

```rust
// Spawn foreground task
let task = cx.spawn(async move |cx| {
    // Async operations
});

// Spawn in specific window
let task = cx.spawn_in(window, async move |cx| {
    // Operations with window access
});

// Spawn background task
let task = cx.background_spawn(async move {
    // CPU-intensive work
});
```

#### Task Lifecycle

```rust
// Detach task to run indefinitely
task.detach();

// Wait for task completion
let result = task.await;

// Create ready task
let task = Task::ready(42);
```

### Observers and Subscriptions

#### Entity Observers

```rust
// Observe entity changes
cx.observe(&entity, |this, entity, cx| {
    // React to changes in observed_entity
    this.update_from_other(observed_entity);
});

cx.observe_in(&entity, window, |this, entity, window, cx| {
    // React to changes with window access
});
```

#### Global Observers

```rust
// Observe global events
cx.observe_global::<SettingsStore>(|event, cx| {
    // Handle global event
});

cx.observe_global_in::<SettingsStore>(window, move |picker, window, cx| {
})
```

#### Window Observers

```rust
// Observe window events
cx.observe_window_appearance(window, |appearance, cx| {
    // React to window appearance changes
});
```

### Focus Management

```rust
// Get current focus
let focused_element = window.focused_element();

// Set focus
focus_handle.focus(window);

// Check focus state
let is_focused = focus_handle.is_focused(window);
```

### Action Dispatching

```rust
// Dispatch action
window.dispatch_action(MyAction { param: 42 }.boxed_clone(), cx);

// Dispatch to specific focus handle
focus_handle.dispatch_action(&MyAction::default(), window, cx);
```

### Timer Operations

```rust
// Schedule delayed operation
cx.spawn(async move |cx| {
    cx.background_executor().timer(Duration::from_secs(1)).await;
    // Execute after 1 second
});
```

### Global State

```rust
// Access theme
let theme = cx.theme();

// Access global settings
let settings = cx.global::<MySettings>();

// Set global state
cx.set_global(MySettings::default());
```

## Context in Component Methods

### Render Method

```rust
impl Render for MyComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // window: &mut Window - access to window state
        // cx: &mut Context<Self> - access to entity and app state

        let bounds = window.bounds();
        let theme = cx.theme();

        div()
            .size_full()
            .bg(theme.colors().background)
            .child(format!("Window size: {}x{}", bounds.width, bounds.height))
    }
}
```

### Event Handlers

```rust
impl MyComponent {
    fn on_button_click(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Access to event, window, and context
        self.clicked = true;
        cx.notify();
    }
}
```

### Async Methods

```rust
impl MyComponent {
    fn load_data(&mut self, cx: &mut Context<Self>) {
        // Capture weak entity to avoid cycles
        let entity = cx.weak_entity();

        cx.spawn(async move |cx| {
            let data = fetch_data().await;

            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |state, cx| {
                    state.data = Some(data);
                    state.loading = false;
                    cx.notify();
                }).await;
            }
        }).detach();
    }
}
```

## Best Practices

### Context Parameter Ordering

Functions should follow the parameter order: `window`, then other parameters, then `cx`:

```rust
fn my_function(window: &mut Window, param: MyParam, cx: &mut App) {
    // Correct order
}
```

### Avoiding Context Borrowing Issues

```rust
// ❌ Bad: Nested entity updates can cause panics
entity1.update(cx, |_, cx| {
    entity2.update(cx, |_, cx| {
        // May panic
    });
});

// ✅ Good: Sequential updates
entity1.update(cx, |_, cx| {
    // Update entity1
});

entity2.update(cx, |_, cx| {
    // Update entity2
});
```

### Weak References in Async Code

```rust
let weak_entity = cx.weak_entity();
cx.spawn(async move |cx| {
    let result = some_operation().await;

    if let Some(entity) = weak_entity.upgrade() {
        entity.update(cx, |state, cx| {
            state.result = result;
            cx.notify();
        }).await;
    }
}).detach();
```

### Context Lifetimes

- `App` contexts are long-lived
- `Context<T>` are scoped to entity operations
- `AsyncApp`/`AsyncWindowContext` can be held across await points
- Always use weak references for long-lived closures

### Error Handling

```rust
// Handle fallible operations
match some_operation(cx).await {
    Ok(result) => {
        // Handle success
    }
    Err(err) => {
        // Handle error - log or show user feedback
        log::error!("Operation failed: {}", err);
    }
}
```

Context types form the backbone of GPUI's API, providing structured access to application state and services while maintaining safety and performance guarantees.</content>
<parameter name="filePath">.claude/skills/cx/SKILL.md

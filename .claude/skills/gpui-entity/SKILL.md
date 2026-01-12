---
name: gpui-entity
description: Describes GPUI Entity usage and application scenarios.
---

## Overview

An `Entity<T>` is a handle to state of type `T`. With `thing: Entity<T>`:

* `thing.entity_id()` returns `EntityId`
* `thing.downgrade()` returns `WeakEntity<T>`
* `thing.read(cx)` returns `&T`.
* `thing.read_with(cx, |thing: &T, cx: &App| ...)` returns the closure's return value.
* `thing.update(cx, |thing: &mut T, cx: &mut Context<T>| ...)` allows the closure to mutate the state, and provides a `Context<T>` for interacting with the entity. It returns the closure's return value.
* `thing.update_in(cx, |thing: &mut T, window: &mut Window, cx: &mut Context<T>| ...)` takes a `AsyncWindowContext` or `VisualTestContext`. It's the same as `update` while also providing the `Window`.

Within the closures, the inner `cx` provided to the closure must be used instead of the outer `cx` to avoid issues with multiple borrows.

Trying to update an entity while it's already being updated must be avoided as this will cause a panic.

When  `read_with`, `update`, or `update_in` are used with an async context, the closure's return value is wrapped in an `anyhow::Result`.

`WeakEntity<T>` is a weak handle. It has `read_with`, `update`, and `update_in` methods that work the same, but always return an `anyhow::Result` so that they can fail if the entity no longer exists. This can be useful to avoid memory leaks - if entities have mutually recursive handles to each other they will never be dropped.

## Core Concepts

### Entity Types

- **`Entity<T>`**: A strong reference to state of type `T`
- **`WeakEntity<T>`**: A weak reference that may become invalid if the entity is dropped
- **`AnyEntity`**: A dynamically-typed entity handle
- **`AnyWeakEntity`**: A dynamically-typed weak entity handle

### Context Methods for Entities

#### Getting Current Entity

```rust
// Get the current entity being updated
let current_entity = cx.entity();

// Get a weak reference to current entity
let weak_entity = cx.entity().downgrade();
```

#### Spawning Async Tasks

```rust
// Spawn foreground task (runs on UI thread)
cx.spawn(async move |this, cx| {
    // `this` is WeakEntity<T> of current entity
    // `cx` is &mut AsyncApp

    let result = some_async_work().await;

    // Update entity safely
    let _ = this.update(cx, |state, cx| {
        state.data = result;
        cx.notify();
    });
}).detach();

// Spawn background task (runs on background thread)
cx.background_spawn(async move {
    // Long-running computation
    let result = heavy_computation().await;
    // Cannot directly update entities here
    // Must use channels or other mechanisms
}).detach();
```

### Entity Creation

Entities are created through the context:

```rust
// Create a new entity with initial state
let my_entity = cx.new(|cx| MyState {
    count: 0,
    name: "Default".to_string(),
});

// Create from existing value
let my_entity = cx.new(|cx| existing_value);
```

### Entity Access Patterns

#### Reading State

```rust
// Read-only access
let count = my_entity.read(cx).count;

// Read with context access
let count = my_entity.read_with(cx, |state, cx| {
    state.count
});

// Read with both state and context
let (count, theme) = my_entity.read_with(cx, |state, cx| {
    (state.count, cx.theme().clone())
});
```

#### Updating State

```rust
// Mutable update
my_entity.update(cx, |state, cx| {
    state.count += 1;
    cx.notify(); // Trigger re-render
});

// Update with window context
my_entity.update_in(cx, |state, window, cx| {
    state.focused = window.is_window_focused();
    cx.notify();
});
```

#### Weak Entity Operations

Weak entities return `Result<T, E>` for all operations since they may be invalid:

```rust
let weak_entity = my_entity.downgrade();

if let Ok(count) = weak_entity.read_with(cx, |state, _cx| state.count) {
    println!("Count: {}", count);
}

// Update that may fail
let _ = weak_entity.update(cx, |state, cx| {
    state.count += 1;
    cx.notify();
});
```

### Entity Lifecycle

#### Entity ID

Every entity has a unique `EntityId`:

```rust
let entity_id = my_entity.entity_id();
```

#### Observing Entity Creation

Register observers for new entities of a type:

```rust
cx.observe_new_entities::<MyState>(|entity, cx| {
    println!("New entity created: {:?}", entity.entity_id());
}).detach();
```

#### Entity Disposal

Entities are automatically disposed when all strong references are dropped. Use weak references to avoid memory leaks in closures.

### Application Scenarios

#### Model-View Separation

```rust
struct CounterModel {
    count: usize,
    listeners: Vec<Box<dyn Fn(usize)>>,
}

struct CounterView {
    model: Entity<CounterModel>,
}

impl CounterModel {
    fn increment(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
        // Notify listeners
        for listener in &self.listeners {
            listener(self.count);
        }
        cx.notify();
    }
}

impl CounterView {
    fn new(cx: &mut App) -> Entity<Self> {
        let model = cx.new(|_cx| CounterModel {
            count: 0,
            listeners: Vec::new(),
        });

        cx.new(|cx| Self { model })
    }
}
```

#### Component State Management

```rust
struct TodoList {
    todos: Vec<String>,
    filter: TodoFilter,
}

enum TodoFilter {
    All,
    Active,
    Completed,
}

impl TodoList {
    fn add_todo(&mut self, todo: String, cx: &mut Context<Self>) {
        self.todos.push(todo);
        cx.notify();
    }

    fn toggle_filter(&mut self, filter: TodoFilter, cx: &mut Context<Self>) {
        self.filter = filter;
        cx.notify();
    }
}
```

#### Cross-Entity Communication

```rust
struct ParentComponent {
    child_entities: Vec<Entity<ChildComponent>>,
    global_state: Entity<GlobalState>,
}

impl ParentComponent {
    fn notify_children(&mut self, cx: &mut Context<Self>) {
        for child in &self.child_entities {
            child.update(cx, |child_state, cx| {
                // Update child based on parent state
                cx.notify();
            });
        }
    }
}
```

#### Async Operations with Entities

```rust
impl MyComponent {
    fn perform_async_operation(&mut self, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();

        cx.spawn(async move |cx| {
            // Perform async work
            let result = some_async_operation().await;

            // Update entity with result
            let _ = entity.update(cx, |state, cx| {
                state.result = Some(result);
                cx.notify();
            });
        }).detach();
    }
}
```

#### Background Tasks

```rust
impl MyComponent {
    fn perform_background_task(&mut self, cx: &mut Context<Self>) {
        cx.background_spawn(async move {
            // Long-running background work
            let result = some_background_operation().await;
            // Handle result (typically by updating an entity)
        }).detach();
    }
}
```

### Best Practices

#### Avoid Entity Borrowing Conflicts

```rust
// ❌ Bad: Nested updates can cause borrowing conflicts
entity1.update(cx, |_, cx| {
    entity2.update(cx, |_, cx| {
        // This may panic if entities are related
    });
});

// ✅ Good: Perform operations sequentially
entity1.update(cx, |_, cx| {
    // Update entity1
});

entity2.update(cx, |_, cx| {
    // Update entity2
});
```

#### Use Weak References in Closures

```rust
// ✅ Good: Use weak references to avoid cycles
let weak_self = cx.entity().downgrade();
some_callback(move || {
    let _ = weak_self.update(cx, |state, cx| {
        // Safe update
        cx.notify();
    });
});
```

#### Entity as Props

```rust
struct ChildComponent {
    parent: WeakEntity<ParentComponent>,
}

impl ChildComponent {
    fn notify_parent(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.parent.upgrade() {
            parent.update(cx, |parent_state, cx| {
                // Update parent
                cx.notify();
            });
        }
    }
}
```

#### Entity Observation

```rust
impl MyComponent {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let entity = cx.entity();

            // Observe self for changes
            cx.observe(&entity, |this, _, cx| {
                // React to changes
                println!("Component changed");
            }).detach();

            Self { /* fields */ }
        })
    }
}
```

#### Entity Subscription and Events

```rust
impl MyComponent {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let entity = cx.entity();

            // Subscribe to events from other entities
            cx.subscribe(&other_entity, |this, other_entity, event, cx| {
                // Handle event from other entity
                match event {
                    SomeEvent::DataChanged => {
                        // Update this component based on the event
                        cx.notify();
                    }
                }
            }).detach();

            Self { /* fields */ }
        })
    }
}
```

### Common Patterns

1. **Stateful Components**: Use entities for components that maintain internal state
2. **Shared State**: Use entities to share state between multiple components
3. **Event Handling**: Use entities to coordinate events between components
4. **Async State**: Use entities to manage state that changes based on async operations
5. **Parent-Child Relationships**: Use weak references to avoid circular dependencies
6. **Observer Pattern**: Use `cx.observe()` to react to entity state changes
7. **Event Subscription**: Use `cx.subscribe()` to handle events emitted by other entities
8. **Resource Management**: Use entities for managing external resources with proper cleanup

### Performance Considerations

- Entity operations are generally fast but avoid excessive updates
- Use `cx.notify()` judiciously to prevent unnecessary re-renders
- Consider using `WeakEntity` for long-lived references to prevent memory leaks
- Batch updates when possible to reduce notification overhead
- Avoid nested entity updates which can cause borrowing conflicts
- Use `read_with` instead of separate `read` calls when you need both state and context
- Prefer `WeakEntity` in event handlers and closures to prevent retain cycles
- Be mindful of entity lifecycle - entities are dropped when all strong references are gone

### Advanced Patterns

#### Entity Cloning and Sharing

```rust
impl MyComponent {
    fn share_entity(&self, cx: &mut Context<Self>) -> Entity<SharedState> {
        // Clone entity for sharing (increases reference count)
        self.shared_entity.clone()
    }
}
```

#### Conditional Updates

```rust
impl MyComponent {
    fn update_if_needed(&mut self, new_value: T, cx: &mut Context<Self>) {
        if self.value != new_value {
            self.value = new_value;
            cx.notify();
        }
    }
}
```

#### Entity Collections

```rust
struct Container {
    items: Vec<Entity<Item>>,
    weak_items: Vec<WeakEntity<Item>>, // For avoiding cycles
}

impl Container {
    fn add_item(&mut self, item: Entity<Item>, cx: &mut Context<Self>) {
        self.items.push(item.clone());
        self.weak_items.push(item.downgrade());
        cx.notify();
    }

    fn cleanup_invalid_items(&mut self, cx: &mut Context<Self>) {
        // Remove items that no longer exist
        self.weak_items.retain(|weak| weak.upgrade().is_some());
        cx.notify();
    }
}
```

Entities form the backbone of GPUI's reactive architecture, enabling safe concurrent access to application state while maintaining clear data flow patterns.</content>
<parameter name="filePath">.claude/skills/entity/SKILL.md

---
name: gpui-entity
description: Entity management and state handling in GPUI. Use when working with entities, managing component state, coordinating between components, handling async operations with state updates, or implementing reactive patterns. Entities provide safe concurrent access to application state.
---

## Overview

An `Entity<T>` is a handle to state of type `T`, providing safe access and updates.

**Key Methods:**
- `entity.read(cx)` → `&T` - Read-only access
- `entity.read_with(cx, |state, cx| ...)` → `R` - Read with closure
- `entity.update(cx, |state, cx| ...)` → `R` - Mutable update
- `entity.downgrade()` → `WeakEntity<T>` - Create weak reference
- `entity.entity_id()` → `EntityId` - Unique identifier

**Entity Types:**
- **`Entity<T>`**: Strong reference (increases ref count)
- **`WeakEntity<T>`**: Weak reference (doesn't prevent cleanup, returns `Result`)

## Quick Start

### Creating and Using Entities

```rust
// Create entity
let counter = cx.new(|cx| Counter { count: 0 });

// Read state
let count = counter.read(cx).count;

// Update state
counter.update(cx, |state, cx| {
    state.count += 1;
    cx.notify(); // Trigger re-render
});

// Weak reference (for closures/callbacks)
let weak = counter.downgrade();
let _ = weak.update(cx, |state, cx| {
    state.count += 1;
    cx.notify();
});
```

### In Components

```rust
struct MyComponent {
    shared_state: Entity<SharedData>,
}

impl MyComponent {
    fn new(cx: &mut App) -> Entity<Self> {
        let shared = cx.new(|_| SharedData::default());

        cx.new(|cx| Self {
            shared_state: shared,
        })
    }

    fn update_shared(&mut self, cx: &mut Context<Self>) {
        self.shared_state.update(cx, |state, cx| {
            state.value = 42;
            cx.notify();
        });
    }
}
```

### Async Operations

```rust
impl MyComponent {
    fn fetch_data(&mut self, cx: &mut Context<Self>) {
        let weak_self = cx.entity().downgrade();

        cx.spawn(async move |cx| {
            let data = fetch_from_api().await;

            // Update entity safely
            let _ = weak_self.update(cx, |state, cx| {
                state.data = Some(data);
                cx.notify();
            });
        }).detach();
    }
}
```

## Core Principles

### Always Use Weak References in Closures

```rust
// ✅ Good: Weak reference prevents retain cycles
let weak = cx.entity().downgrade();
callback(move || {
    let _ = weak.update(cx, |state, cx| cx.notify());
});

// ❌ Bad: Strong reference may cause memory leak
let strong = cx.entity();
callback(move || {
    strong.update(cx, |state, cx| cx.notify());
});
```

### Use Inner Context

```rust
// ✅ Good: Use inner cx from closure
entity.update(cx, |state, inner_cx| {
    inner_cx.notify(); // Correct
});

// ❌ Bad: Use outer cx (multiple borrow error)
entity.update(cx, |state, inner_cx| {
    cx.notify(); // Wrong!
});
```

### Avoid Nested Updates

```rust
// ✅ Good: Sequential updates
entity1.update(cx, |state, cx| { /* ... */ });
entity2.update(cx, |state, cx| { /* ... */ });

// ❌ Bad: Nested updates (may panic)
entity1.update(cx, |_, cx| {
    entity2.update(cx, |_, cx| { /* ... */ });
});
```

## Common Use Cases

1. **Component State**: Internal state that needs reactivity
2. **Shared State**: State shared between multiple components
3. **Parent-Child**: Coordinating between related components (use weak refs)
4. **Async State**: Managing state that changes from async operations
5. **Observations**: Reacting to changes in other entities

## Reference Documentation

### Complete API Documentation
- **Entity API**: See [api-reference.md](references/api-reference.md)
  - Entity types, methods, lifecycle
  - Context methods, async operations
  - Error handling, type conversions

### Implementation Guides
- **Patterns**: See [patterns.md](references/patterns.md)
  - Model-view separation, state management
  - Cross-entity communication, async operations
  - Observer pattern, event subscription
  - Pattern selection guide

- **Best Practices**: See [best-practices.md](references/best-practices.md)
  - Avoiding common pitfalls, memory leaks
  - Performance optimization, batching updates
  - Lifecycle management, cleanup
  - Async best practices, testing

- **Advanced Patterns**: See [advanced.md](references/advanced.md)
  - Entity collections, registry pattern
  - Debounced/throttled updates, state machines
  - Entity snapshots, transactions, pools

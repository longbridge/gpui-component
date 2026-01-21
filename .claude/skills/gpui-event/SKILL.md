---
name: gpui-event
description: Event handling and subscriptions in GPUI. Use when implementing events, observers, or event-driven patterns. Supports custom events, entity observations, and event subscriptions for coordinating between components.
---

## Overview

GPUI provides event system for component coordination:

**Event Mechanisms:**
- **Custom Events**: Define and emit type-safe events
- **Observations**: React to entity state changes
- **Subscriptions**: Listen to events from other entities
- **Global Events**: App-wide event handling

## Quick Start

### Define and Emit Events

```rust
#[derive(Clone)]
enum MyEvent {
    DataUpdated(String),
    ActionTriggered,
}

impl MyComponent {
    fn update_data(&mut self, data: String, cx: &mut Context<Self>) {
        self.data = data.clone();

        // Emit event
        cx.emit(MyEvent::DataUpdated(data));
        cx.notify();
    }
}
```

### Subscribe to Events

```rust
impl Listener {
    fn new(source: Entity<MyComponent>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            // Subscribe to events
            cx.subscribe(&source, |this, emitter, event: &MyEvent, cx| {
                match event {
                    MyEvent::DataUpdated(data) => {
                        this.handle_update(data.clone(), cx);
                    }
                    MyEvent::ActionTriggered => {
                        this.handle_action(cx);
                    }
                }
            }).detach();

            Self { source }
        })
    }
}
```

### Observe Entity Changes

```rust
impl Observer {
    fn new(target: Entity<Target>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            // Observe entity for any changes
            cx.observe(&target, |this, observed, cx| {
                // Called when observed.update() calls cx.notify()
                println!("Target changed");
                cx.notify();
            }).detach();

            Self { target }
        })
    }
}
```

## Common Patterns

### 1. Parent-Child Communication

```rust
// Parent emits events
impl Parent {
    fn notify_children(&mut self, cx: &mut Context<Self>) {
        cx.emit(ParentEvent::Updated);
        cx.notify();
    }
}

// Children subscribe
impl Child {
    fn new(parent: Entity<Parent>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            cx.subscribe(&parent, |this, parent, event, cx| {
                this.handle_parent_event(event, cx);
            }).detach();

            Self { parent }
        })
    }
}
```

### 2. Global Event Broadcasting

```rust
struct EventBus {
    listeners: Vec<WeakEntity<dyn Listener>>,
}

impl EventBus {
    fn broadcast(&mut self, event: GlobalEvent, cx: &mut Context<Self>) {
        self.listeners.retain(|weak| {
            weak.update(cx, |listener, cx| {
                listener.on_event(&event, cx);
            }).is_ok()
        });
    }
}
```

### 3. Observer Pattern

```rust
cx.observe(&entity, |this, observed, cx| {
    // React to any state change
    let state = observed.read(cx);
    this.sync_with_state(state, cx);
}).detach();
```

## Best Practices

### ✅ Detach Subscriptions

```rust
// ✅ Detach to keep alive
cx.subscribe(&entity, |this, source, event, cx| {
    // Handle event
}).detach();
```

### ✅ Clean Event Types

```rust
#[derive(Clone)]
enum AppEvent {
    DataChanged { id: usize, value: String },
    ActionPerformed(ActionType),
    Error(String),
}
```

### ❌ Avoid Event Loops

```rust
// ❌ Don't create mutual subscriptions
entity1.subscribe(entity2) → emits event
entity2.subscribe(entity1) → emits event → infinite loop!
```

## Reference Documentation

- **API Reference**: See [api-reference.md](references/api-reference.md)
  - Event definition, emission, subscriptions
  - Observations, global events
  - Subscription lifecycle

- **Patterns**: See [patterns.md](references/patterns.md)
  - Event-driven architectures
  - Communication patterns
  - Best practices and pitfalls

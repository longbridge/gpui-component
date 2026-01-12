---
name: gpui-event
description: Describes the GPUI event system content.
---

## Overview

GPUI's event system enables loose coupling between components through typed events. Components can emit events to notify other parts of the application about state changes, and other components can subscribe to these events to react accordingly. The event system is built around the observer pattern with strong typing and automatic cleanup.

## Core Concepts

### EventEmitter Trait

Components declare the events they can emit by implementing `EventEmitter`:

```rust
#[derive(Clone)]
pub struct ItemSelected {
    pub item_id: usize,
}

#[derive(Clone)]
pub struct ItemDeleted {
    pub item_id: usize,
}

impl EventEmitter<ItemSelected> for MyComponent {}
impl EventEmitter<ItemDeleted> for MyComponent {}
```

### Event Emission

Emit events during entity updates:

```rust
impl MyComponent {
    fn select_item(&mut self, item_id: usize, cx: &mut Context<Self>) {
        self.selected_item = Some(item_id);

        // Emit event to notify subscribers
        cx.emit(ItemSelected { item_id });

        cx.notify(); // Also trigger re-render
    }

    fn delete_item(&mut self, item_id: usize, cx: &mut Context<Self>) {
        if let Some(pos) = self.items.iter().position(|item| item.id == item_id) {
            self.items.remove(pos);

            // Emit deletion event
            cx.emit(ItemDeleted { item_id });

            cx.notify();
        }
    }
}
```

### Event Subscription

Subscribe to events from other entities:

```rust
impl OtherComponent {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut this = Self::default();

            // Subscribe to events from another entity
            cx.subscribe(&other_entity, |this, other_entity, event, cx| {
                match event {
                    ItemSelected { item_id } => {
                        this.handle_item_selected(*item_id, cx);
                    }
                    ItemDeleted { item_id } => {
                        this.handle_item_deleted(*item_id, cx);
                    }
                }
            });

            this
        })
    }

    fn handle_item_selected(&mut self, item_id: usize, cx: &mut Context<Self>) {
        println!("Item {} was selected", item_id);
        // Update local state in response
        self.selected_items.insert(item_id);
        cx.notify();
    }

    fn handle_item_deleted(&mut self, item_id: usize, cx: &mut Context<Self>) {
        println!("Item {} was deleted", item_id);
        // Clean up references
        self.selected_items.remove(&item_id);
        cx.notify();
    }
}
```

## Subscription Management

### Subscription Lifetime

Subscriptions are automatically cleaned up when dropped:

```rust
struct ComponentWithSubscriptions {
    _item_selected_subscription: Subscription,
    _item_deleted_subscription: Subscription,
}

impl ComponentWithSubscriptions {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            // Subscriptions stored as fields - cleaned up when component is dropped
            let item_selected_subscription = cx.subscribe(&other_entity, |this, _, event, cx| {
                if let ItemSelected { item_id } = event {
                    this.handle_selection(*item_id, cx);
                }
            });

            let item_deleted_subscription = cx.subscribe(&other_entity, |this, _, event, cx| {
                if let ItemDeleted { item_id } = event {
                    this.handle_deletion(*item_id, cx);
                }
            });

            Self {
                _item_selected_subscription: item_selected_subscription,
                _item_deleted_subscription: item_deleted_subscription,
            }
        })
    }
}
```

### Conditional Subscriptions

Subscribe based on conditions:

```rust
impl MyComponent {
    fn setup_conditional_subscription(&mut self, cx: &mut Context<Self>) {
        if self.enable_notifications {
            self.notification_subscription = Some(cx.subscribe(&other_entity, |this, _, event, cx| {
                // Handle notification
            }));
        }
    }

    fn toggle_notifications(&mut self, cx: &mut Context<Self>) {
        self.enable_notifications = !self.enable_notifications;

        if self.enable_notifications {
            self.setup_conditional_subscription(cx);
        } else {
            self.notification_subscription = None; // Drops subscription
        }

        cx.notify();
    }
}
```

## Global Events

### Global Event Emission

Emit events that aren't tied to a specific entity:

```rust
#[derive(Clone)]
pub struct ThemeChanged {
    pub new_theme: Theme,
}

// Register as global
impl Global for ThemeChanged {}

// Emit globally
cx.emit_global(ThemeChanged {
    new_theme: new_theme.clone(),
});
```

### Global Event Subscription

Subscribe to global events:

```rust
impl ThemeAwareComponent {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut this = Self::default();

            // Subscribe to global theme changes
            cx.observe_global::<ThemeChanged>(|event, cx| {
                // Handle theme change
                this.apply_theme(&event.new_theme, cx);
            });

            this
        })
    }

    fn apply_theme(&mut self, theme: &Theme, cx: &mut Context<Self>) {
        self.current_theme = theme.clone();
        cx.notify();
    }
}
```

## Advanced Event Patterns

### Event Filtering

Filter events based on criteria:

```rust
impl EventFilteringComponent {
    fn setup_filtered_subscription(&mut self, cx: &mut Context<Self>) {
        cx.subscribe(&source_entity, |this, _, event, cx| {
            match event {
                ItemSelected { item_id } => {
                    // Only handle if item is relevant to this component
                    if this.is_interested_in_item(*item_id) {
                        this.handle_relevant_selection(*item_id, cx);
                    }
                }
                ItemDeleted { item_id } => {
                    // Always handle deletions to clean up state
                    this.handle_deletion(*item_id, cx);
                }
            }
        });
    }
}
```

### Event Transformation

Transform events before emitting:

```rust
#[derive(Clone)]
pub struct ItemUpdated {
    pub item: Item,
    pub changes: Vec<Change>,
}

impl MyComponent {
    fn update_item(&mut self, item_id: usize, new_data: ItemData, cx: &mut Context<Self>) {
        let old_item = &self.items[item_id];
        let changes = self.compute_changes(old_item, &new_data);

        // Update local state
        self.items[item_id] = Item::from(new_data);

        // Emit transformed event
        cx.emit(ItemUpdated {
            item: self.items[item_id].clone(),
            changes,
        });

        cx.notify();
    }
}
```

### Event Buffering

Buffer events for batch processing:

```rust
struct EventBufferingComponent {
    pending_events: Vec<ItemSelected>,
    flush_timer: Option<Task<()>>,
}

impl EventBufferingComponent {
    fn handle_selection(&mut self, item_id: usize, cx: &mut Context<Self>) {
        self.pending_events.push(ItemSelected { item_id });

        // Schedule flush if not already scheduled
        if self.flush_timer.is_none() {
            let entity = cx.weak_entity();
            self.flush_timer = Some(cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_millis(100)).await;

                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.flush_pending_events(cx);
                    }).await;
                }
            }));
        }
    }

    fn flush_pending_events(&mut self, cx: &mut Context<Self>) {
        // Process all pending events at once
        for event in &self.pending_events {
            self.process_selection(event.item_id, cx);
        }
        self.pending_events.clear();
        self.flush_timer = None;
    }
}
```

## Event System Architecture

### Subscriber Sets

GPUI uses `SubscriberSet` for efficient event routing:

```rust
// Internal implementation detail
pub struct SubscriberSet<EntityId, Handler> {
    subscribers: HashMap<EntityId, Vec<Handler>>,
}

// Events are routed to specific entity IDs
cx.emit_to(entity_id, event);
```

### Event Listener Types

Different types of event listeners:

- **Entity Listeners**: `cx.subscribe(entity, handler)`
- **Global Listeners**: `cx.observe_global::<Event>(handler)`
- **Window Listeners**: `cx.observe_window(window, handler)`
- **New Entity Listeners**: `cx.observe_new_entities::<T>(handler)`

## Testing Events

```rust
#[cfg(test)]
impl MyComponent {
    fn test_event_emission(&mut self, cx: &mut TestAppContext) {
        // Create subscriber
        let subscriber = cx.new(|cx| {
            let mut component = SubscriberComponent::default();

            cx.subscribe(&self.entity(), |subscriber, _, event, cx| {
                match event {
                    ItemSelected { item_id } => {
                        subscriber.received_events.push(*item_id);
                        cx.notify();
                    }
                }
            });

            component
        });

        // Trigger event emission
        self.select_item(42, cx);

        // Run event loop
        cx.run_until_parked();

        // Assert event was received
        subscriber.read(cx, |subscriber, _| {
            assert_eq!(subscriber.received_events, vec![42]);
        });
    }
}
```

## Best Practices

### Event Design

- Use descriptive event names
- Include all relevant data in events
- Keep events immutable (Clone)
- Use specific event types over generic ones

### Subscription Management

- Store subscriptions as fields with underscore prefix
- Use weak entity references to avoid cycles
- Clean up subscriptions when no longer needed
- Consider subscription lifetime

### Performance Considerations

- Events are cloned for each subscriber
- Minimize data in frequently emitted events
- Use buffering for high-frequency events
- Avoid deep subscription chains

### Error Handling

- Events should not fail - handle errors internally
- Use logging for debugging event flow
- Ensure event handlers are robust

### Common Patterns

#### Model-View Coordination

```rust
// Model emits events
impl DataModel {
    fn update_data(&mut self, new_data: Data, cx: &mut Context<Self>) {
        self.data = new_data;
        cx.emit(DataUpdated { data: self.data.clone() });
        cx.notify();
    }
}

// View subscribes to model
impl DataView {
    fn new(model: &Entity<DataModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(model, |this, _, event, cx| {
            match event {
                DataUpdated { data } => {
                    this.display_data(data.clone(), cx);
                }
            }
        });

        Self { model: model.downgrade() }
    }
}
```

#### Event-Driven State Machines

```rust
enum ComponentState {
    Idle,
    Processing,
    Complete,
}

impl StateMachineComponent {
    fn transition(&mut self, event: &ComponentEvent, cx: &mut Context<Self>) {
        let new_state = match (&self.state, event) {
            (ComponentState::Idle, ComponentEvent::Start) => {
                self.start_processing(cx);
                ComponentState::Processing
            }
            (ComponentState::Processing, ComponentEvent::Complete) => {
                self.finish_processing(cx);
                ComponentState::Complete
            }
            _ => return, // Invalid transition
        };

        self.state = new_state;
        cx.emit(StateChanged { new_state });
        cx.notify();
    }
}
```

GPUI's event system enables decoupled, reactive architectures where components can communicate changes without tight coupling. Proper use of events leads to maintainable, testable code with clear data flow.</content>
<parameter name="filePath">.claude/skills/event/SKILL.md

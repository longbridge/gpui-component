---
name: gpui-event
description: Event handling and subscriptions in GPUI. Use when implementing events, observers, or event-driven patterns.
---

## Overview

GPUI's event system enables loose coupling between components through typed events. Components can emit events to notify other parts of the application about state changes, and other components can subscribe to these events to react accordingly. The event system is built around the observer pattern with strong typing and automatic cleanup.

## Core Concepts

There have `cx.observe`, `cx.subscribe`, `cx.observe_global` methods in GPUI:

- `cx.observe`: Observe changes to the entity itself, when the entity updates, the observer callback is invoked.
- `cx.subscribe`: Subscribe to events emitted by entities, if the entity invokes `cx.emit(event)`, all subscribers to that event type will be notified.
- `cx.observe_global`: Observe global events that are not tied to a specific entity.

### EventEmitter Trait

Components declare the events they can call `cx.emit` by implementing `EventEmitter`:

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
    // We perfer to store subscriptions like this:
    _subscriptions: Vec<Subscription>,
}

impl ComponentWithSubscriptions {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            // Subscriptions stored as fields - cleaned up when component is dropped
            let _subscriptions = vec![
                cx.subscribe(&other_entity, |this, _, event, cx| {
                    if let ItemSelected { item_id } = event {
                        this.handle_selection(*item_id, cx);
                    }
                }),
                cx.subscribe(&other_entity, |this, _, event, cx| {
                    if let ItemDeleted { item_id } = event {
                        this.handle_deletion(*item_id, cx);
                    }
                })
            ];

            Self { _subscriptions }
        })
    }
}
```

## Observing Entity Updates

```rust
struct MyState {
    title: SharedString,
}

struct MyView {
    state: Entity<MyState>,
}

impl MyView {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let my_state = cx.new(|_| MyState { title: SharedString::default() });
            
            // Observe updates to another entity
            cx.observe(&my_state, |this, my_state, cx| {
                // React to observed entity update
                let new_title = my_state.read(cx).title;
            });

            Self {
                my_state,
            }
        })
    }

    fn update_title(&mut self, new_title: SharedString, cx: &mut Context<Self>) {
        my_state.update(cx, |state, cx| {
            state.title = new_title.clone();
            cx.notify();
        });
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

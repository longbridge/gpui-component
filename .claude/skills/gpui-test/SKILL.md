---
name: gpui-test
description: Describes how to write GPUI tests using TestAppContext, VisualTestContext, and various testing patterns.
---

## Overview

GPUI provides a comprehensive testing framework that allows you to test UI components, async operations, and distributed systems. Tests run on a single-threaded executor that provides deterministic execution and the ability to test complex async scenarios. GPUI tests use the `#[gpui::test]` attribute and work with `TestAppContext` for basic testing and `VisualTestContext` for window-dependent tests.

## Core Testing Infrastructure

### Test Attributes

#### Basic Test

```rust
#[gpui::test]
fn my_test(cx: &mut TestAppContext) {
    // Test implementation
}
```

#### Async Test

```rust
#[gpui::test]
async fn my_async_test(cx: &mut TestAppContext) {
    // Async test implementation
}
```

#### Property Test with Iterations

```rust
#[gpui::test(iterations = 10)]
fn my_property_test(cx: &mut TestAppContext, mut rng: StdRng) {
    // Property testing with random data
}
```

### Test Contexts

#### TestAppContext

`TestAppContext` provides access to GPUI's core functionality without windows:

```rust
#[gpui::test]
fn test_entity_operations(cx: &mut TestAppContext) {
    // Create entities
    let entity = cx.new(|cx| MyComponent::new(cx));

    // Update entities
    entity.update(cx, |component, cx| {
        component.value = 42;
        cx.notify();
    });

    // Read entities
    let value = entity.read_with(cx, |component, _| component.value);
    assert_eq!(value, 42);
}
```

#### VisualTestContext

`VisualTestContext` extends `TestAppContext` with window support:

```rust
#[gpui::test]
fn test_with_window(cx: &mut TestAppContext) {
    // Create window with component
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| MyComponent::new(cx))
        }).unwrap()
    });

    // Convert to visual context
    let mut cx = VisualTestContext::from_window(window.into(), cx);

    // Access window and component
    let component = window.root(&mut cx).unwrap();
}
```

## Testing Patterns

### Basic Entity Testing

Test entity creation, updates, and reads:

```rust
#[gpui::test]
fn test_counter_entity(cx: &mut TestAppContext) {
    let counter = cx.new(|cx| Counter::new(cx));

    // Test initial state
    let initial_count = counter.read_with(cx, |counter, _| counter.count);
    assert_eq!(initial_count, 0);

    // Test updates
    counter.update(cx, |counter, cx| {
        counter.count = 42;
        cx.notify();
    });

    let updated_count = counter.read_with(cx, |counter, _| counter.count);
    assert_eq!(updated_count, 42);
}
```

### Event Testing

Test event emission and handling:

```rust
#[derive(Clone)]
struct ValueChanged {
    new_value: i32,
}

impl EventEmitter<ValueChanged> for MyComponent {}

#[gpui::test]
fn test_event_emission(cx: &mut TestAppContext) {
    let component = cx.new(|cx| {
        let mut comp = MyComponent::default();

        // Subscribe to self
        cx.subscribe_self(|this, event: &ValueChanged, cx| {
            this.received_value = event.new_value;
            cx.notify();
        });

        comp
    });

    // Emit event
    component.update(cx, |_, cx| {
        cx.emit(ValueChanged { new_value: 123 });
    });

    // Verify event was handled
    let received = component.read_with(cx, |comp, _| comp.received_value);
    assert_eq!(received, 123);
}
```

### Action Testing

Test action dispatching and handling:

```rust
actions!(my_app, [Increment, Decrement]);

#[gpui::test]
fn test_action_dispatch(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| Counter::new(cx))
        }).unwrap()
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let counter = window.root(&mut cx).unwrap();

    // Dispatch action via focus handle
    let focus_handle = counter.read_with(&cx, |counter, _| counter.focus_handle.clone());
    cx.update(|window, cx| {
        focus_handle.dispatch_action(&Increment, window, cx);
    });

    let count = counter.read_with(&cx, |counter, _| counter.count);
    assert_eq!(count, 1);
}
```

### Async Testing

Test async operations and background tasks:

```rust
impl MyComponent {
    fn load_data(&self, cx: &mut Context<Self>) -> Task<i32> {
        cx.spawn(async move |this, cx| {
            // Simulate async work
            this.update(cx, |comp, _| comp.loading = true).await;
            // Return result
            42
        })
    }

    fn background_update(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            // Background work
            this.update(cx, |comp, _| {
                comp.value += 10;
            }).await;
        }).detach();
    }
}

#[gpui::test]
async fn test_async_operations(cx: &mut TestAppContext) {
    let component = cx.new(|cx| MyComponent::new(cx));

    // Test awaited task
    let result = component.update(cx, |comp, cx| comp.load_data(cx)).await;
    assert_eq!(result, 42);

    // Test detached task
    component.update(cx, |comp, cx| comp.background_update(cx));

    // Detached tasks don't run until you yield
    let value_before = component.read_with(cx, |comp, _| comp.value);
    assert_eq!(value_before, 0);

    // Run pending tasks
    cx.run_until_parked();

    let value_after = component.read_with(cx, |comp, _| comp.value);
    assert_eq!(value_after, 10);
}
```

### Timer Testing

Test timer-based operations:

```rust
impl MyComponent {
    fn delayed_action(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;

            this.update(cx, |comp, cx| {
                comp.action_performed = true;
                cx.notify();
            }).await;
        }).detach();
    }
}

#[gpui::test]
async fn test_timers(cx: &mut TestAppContext) {
    let component = cx.new(|cx| MyComponent::new(cx));

    component.update(cx, |comp, cx| comp.delayed_action(cx));

    // Action shouldn't have completed yet
    let performed = component.read_with(cx, |comp, _| comp.action_performed);
    assert!(!performed);

    // Run until parked (timers complete)
    cx.run_until_parked();

    let performed = component.read_with(cx, |comp, _| comp.action_performed);
    assert!(performed);
}
```

### External I/O Testing

For tests involving external systems, use `allow_parking()`:

```rust
#[gpui::test]
async fn test_external_io(cx: &mut TestAppContext) {
    // Allow parking for external I/O
    cx.executor().allow_parking();

    // Simulate external operation
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        tx.send(42).ok();
    });

    let result = rx.await.unwrap();
    assert_eq!(result, 42);
}
```

## Property Testing

Use random data to test edge cases:

```rust
#[gpui::test(iterations = 10)]
fn test_counter_random_operations(cx: &mut TestAppContext, mut rng: StdRng) {
    let counter = cx.new(|cx| Counter::new(cx));

    let mut expected = 0i32;
    for _ in 0..100 {
        let delta = rng.random_range(-10..=10);
        expected += delta;

        counter.update(cx, |counter, cx| {
            counter.count += delta;
            cx.notify();
        });
    }

    let actual = counter.read_with(cx, |counter, _| counter.count);
    assert_eq!(actual, expected);
}
```

## Distributed Systems Testing

Test multiple app contexts communicating:

```rust
#[derive(Clone)]
struct NetworkMessage {
    from: String,
    to: String,
    data: i32,
}

#[gpui::test]
fn test_distributed_apps(cx_a: &mut TestAppContext, cx_b: &mut TestAppContext) {
    // Create components in different app contexts
    let comp_a = cx_a.new(|_| MyComponent::new("A".to_string()));
    let comp_b = cx_b.new(|_| MyComponent::new("B".to_string()));

    // Simulate message passing
    comp_a.update(cx_a, |comp, cx| {
        comp.send_message("B", 42, cx);
    });

    // Run async operations
    cx_a.run_until_parked();

    // Verify message received in other context
    comp_b.update(cx_b, |comp, _| {
        comp.receive_messages();
    });

    let messages = comp_b.read_with(cx_b, |comp, _| comp.messages.clone());
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].data, 42);
}
```

### Interleaving Testing

Test concurrent operations with random execution order:

```rust
#[gpui::test(iterations = 10)]
fn test_concurrent_operations(
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    mut rng: StdRng,
) {
    let comp_a = cx_a.new(|_| MyComponent::new());
    let comp_b = cx_b.new(|_| MyComponent::new());

    // Perform random operations across contexts
    for i in 0..20 {
        if rng.random_bool(0.5) {
            comp_a.update(cx_a, |comp, cx| {
                comp.perform_operation(i, cx);
            });
        } else {
            comp_b.update(cx_b, |comp, cx| {
                comp.perform_operation(i, cx);
            });
        }
    }

    // Run all pending operations
    cx_a.run_until_parked();

    // Verify final state
    let state_a = comp_a.read_with(cx_a, |comp, _| comp.state);
    let state_b = comp_b.read_with(cx_b, |comp, _| comp.state);

    // Assert invariants hold despite execution order
    assert!(state_a.is_consistent());
    assert!(state_b.is_consistent());
}
```

## Mocking and Isolation

### Network Mocking

Create mock networks for testing distributed features:

```rust
struct MockNetwork {
    messages: Arc<Mutex<Vec<NetworkMessage>>>,
}

impl MockNetwork {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn send(&self, message: NetworkMessage) {
        self.messages.lock().unwrap().push(message);
    }

    fn receive_all(&self) -> Vec<NetworkMessage> {
        self.messages.lock().unwrap().drain(..).collect()
    }
}

#[gpui::test]
fn test_networked_components(cx: &mut TestAppContext) {
    let network = Arc::new(MockNetwork::new());

    let sender = cx.new(|_| MessageSender::new(network.clone()));
    let receiver = cx.new(|_| MessageReceiver::new(network));

    // Send message
    sender.update(cx, |sender, _| {
        sender.send("Hello");
    });

    // Receive message
    receiver.update(cx, |receiver, _| {
        receiver.receive_all();
    });

    let received = receiver.read_with(cx, |receiver, _| receiver.messages.clone());
    assert_eq!(received, vec!["Hello"]);
}
```

## Testing Best Practices

### Test Organization

Group related tests in modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod entity_tests {
        use super::*;

        #[gpui::test]
        fn test_creation() { /* ... */ }

        #[gpui::test]
        fn test_updates() { /* ... */ }
    }

    mod async_tests {
        use super::*;

        #[gpui::test]
        async fn test_async_ops() { /* ... */ }
    }

    mod distributed_tests {
        use super::*;

        #[gpui::test]
        fn test_multi_app() { /* ... */ }
    }
}
```

### Setup and Teardown

Use helper functions for common setup:

```rust
fn create_test_counter(cx: &mut TestAppContext) -> Entity<Counter> {
    cx.new(|cx| Counter::new(cx))
}

#[gpui::test]
fn test_counter_operations(cx: &mut TestAppContext) {
    let counter = create_test_counter(cx);

    // Test operations
}
```

### Assertions

Use descriptive assertions:

```rust
#[gpui::test]
fn test_counter_bounds(cx: &mut TestAppContext) {
    let counter = create_test_counter(cx);

    // Test upper bound
    for _ in 0..100 {
        counter.update(cx, |counter, cx| {
            counter.increment(cx);
        });
    }

    let count = counter.read_with(cx, |counter, _| counter.count);
    assert!(count <= 100, "Counter should not exceed maximum");

    // Test lower bound
    for _ in 0..200 {
        counter.update(cx, |counter, cx| {
            counter.decrement(cx);
        });
    }

    let count = counter.read_with(cx, |counter, _| counter.count);
    assert!(count >= 0, "Counter should not go below minimum");
}
```

### Performance Testing

Test performance characteristics:

```rust
#[gpui::test]
fn test_operation_performance(cx: &mut TestAppContext) {
    let component = cx.new(|cx| MyComponent::new(cx));

    let start = std::time::Instant::now();

    // Perform many operations
    for i in 0..1000 {
        component.update(cx, |comp, cx| {
            comp.perform_operation(i, cx);
        });
    }

    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(100), "Operations should complete quickly");
}
```

## Running Tests

### Basic Test Execution

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_counter_operations

# Run tests in a specific module
cargo test entity_tests::

# Run with output
cargo test -- --nocapture
```

### Test Configuration

Enable test-support feature for GPUI tests:

```toml
[features]
test-support = ["gpui/test-support"]
```

```bash
cargo test --features test-support
```

GPUI's testing framework provides deterministic, fast, and comprehensive testing capabilities that mirror real application behavior while providing the control needed for thorough testing of complex UI and async scenarios.</content>
<parameter name="filePath">.claude/skills/gpui-test/SKILL.md
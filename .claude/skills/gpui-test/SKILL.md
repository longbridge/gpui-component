---
name: gpui-test
description: Writing tests for GPUI applications. Use when testing components, async operations, or UI behavior.
---

## Overview

GPUI provides a comprehensive testing framework that allows you to test UI components, async operations, and distributed systems. Tests run on a single-threaded executor that provides deterministic execution and the ability to test complex async scenarios. GPUI tests use the `#[gpui::test]` attribute and work with `TestAppContext` for basic testing and `VisualTestContext` for window-dependent tests.

### Rules

- If test does not require windows or rendering, we can avoid use `[gpui::test]` and `TestAppContext`, just write simple rust test.

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

## Additional Resources

- For detailed testing patterns and examples, see [reference.md](reference.md)
- For best practices and running tests, see [examples.md](examples.md)

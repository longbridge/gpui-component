# Testing

**Contents:** [Overview](#overview) · [Core Testing Infrastructure](#core-testing-infrastructure) · [Additional Resources](#additional-resources)

## Overview

GPUI provides a comprehensive testing framework that allows you to test UI components, async operations, and distributed systems. Tests run on a single-threaded executor that provides deterministic execution and the ability to test complex async scenarios. GPUI tests use the `#[gpui_kit::test]` attribute and work with `TestAppContext` for basic testing and `VisualTestContext` for window-dependent tests.

### Rules

- `#[gpui_kit::test]` is GPUI's test attribute reached through the umbrella crate. It needs the `test-support` feature: add `gpui-kit = { version = "...", features = ["test-support"] }` under `[dev-dependencies]`.
- If the test does not need windows or rendering, skip `#[gpui_kit::test]` and `TestAppContext` and write a plain `#[test]`.

## Core Testing Infrastructure

### Test Attributes

#### Basic Test

```rust
#[gpui_kit::test]
fn my_test(cx: &mut TestAppContext) {
    // Test implementation
}
```

#### Async Test

```rust
#[gpui_kit::test]
async fn my_async_test(cx: &mut TestAppContext) {
    // Async test implementation
}
```

#### Property Test with Iterations

```rust
#[gpui_kit::test(iterations = 10)]
fn my_property_test(cx: &mut TestAppContext, mut rng: StdRng) {
    // Property testing with random data
}
```

### Test Contexts

#### TestAppContext

`TestAppContext` provides access to GPUI's core functionality without windows:

```rust
#[gpui_kit::test]
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
#[gpui_kit::test]
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

- For detailed patterns (including re-entrancy crash-free testing), see [test-reference.md](test-reference.md)
- For examples and best practices, see [test-examples.md](test-examples.md)

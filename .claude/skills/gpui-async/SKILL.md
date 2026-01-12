---
name: gpui-async
description: Async operations and background tasks in GPUI. Use when working with async, spawn, background tasks, or concurrent operations.
---

## Overview

GPUI provides a sophisticated async runtime that integrates foreground UI updates with background computation. All entity updates and UI rendering occur on a single foreground thread, while CPU-intensive work can be offloaded to background threads. The async system ensures safe concurrent access to shared state while maintaining UI responsiveness.

## Core Components

### Executors

#### Foreground Executor

The foreground executor runs on the main UI thread and handles:

- Entity updates and notifications
- UI rendering and layout
- Event handling
- Coordination between UI and background work

#### Background Executor

The background executor runs CPU-intensive tasks on separate threads:

- File I/O operations
- Network requests
- Heavy computations
- Long-running tasks

### Task Types

#### Task<T>

A `Task<T>` represents an asynchronous operation that will complete with a value of type `T`:

```rust
struct MyView {
    // We perfer named `_task` if there only 1 task in the struct.
    _task: Task<()>,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        let entity = cx.weak_entity();
        let _task = cx.spawn(async move |cx| {
            // Async work here
            let data = fetch_data().await;

            // Update entity state
            entity.update(cx, |state, cx| {
                state.data = data;
                cx.notify();
            });
            
            Ok(())
        });

        Self {
            _task,
        }
    }
}
```

## Spawning Tasks

### Foreground Tasks

`cx.spawn` is used to spawn tasks on the foreground executor, allowing safe access to UI state:

If you want to update the UI, we need use `cx.spawn` instead of `cx.background_spawn`.

```rust
// Basic spawn
cx.spawn(async move |cx| {
    // This runs on the foreground thread
    // Can safely update entities and UI state

    let _ = entity.update(cx, |state, cx| {
        state.status = "Loading...";
        cx.notify();
    });

    // Perform async work
    let result = some_async_operation().await;

    let _ = entity.update(cx, |state, cx| {
        state.data = result;
        state.status = "Complete";
        cx.notify();
    });
}).detach();

// Note: The closure takes only `cx`, not `(this, cx)` as shown in some examples

// Spawn in specific window context
cx.spawn_in(window, async move |cx| {
    // Has access to both async and window operations
    let window_bounds = cx.bounds();

    entity.update_in(cx, |state, window, cx| {
        state.window_size = window.bounds().size;
        cx.notify();
    }).await;
}).detach();
```

### Background Tasks

Spawn CPU-intensive work on background threads, in this async closure that cannot access UI state directly:

```rust
cx.background_spawn(async move {
    // This runs on background threads
    // Cannot directly access UI state
    // Use for I/O, computation, etc.

    let result = heavy_computation().await;

    // Return result to be processed on foreground
    result
}).await;
```

## Task Coordination

### Combining Foreground and Background

```rust
impl MyComponent {
    fn perform_operation(&mut self, cx: &mut Context<Self>) {
        let entity = cx.weak_entity();

        cx.spawn(async move |cx| {
            // Update UI to show loading
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |state, cx| {
                    state.loading = true;
                    cx.notify();
                }).await;
            }

            // Perform background work
            let result = cx.background_spawn(async move {
                expensive_operation()
            }).await;

            // Update UI with result
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |state, cx| {
                    state.result = result;
                    state.loading = false;
                    cx.notify();
                }).await;
            }
        }).detach();
    }
}
```

### Task Cancellation

Tasks are automatically cancelled when dropped:

```rust
struct MyComponent {
    current_task: Option<Task<()>>,
}

impl MyComponent {
    fn start_operation(&mut self, cx: &mut Context<Self>) {
        // Cancel existing task
        self.current_task.take();

        // Start new task
        self.current_task = Some(cx.spawn(async move |cx| {
            // Long-running operation
            slow_operation().await;
        }));
    }

    fn cancel_operation(&mut self) {
        // Drop task to cancel it
        self.current_task.take();
    }
}
```

### Error Handling

```rust
cx.spawn(async move |cx| {
    match fallible_operation().await {
        Ok(result) => {
            entity.update(cx, |state, cx| {
                state.result = Some(result);
                cx.notify();
            }).await;
        }
        Err(error) => {
            entity.update(cx, |state, cx| {
                state.error = Some(error.to_string());
                cx.notify();
            }).await;
        }
    }
}).detach();
```

## Timing and Scheduling

### Timers

```rust
// Schedule delayed execution
cx.spawn(async move |cx| {
    // Wait 1 second
    cx.background_executor()
        .timer(Duration::from_secs(1))
        .await;

    entity.update(cx, |state, cx| {
        state.message = "Timer fired!";
        cx.notify();
    }).await;
}).detach();

// Periodic tasks
cx.spawn(async move |cx| {
    loop {
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        entity.update(cx, |state, cx| {
            state.counter += 1;
            cx.notify();
        }).await;
    }
}).detach();
```

### Task Priorities

GPUI supports task prioritization:

```rust
use gpui::Priority;

// High priority task
cx.background_spawn(Priority::High, async move {
    // Urgent work
});

// Default priority
cx.background_spawn(async move {
    // Normal priority work
});
```

## Best Practices

### Task Lifecycle Management

```rust
struct ComponentWithTasks {
    tasks: Vec<Task<()>>,
}

impl ComponentWithTasks {
    fn cleanup(&mut self) {
        // Cancel all tasks
        self.tasks.clear();
    }
}

impl Drop for ComponentWithTasks {
    fn drop(&mut self) {
        // Tasks cancelled automatically when dropped
    }
}
```

### Avoiding Memory Leaks

```rust
// ✅ Good: Use weak references
let weak_entity = cx.weak_entity();
cx.spawn(async move |cx| {
    let result = operation().await;

    if let Some(entity) = weak_entity.upgrade() {
        entity.update(cx, |state, cx| {
            state.result = result;
            cx.notify();
        }).await;
    }
    // Entity not upgraded - task completes cleanly
}).detach();

// ❌ Bad: Strong reference cycle
let entity = cx.entity().clone(); // Creates cycle
cx.spawn(async move |cx| {
    // entity holds reference to component
    // component holds reference to task
    // task holds reference to entity
    // Memory leak!
}).detach();
```

### Error Propagation

```rust
// Propagate errors properly
cx.spawn(async move |cx| {
    let result = cx.background_spawn(async move {
        fallible_operation()
    }).await;

    match result {
        Ok(data) => {
            entity.update(cx, |state, cx| {
                state.data = data;
                cx.notify();
            }).await;
        }
        Err(err) => {
            // Handle error - log and/or show to user
            log::error!("Background operation failed: {}", err);
            entity.update(cx, |state, cx| {
                state.error = Some(err.to_string());
                cx.notify();
            }).await;
        }
    }
}).detach();
```

### Testing Async Code

```rust
#[cfg(test)]
impl MyComponent {
    async fn test_async_operation(&mut self, cx: &mut TestAppContext) {
        // In tests, use run_until_parked to wait for async operations
        cx.spawn(async move |cx| {
            // Async test operation
        }).detach();

        cx.run_until_parked();
        // Assert state changes
    }
}
```

### Performance Considerations

- Use background executor for CPU-intensive work
- Avoid blocking the foreground thread
- Use `detach()` for fire-and-forget tasks
- Avoid `detach()` if this task has a loop or other long-running behavior to avoid memory leaks
- Cancel unnecessary tasks to free resources
- Use weak entity references to prevent leaks
- Batch UI updates to reduce notification overhead

### Common Patterns

#### Debounced Operations

```rust
struct DebouncedComponent {
    debounce_task: Option<Task<()>>,
}

impl DebouncedComponent {
    fn trigger_search(&mut self, query: String, cx: &mut Context<Self>) {
        // Cancel existing debounce
        self.debounce_task.take();

        let entity = cx.weak_entity();
        self.debounce_task = Some(cx.spawn(async move |cx| {
            // Wait for debounce period
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;

            // Perform search
            let results = search(query).await;

            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |state, cx| {
                    state.search_results = results;
                    cx.notify();
                }).await;
            }
        }));
    }
}
```

#### Polling Operations

```rust
impl PollingComponent {
    fn start_polling(&mut self, cx: &mut Context<Self>) {
        let entity = cx.weak_entity();

        cx.spawn(async move |cx| {
            loop {
                // Poll every 5 seconds
                cx.background_executor()
                    .timer(Duration::from_secs(5))
                    .await;

                if let Some(entity) = entity.upgrade() {
                    let should_continue = entity.read(cx, |state, _| state.should_poll).await;

                    if !should_continue {
                        break;
                    }

                    // Perform poll
                    let update = poll_for_updates().await;

                    entity.update(cx, |state, cx| {
                        state.last_poll = update;
                        cx.notify();
                    }).await;
                } else {
                    break; // Entity dropped
                }
            }
        }).detach();
    }
}
```

GPUI's async system provides powerful concurrency primitives while maintaining UI thread safety and responsiveness. Proper use of foreground and background executors ensures smooth user experiences even during intensive operations.</content>
<parameter name="filePath">.claude/skills/async/SKILL.md

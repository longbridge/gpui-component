---
name: gpui-async
description: Async operations and background tasks in GPUI. Use when working with async, spawn, background tasks, or concurrent operations. Essential for handling async I/O, long-running computations, and coordinating between foreground UI updates and background work.
---

## Overview

GPUI provides integrated async runtime for foreground UI updates and background computation.

**Key Concepts:**
- **Foreground tasks**: UI thread, can update entities (`cx.spawn`)
- **Background tasks**: Worker threads, CPU-intensive work (`cx.background_spawn`)
- All entity updates happen on foreground thread

## Quick Start

### Foreground Tasks (UI Updates)

```rust
impl MyComponent {
    fn fetch_data(&mut self, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();

        cx.spawn(async move |cx| {
            // Runs on UI thread, can await and update entities
            let data = fetch_from_api().await;

            entity.update(cx, |state, cx| {
                state.data = Some(data);
                cx.notify();
            }).ok();
        }).detach();
    }
}
```

### Background Tasks (Heavy Work)

```rust
impl MyComponent {
    fn process_file(&mut self, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();

        cx.background_spawn(async move {
            // Runs on background thread, CPU-intensive
            let result = heavy_computation().await;
            result
        })
        .then(cx.spawn(move |result, cx| {
            // Back to foreground to update UI
            entity.update(cx, |state, cx| {
                state.result = result;
                cx.notify();
            }).ok();
        }))
        .detach();
    }
}
```

### Task Management

```rust
struct MyView {
    _task: Task<()>,  // Prefix with _ if stored but not accessed
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        let entity = cx.entity().downgrade();

        let _task = cx.spawn(async move |cx| {
            // Task automatically cancelled when dropped
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                entity.update(cx, |state, cx| {
                    state.tick();
                    cx.notify();
                }).ok();
            }
        });

        Self { _task }
    }
}
```

## Core Patterns

### 1. Async Data Fetching

```rust
cx.spawn(async move |cx| {
    let data = fetch_data().await?;
    entity.update(cx, |state, cx| {
        state.data = Some(data);
        cx.notify();
    })?;
    Ok::<_, anyhow::Error>(())
}).detach();
```

### 2. Background Computation + UI Update

```rust
cx.background_spawn(async move {
    heavy_work()
})
.then(cx.spawn(move |result, cx| {
    entity.update(cx, |state, cx| {
        state.result = result;
        cx.notify();
    }).ok();
}))
.detach();
```

### 3. Periodic Tasks

```rust
cx.spawn(async move |cx| {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        // Update every 5 seconds
    }
}).detach();
```

### 4. Task Cancellation

Tasks are automatically cancelled when dropped. Store in struct to keep alive.

## Common Pitfalls

### ❌ Don't: Update entities from background tasks

```rust
// ❌ Wrong: Can't update entities from background thread
cx.background_spawn(async move {
    entity.update(cx, |state, cx| { // Compile error!
        state.data = data;
    });
});
```

### ✅ Do: Use foreground task or chain

```rust
// ✅ Correct: Chain with foreground task
cx.background_spawn(async move { data })
    .then(cx.spawn(move |data, cx| {
        entity.update(cx, |state, cx| {
            state.data = data;
            cx.notify();
        }).ok();
    }))
    .detach();
```

## Reference Documentation

### Complete Guides
- **API Reference**: See [api-reference.md](references/api-reference.md)
  - Task types, spawning methods, contexts
  - Executors, cancellation, error handling

- **Patterns**: See [patterns.md](references/patterns.md)
  - Data fetching, background processing
  - Polling, debouncing, parallel tasks
  - Pattern selection guide

- **Best Practices**: See [best-practices.md](references/best-practices.md)
  - Error handling, cancellation
  - Performance optimization, testing
  - Common pitfalls and solutions

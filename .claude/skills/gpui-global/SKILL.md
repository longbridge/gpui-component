---
name: gpui-global
description: Global state management in GPUI. Use when implementing global state, app-wide configuration, or shared resources.
---

## Overview

In GPUI, the `Global` trait enables types to be stored as global state within the application context. This is useful for application-wide configuration, shared resources, and state that needs to be accessible from anywhere in the app.

Types implementing `Global` can be set once and accessed globally through the `App` context. Global state is typically initialized during app startup and persists for the entire application lifetime.

## Core Concepts

### Global Trait Implementation

To make a type global, implement the `Global` trait:

```rust
use gpui::Global;

#[derive(Debug)]
pub struct MyGlobalState {
    pub value: i32,
    pub name: String,
}

impl Global for MyGlobalState {}
```

### Global State Access Methods

#### Setting Global State

```rust
// Set global state (usually done during initialization)
let global_state = MyGlobalState {
    value: 42,
    name: "My App".to_string(),
};
cx.set_global(global_state);
```

#### Accessing Global State

```rust
// Check if global state exists
if cx.has_global::<MyGlobalState>() {
    // Access read-only reference
    let state = cx.global::<MyGlobalState>();
    println!("Value: {}", state.value);

    // Access mutable reference
    let state_mut = cx.global_mut::<MyGlobalState>();
    state_mut.value += 1;
}
```

#### Convenience Methods

Most global types provide convenience methods for access:

```rust
impl MyGlobalState {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

// Usage
let value = MyGlobalState::global(cx).value;
MyGlobalState::global_mut(cx).value = 100;
```

### Initialization Pattern

Global state is typically initialized in an `init` function:

```rust
pub fn init(cx: &mut App) {
    // Create and set global state
    cx.set_global(MyGlobalState::default());

    // Additional setup can access the global state
    MyGlobalState::global_mut(cx).initialize(cx);
}
```

### Observing Global Changes

You can observe changes to global state:

```rust
cx.observe_global::<MyGlobalState>(|cx| {
    // This callback runs whenever the global state changes
    let state = MyGlobalState::global(cx);
    println!("Global state changed: {}", state.value);

    // Trigger UI updates or other side effects
    cx.refresh_windows();
}).detach();
```

## Application Scenarios

### Application Configuration

```rust
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub theme: ThemeMode,
    pub language: String,
    pub debug_mode: bool,
}

impl Global for AppConfig {}

impl AppConfig {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn update_theme(&mut self, theme: ThemeMode, cx: &mut App) {
        self.theme = theme;
        // Apply theme changes throughout the app
        Theme::change(theme, None, cx);
    }
}
```

### Theme Management (Real Example)

```rust
// From crates/ui/src/theme/mod.rs
impl Global for Theme {}

impl Theme {
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App) {
        let mode = mode.into();
        if !cx.has_global::<Theme>() {
            let mut theme = Theme::default();
            theme.light_theme = ThemeRegistry::global(cx).default_light_theme().clone();
            theme.dark_theme = ThemeRegistry::global(cx).default_dark_theme().clone();
            cx.set_global(theme);
        }

        let theme = cx.global_mut::<Theme>();
        theme.mode = mode;
        // Apply theme configuration...
    }
}
```

### Resource Registry

```rust
#[derive(Default)]
pub struct ResourceRegistry {
    images: HashMap<String, Arc<ImageData>>,
    fonts: HashMap<String, FontHandle>,
}

impl Global for ResourceRegistry {}

impl ResourceRegistry {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn load_image(&mut self, name: &str, path: PathBuf) -> Result<(), Error> {
        let image_data = load_image_from_path(path)?;
        self.images.insert(name.to_string(), Arc::new(image_data));
        Ok(())
    }

    pub fn get_image(&self, name: &str) -> Option<&Arc<ImageData>> {
        self.images.get(name)
    }
}
```

### Global State with Internal State

```rust
pub struct GlobalState {
    pub counter: usize,
    pub settings: AppSettings,
}

impl Global for GlobalState {}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            counter: 0,
            settings: AppSettings::default(),
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }
}

// Initialization
pub fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}
```

### Text View State Stack (Real Example)

```rust
// From crates/ui/src/global_state.rs
impl Global for GlobalState {}

pub(crate) struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
}

impl GlobalState {
    pub(crate) fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }
}
```

### Theme Registry (Real Example)

```rust
// From crates/ui/src/theme/registry.rs
impl Global for ThemeRegistry {}

impl ThemeRegistry {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    // Additional methods for theme management...
}
```

## Best Practices

### Initialization Order

Initialize global state in dependency order:

```rust
pub fn init_app(cx: &mut App) {
    // Initialize foundational globals first
    resource_registry::init(cx);

    // Then globals that depend on others
    theme_registry::init(cx);
    theme::init(cx);

    // Finally application-specific globals
    my_app_state::init(cx);
}
```

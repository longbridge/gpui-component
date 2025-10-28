---
title: Resizable
description: A flexible panel layout system with draggable resize handles and adjustable panels.
---

# Resizable

The resizable component system provides a flexible way to create layouts with resizable panels. It supports both horizontal and vertical resizing, nested layouts, size constraints, and drag handles. Perfect for creating paned interfaces, split views, and adjustable dashboards.

## Import

```rust
use gpui_component::resizable::{
    h_resizable, v_resizable, resizable_panel,
    ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent
};
```

## Usage

### Basic Horizontal Layout

```rust
let state = ResizableState::new(cx);

h_resizable("my-layout", state)
    .child(
        resizable_panel()
            .size(px(200.))
            .child("Left Panel")
    )
    .child(
        resizable_panel()
            .child("Right Panel")
    )
```

### Basic Vertical Layout

```rust
let state = ResizableState::new(cx);

v_resizable("vertical-layout", state)
    .child(
        resizable_panel()
            .size(px(100.))
            .child("Top Panel")
    )
    .child(
        resizable_panel()
            .child("Bottom Panel")
    )
```

### Panel Size Constraints

```rust
resizable_panel()
    .size(px(200.))                    // Initial size
    .size_range(px(150.)..px(400.))    // Min and max size
    .child("Constrained Panel")
```

### Multiple Panels

```rust
h_resizable("multi-panel", state)
    .child(
        resizable_panel()
            .size(px(200.))
            .size_range(px(150.)..px(300.))
            .child("Left Panel")
    )
    .child(
        resizable_panel()
            .child("Center Panel")
    )
    .child(
        resizable_panel()
            .size(px(250.))
            .child("Right Panel")
    )
```

### Nested Layouts

```rust
let main_state = ResizableState::new(cx);
let nested_state = ResizableState::new(cx);

v_resizable("main-layout", main_state)
    .child(
        resizable_panel()
            .size(px(300.))
            .child(
                h_resizable("nested-layout", nested_state)
                    .child(
                        resizable_panel()
                            .size(px(200.))
                            .child("Top Left")
                    )
                    .child(
                        resizable_panel()
                            .child("Top Right")
                    )
            )
    )
    .child(
        resizable_panel()
            .child("Bottom Panel")
    )
```

### Nested Panel Groups

```rust
let outer_state = ResizableState::new(cx);
let inner_state = ResizableState::new(cx);

h_resizable("outer", outer_state)
    .child(
        resizable_panel()
            .size(px(200.))
            .child("Left Panel")
    )
    .group(
        v_resizable("inner", inner_state)
            .child(
                resizable_panel()
                    .size(px(150.))
                    .child("Top Right")
            )
            .child(
                resizable_panel()
                    .child("Bottom Right")
            )
    )
```

### Handling Resize Events

```rust
struct MyView {
    resizable_state: Entity<ResizableState>,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);

        // Subscribe to resize events
        let subscription = cx.subscribe(&resizable_state, |this, _, event: &ResizablePanelEvent, cx| {
            match event {
                ResizablePanelEvent::Resized => {
                    // Handle resize completion
                    println!("Panel resized!");
                    this.handle_resize_complete(cx);
                }
            }
        });

        Self {
            resizable_state,
        }
    }

    fn handle_resize_complete(&mut self, cx: &mut Context<Self>) {
        // Access current panel sizes
        let sizes = self.resizable_state.read(cx).sizes();
        println!("Current panel sizes: {:?}", sizes);
        cx.notify();
    }
}
```

### Conditional Panel Visibility

```rust
resizable_panel()
    .visible(self.show_sidebar)
    .size(px(250.))
    .child("Sidebar Content")
```

### Panel with Size Limits

```rust
// Panel with minimum size only
resizable_panel()
    .size_range(px(100.)..Pixels::MAX)
    .child("Flexible Panel")

// Panel with both min and max
resizable_panel()
    .size_range(px(200.)..px(500.))
    .child("Constrained Panel")

// Panel with exact constraints
resizable_panel()
    .size(px(300.))
    .size_range(px(300.)..px(300.))  // Fixed size
    .child("Fixed Panel")
```

## API Reference

### ResizableState

| Method    | Description                               |
| --------- | ----------------------------------------- |
| `new(cx)` | Create a new resizable state entity       |
| `sizes()` | Get current panel sizes as `&Vec<Pixels>` |

### Resizable Panel Group Functions

| Function                 | Description                             |
| ------------------------ | --------------------------------------- |
| `h_resizable(id, state)` | Create horizontal resizable panel group |
| `v_resizable(id, state)` | Create vertical resizable panel group   |
| `resizable_panel()`      | Create a new resizable panel            |

### ResizablePanelGroup

| Method             | Description                               |
| ------------------ | ----------------------------------------- |
| `new(id, state)`   | Create a new panel group                  |
| `axis(axis)`       | Set resize axis (Horizontal/Vertical)     |
| `child(panel)`     | Add a resizable panel to the group        |
| `children(panels)` | Add multiple panels at once               |
| `group(group)`     | Add another panel group as a nested child |
| `size(size)`       | Set the size of the group container       |

### ResizablePanel

| Method              | Description                     |
| ------------------- | ------------------------------- |
| `new()`             | Create a new resizable panel    |
| `child(element)`    | Add child element to the panel  |
| `size(pixels)`      | Set initial panel size          |
| `size_range(range)` | Set size constraints (min..max) |
| `visible(bool)`     | Control panel visibility        |

### Size Constraints

| Constraint                    | Description                     |
| ----------------------------- | ------------------------------- |
| `px(100.)..px(400.)`          | Panel can be 100px to 400px     |
| `px(150.)..Pixels::MAX`       | Panel minimum 150px, no maximum |
| `PANEL_MIN_SIZE..Pixels::MAX` | Default constraints             |

### ResizablePanelEvent

| Event     | Description                            |
| --------- | -------------------------------------- |
| `Resized` | Emitted when a panel finishes resizing |

## Drag Handles

Resize handles are automatically created between panels:

- **Horizontal layouts**: Vertical drag handles between panels
- **Vertical layouts**: Horizontal drag handles between panels
- **Visual feedback**: Handles show hover and active states
- **Cursor changes**: Appropriate resize cursors on hover
- **Handle size**: 1px wide with 4px padding for easier interaction

### Handle Behavior

- Handles appear between adjacent panels
- Dragging adjusts sizes of neighboring panels
- Panels respect their size constraints during resize
- Overflow is handled by adjusting panel sizes proportionally

## Direction Support

### Horizontal Resizing

```rust
h_resizable("horizontal", state)
    .child(resizable_panel().child("Left"))
    .child(resizable_panel().child("Right"))
```

- Panels are arranged side by side
- Vertical drag handles between panels
- Resize by dragging left/right

### Vertical Resizing

```rust
v_resizable("vertical", state)
    .child(resizable_panel().child("Top"))
    .child(resizable_panel().child("Bottom"))
```

- Panels are stacked vertically
- Horizontal drag handles between panels
- Resize by dragging up/down

## Examples

### File Explorer Layout

```rust
struct FileExplorer {
    layout_state: Entity<ResizableState>,
    show_sidebar: bool,
}

impl Render for FileExplorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_resizable("file-explorer", self.layout_state.clone())
            .child(
                resizable_panel()
                    .visible(self.show_sidebar)
                    .size(px(250.))
                    .size_range(px(200.)..px(400.))
                    .child(
                        v_flex()
                            .p_4()
                            .child("📁 Folders")
                            .child("• Documents")
                            .child("• Pictures")
                            .child("• Downloads")
                    )
            )
            .child(
                resizable_panel()
                    .child(
                        v_flex()
                            .p_4()
                            .child("📄 Files")
                            .child("file1.txt")
                            .child("file2.pdf")
                            .child("image.png")
                    )
            )
    }
}
```

### IDE Layout

```rust
struct IDELayout {
    main_state: Entity<ResizableState>,
    sidebar_state: Entity<ResizableState>,
    bottom_state: Entity<ResizableState>,
}

impl Render for IDELayout {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_resizable("ide-main", self.main_state.clone())
            .child(
                resizable_panel()
                    .size(px(300.))
                    .size_range(px(200.)..px(500.))
                    .child(
                        v_resizable("sidebar", self.sidebar_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(200.))
                                    .child("File Explorer")
                            )
                            .child(
                                resizable_panel()
                                    .child("Outline")
                            )
                    )
            )
            .child(
                resizable_panel()
                    .child(
                        v_resizable("editor-area", self.bottom_state.clone())
                            .child(
                                resizable_panel()
                                    .child("Code Editor")
                            )
                            .child(
                                resizable_panel()
                                    .size(px(150.))
                                    .size_range(px(100.)..px(300.))
                                    .child("Terminal / Output")
                            )
                    )
            )
    }
}
```

### Dashboard with Widgets

```rust
struct Dashboard {
    layout_state: Entity<ResizableState>,
    widget_state: Entity<ResizableState>,
}

impl Render for Dashboard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_resizable("dashboard", self.layout_state.clone())
            .child(
                resizable_panel()
                    .size(px(120.))
                    .child("Header / Navigation")
            )
            .child(
                resizable_panel()
                    .child(
                        h_resizable("widgets", self.widget_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(300.))
                                    .child("Chart Widget")
                            )
                            .child(
                                resizable_panel()
                                    .child("Data Table")
                            )
                            .child(
                                resizable_panel()
                                    .size(px(250.))
                                    .child("Stats Panel")
                            )
                    )
            )
            .child(
                resizable_panel()
                    .size(px(60.))
                    .child("Footer")
            )
    }
}
```

### Settings Panel

```rust
struct SettingsPanel {
    settings_state: Entity<ResizableState>,
}

impl SettingsPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        let settings_state = ResizableState::new(cx);

        // Listen for resize events to save layout preferences
        cx.subscribe(&settings_state, |this, _, event: &ResizablePanelEvent, cx| {
            match event {
                ResizablePanelEvent::Resized => {
                    this.save_layout_preferences(cx);
                }
            }
        });

        Self { settings_state }
    }

    fn save_layout_preferences(&self, cx: &mut Context<Self>) {
        let sizes = self.settings_state.read(cx).sizes();
        // Save to preferences
        println!("Saving layout: {:?}", sizes);
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_resizable("settings", self.settings_state.clone())
            .child(
                resizable_panel()
                    .size(px(200.))
                    .size_range(px(150.)..px(300.))
                    .child(
                        v_flex()
                            .gap_2()
                            .p_4()
                            .child("Categories")
                            .child("• General")
                            .child("• Appearance")
                            .child("• Advanced")
                    )
            )
            .child(
                resizable_panel()
                    .child(
                        div()
                            .p_6()
                            .child("Settings Content Area")
                    )
            )
    }
}
```

## Best Practices

1. **State Management**: Use separate ResizableState for independent layouts
2. **Size Constraints**: Always set reasonable min/max sizes for panels
3. **Event Handling**: Subscribe to ResizablePanelEvent for layout persistence
4. **Nested Layouts**: Use `.group()` method for clean nested structures
5. **Performance**: Avoid excessive nesting for better performance
6. **User Experience**: Provide adequate handle padding for easier interaction

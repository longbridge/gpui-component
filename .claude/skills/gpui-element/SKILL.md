---
name: gpui-element
description: Implementing custom elements using GPUI's low-level Element API (vs. high-level Render/RenderOnce APIs). Use when you need maximum control over layout, prepaint, and paint phases for complex, performance-critical custom UI components that cannot be achieved with Render/RenderOnce traits.
---

## When to Use

Use the low-level `Element` trait when:
- Need fine-grained control over layout calculation
- Building complex, performance-critical components
- Implementing custom layout algorithms (masonry, circular, etc.)
- High-level `Render`/`RenderOnce` APIs are insufficient

**Prefer `Render`/`RenderOnce` for:** Simple components, standard layouts, declarative UI

## Quick Start

The `Element` trait provides direct control over three rendering phases:

```rust
impl Element for MyElement {
    type RequestLayoutState = MyLayoutState;  // Data passed to later phases
    type PrepaintState = MyPaintState;        // Data for painting

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    // Phase 1: Calculate sizes and positions
    fn request_layout(&mut self, .., window: &mut Window, cx: &mut App)
        -> (LayoutId, Self::RequestLayoutState)
    {
        let layout_id = window.request_layout(
            Style { size: size(px(200.), px(100.)), ..default() },
            vec![],
            cx
        );
        (layout_id, MyLayoutState { /* ... */ })
    }

    // Phase 2: Create hitboxes, prepare for painting
    fn prepaint(&mut self, .., bounds: Bounds<Pixels>, layout: &mut Self::RequestLayoutState,
                window: &mut Window, cx: &mut App) -> Self::PrepaintState
    {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        MyPaintState { hitbox }
    }

    // Phase 3: Render and handle interactions
    fn paint(&mut self, .., bounds: Bounds<Pixels>, layout: &mut Self::RequestLayoutState,
             paint_state: &mut Self::PrepaintState, window: &mut Window, cx: &mut App)
    {
        window.paint_quad(paint_quad(bounds, Corners::all(px(4.)), cx.theme().background));

        window.on_mouse_event({
            let hitbox = paint_state.hitbox.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if hitbox.is_hovered(window) && phase.bubble() {
                    // Handle interaction
                    cx.stop_propagation();
                }
            }
        });
    }
}

// Enable element to be used as child
impl IntoElement for MyElement {
    type Element = Self;
    fn into_element(self) -> Self::Element { self }
}
```

## Core Concepts

### Three-Phase Rendering

1. **request_layout**: Calculate sizes and positions, return layout ID and state
2. **prepaint**: Create hitboxes, compute final bounds, prepare for painting
3. **paint**: Render element, set up interactions (mouse events, cursor styles)

### State Flow

```
RequestLayoutState → PrepaintState → paint
```

State flows in one direction through associated types, passed as mutable references between phases.

### Key Operations

- **Layout**: `window.request_layout(style, children, cx)` - Create layout node
- **Hitboxes**: `window.insert_hitbox(bounds, behavior)` - Create interaction area
- **Painting**: `window.paint_quad(...)` - Render visual content
- **Events**: `window.on_mouse_event(handler)` - Handle user input

## Reference Documentation

### Complete API Documentation
- **Element Trait API**: See [api-reference.md](references/api-reference.md)
  - Associated types, methods, parameters, return values
  - Hitbox system, event handling, cursor styles

### Implementation Guides
- **Examples**: See [examples.md](references/examples.md)
  - Simple text element with highlighting
  - Interactive element with selection
  - Complex element with child management

- **Best Practices**: See [best-practices.md](references/best-practices.md)
  - State management, performance optimization
  - Interaction handling, layout strategies
  - Error handling, testing, common pitfalls

- **Common Patterns**: See [patterns.md](references/patterns.md)
  - Text rendering, container, interactive, composite, scrollable patterns
  - Pattern selection guide

- **Advanced Patterns**: See [advanced-patterns.md](references/advanced-patterns.md)
  - Custom layout algorithms (masonry, circular)
  - Element composition with traits
  - Async updates, memoization, virtual lists

---
name: gpui-element
description: Implementing custom elements using GPUI's low-level Element API (vs. high-level Render/RenderOnce APIs).
---

## Overview

GPUI provides two main APIs for creating custom UI elements:

1. **High-level APIs**: `Render` and `RenderOnce` traits - declarative, easier to use
2. **Low-level API**: `Element` trait - imperative, maximum control, used for complex components

The `Element` trait gives you direct control over the three phases of element rendering:
- **Request Layout**: Calculate element sizes and positions
- **Prepaint**: Prepare for painting (create hitboxes, etc.)
- **Paint**: Render the element and handle interactions

This SKILL focuses on implementing custom elements using the low-level `Element` trait.

## Core Concepts

### Element Trait Structure

The `Element` trait requires implementing three associated types and three methods:

```rust
impl Element for MyElement {
    type RequestLayoutState = MyLayoutState;  // Data passed between layout phases
    type PrepaintState = MyPaintState;        // Data for painting phase

    fn fn id(&self) -> Option<ElementId>
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(&mut self, ...) -> (LayoutId, Self::RequestLayoutState)
    fn prepaint(&mut self, ..., &mut Self::RequestLayoutState) -> Self::PrepaintState
    fn paint(&mut self, ..., &mut Self::RequestLayoutState, &mut Self::PrepaintState)
}
```

### Three-Phase Rendering Process

#### 1. Element Identification

Each element can optionally provide a unique `ElementId` for debugging and inspection:

```rust
fn id(&self) -> Option<ElementId> {
    Some(self.id.clone())
}
```

#### 2. Source Location Tracking (optional)

Here if we not wanted, just return None:

```rust
fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
}
```

#### 3. Request Layout Phase

This phase calculates sizes and positions for the element tree:

```rust
fn request_layout(
    &mut self,
    global_id: Option<&GlobalElementId>,
    inspector_id: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
) -> (LayoutId, Self::RequestLayoutState) {
    // Calculate child layouts
    let child_layout_id = child.request_layout(window, cx);

    // Create your own layout based on children
    let layout_id = window.request_layout(style, child_layout_id, cx);

    // Return layout ID and state to pass to next phases
    (layout_id, MyLayoutState { child_layout_id, ... })
}
```

#### 4. Prepaint Phase

Prepare for painting - create hitboxes, compute final bounds:

```rust
fn prepaint(
    &mut self,
    global_id: Option<&GlobalElementId>,
    inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    request_layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    cx: &mut App,
) -> Self::PrepaintState {
    // Compute child bounds
    let child_bounds = bounds; // or calculated subset

    // Prepaint children
    child.prepaint(window, cx);

    // Create hitboxes for interaction
    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

    // Return state for paint phase
    MyPaintState { hitbox, ... }
}
```

#### 5. Paint Phase

Render the element and handle interactions:

```rust
fn paint(
    &mut self,
    global_id: Option<&GlobalElementId>,
    inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    request_layout: &mut Self::RequestLayoutState,
    prepaint: &mut Self::PrepaintState,
    window: &mut Window,
    cx: &mut App,
) {
    // Paint children first
    child.paint(window, cx);

    // Paint your own content
    window.paint_quad(paint_quad(bounds, ...));

    // Set up interactions
    window.on_mouse_event(move |event, phase, window, cx| {
        // Handle interactions
    });
}
```

## Application Scenarios

### Simple Text Element

To write a `Element` that we need:

1. Impl `Element` for write low-level rendering control.
2. Impl `IntoElement` then this can be used as a child directly like elements implemented `RenderOnce` or `Render`.

```rust
pub struct SimpleText {
    id: ElementId,
    text: SharedString,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
}

impl IntoElement for SimpleText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SimpleText {
    type RequestLayoutState = StyledText;
    type PrepaintState = Hitbox;

    fn request_layout(&mut self, .., window: &mut Window, cx: &mut App)
        -> (LayoutId, Self::RequestLayoutState)
    {
        // Create styled text with highlights
        let mut runs = Vec::new();
        let mut ix = 0;
        for (range, highlight) in &self.highlights {
            if ix < range.start {
                runs.push(window.text_style().to_run(range.start - ix));
            }
            runs.push(window.text_style().highlight(*highlight).to_run(range.len()));
            ix = range.end;
        }
        if ix < self.text.len() {
            runs.push(window.text_style().to_run(self.text.len() - ix));
        }

        let styled_text = StyledText::new(self.text.clone()).with_runs(runs);
        let (layout_id, _) = styled_text.request_layout(None, None, window, cx);

        (layout_id, styled_text)
    }

    fn prepaint(&mut self, .., bounds: Bounds<Pixels>,
                styled_text: &mut Self::RequestLayoutState, window: &mut Window, cx: &mut App)
        -> Self::PrepaintState
    {
        styled_text.prepaint(None, None, bounds, &mut (), window, cx);
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        hitbox
    }

    fn paint(&mut self, .., bounds: Bounds<Pixels>,
             styled_text: &mut Self::RequestLayoutState, hitbox: &mut Self::PrepaintState,
             window: &mut Window, cx: &mut App)
    {
        styled_text.paint(None, None, bounds, &mut (), &mut (), window, cx);

        // Set cursor style for text
        window.set_cursor_style(CursorStyle::IBeam, hitbox);
    }
}
```

### Interactive Element with Selection

Based on `TextView` element with selection support:

```rust
pub struct SelectableText {
    id: ElementId,
    text: SharedString,
    selectable: bool,
    selection: Option<Selection>,
}

impl Element for SelectableText {
    type RequestLayoutState = TextLayout;
    type PrepaintState = Option<Hitbox>;

    fn request_layout(&mut self, .., window: &mut Window, cx: &mut App)
        -> (LayoutId, Self::RequestLayoutState)
    {
        let styled_text = StyledText::new(self.text.clone());
        let (layout_id, _) = styled_text.request_layout(None, None, window, cx);
        (layout_id, styled_text.layout().clone())
    }

    fn prepaint(&mut self, .., bounds: Bounds<Pixels>,
                text_layout: &mut Self::RequestLayoutState, window: &mut Window, cx: &mut App)
        -> Self::PrepaintState
    {
        if self.selectable {
            Some(window.insert_hitbox(bounds, HitboxBehavior::Normal))
        } else {
            None
        }
    }

    fn paint(&mut self, .., bounds: Bounds<Pixels>,
             text_layout: &mut Self::RequestLayoutState, hitbox: &mut Self::PrepaintState,
             window: &mut Window, cx: &mut App)
    {
        // Paint text
        let styled_text = StyledText::new(self.text.clone());
        styled_text.paint(None, None, bounds, &mut (), &mut (), window, cx);

        // Paint selection if any
        if let Some(selection) = &self.selection {
            Self::paint_selection(selection, text_layout, &bounds, window, cx);
        }

        // Handle mouse events for selection
        if let Some(hitbox) = hitbox {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);

            window.on_mouse_event({
                let bounds = bounds.clone();
                move |event: &MouseDownEvent, phase, _, cx| {
                    if bounds.contains(&event.position) && phase.bubble() {
                        // Start selection
                        self.start_selection(event.position);
                        cx.notify();
                    }
                }
            });
        }
    }
}
```

### Complex Element with Child Management

```rust
pub struct ComplexElement {
    id: ElementId,
    children: Vec<Box<dyn Element<RequestLayoutState = (), PrepaintState = ()>>>,
    scrollable: bool,
    scroll_state: Option<ScrollState>,
}

impl Element for ComplexElement {
    type RequestLayoutState = ComplexLayoutState;
    type PrepaintState = ComplexPaintState;

    fn request_layout(&mut self, .., window: &mut Window, cx: &mut App)
        -> (LayoutId, Self::RequestLayoutState)
    {
        let mut child_layouts = Vec::new();
        let mut max_width = px(0.);
        let mut total_height = px(0.);

        for child in &mut self.children {
            let (child_layout_id, child_state) = child.request_layout(window, cx);
            child_layouts.push((child_layout_id, child_state));
            // Calculate bounds based on child layouts
            // ...
        }

        let layout_id = if self.scrollable {
            // Create scrollable layout
            window.request_layout(
                Style {
                    size: size(max_width, total_height),
                    ..default()
                },
                child_layouts.into_iter().map(|(id, _)| id).collect(),
                cx
            )
        } else {
            // Create fixed layout
            window.request_layout(
                Style {
                    size: size(max_width, total_height),
                    ..default()
                },
                child_layouts.into_iter().map(|(id, _)| id).collect(),
                cx
            )
        };

        (layout_id, ComplexLayoutState {
            child_layouts,
            bounds: Bounds::new(point(px(0.), px(0.)), size(max_width, total_height)),
        })
    }

    fn prepaint(&mut self, .., bounds: Bounds<Pixels>,
                layout_state: &mut Self::RequestLayoutState, window: &mut Window, cx: &mut App)
        -> Self::PrepaintState
    {
        let mut child_bounds = Vec::new();
        let mut y_offset = px(0.);

        for (child, (_, _)) in self.children.iter_mut().zip(&layout_state.child_layouts) {
            let child_bound = Bounds::new(
                point(bounds.left(), bounds.top() + y_offset),
                size(bounds.width(), px(50.)) // Calculate actual height
            );
            child.prepaint(window, cx);
            child_bounds.push(child_bound);
            y_offset += child_bound.height();
        }

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        ComplexPaintState {
            child_bounds,
            hitbox,
        }
    }

    fn paint(&mut self, .., bounds: Bounds<Pixels>,
             layout_state: &mut Self::RequestLayoutState, paint_state: &mut Self::PrepaintState,
             window: &mut Window, cx: &mut App)
    {
        // Paint children
        for (i, child) in self.children.iter_mut().enumerate() {
            let child_bounds = paint_state.child_bounds[i];
            child.paint(window, cx);
        }

        // Paint scrollbars if needed
        if self.scrollable {
            // Paint scrollbar
        }

        // Handle scrolling events
        window.on_mouse_event({
            let hitbox = paint_state.hitbox.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if hitbox.is_hovered(window) && phase.bubble() {
                    // Handle scrolling
                }
            }
        });
    }
}
```

## Best Practices

### State Management

#### Using Associated Types Effectively

```rust
// Good: Use associated types to pass data between phases
type RequestLayoutState = (StyledText, Vec<ChildLayout>);
type PrepaintState = (Hitbox, Vec<ChildBounds>);

// Bad: Don't use () everywhere
type RequestLayoutState = ();
type PrepaintState = ();
```

#### Managing Complex State

```rust
// For elements with complex state, create dedicated structs
pub struct TextElementState {
    pub styled_text: StyledText,
    pub hitbox: Hitbox,
    pub selection: Option<Selection>,
    pub child_states: Vec<ChildState>,
}
```

### Performance Considerations

#### Minimize Allocations in Paint Phase

```rust
// Good: Pre-allocate in request_layout or prepaint
impl Element for MyElement {
    fn request_layout(&mut self, ..) -> (LayoutId, Vec<StyledText>) {
        let styled_texts = self.children.iter().map(|child| {
            StyledText::new(child.text.clone())
        }).collect();
        // ...
    }
}

// Bad: Allocate in paint phase
fn paint(&mut self, ..) {
    let styled_texts: Vec<_> = self.children.iter().map(|child| {
        StyledText::new(child.text.clone()) // Allocation in paint!
    }).collect();
}
```

#### Cache Expensive Computations

```rust
pub struct CachedElement {
    cached_layout: Option<TextLayout>,
    last_text: SharedString,
}

impl Element for CachedElement {
    fn request_layout(&mut self, ..) -> (LayoutId, Self::RequestLayoutState) {
        if self.last_text != self.current_text || self.cached_layout.is_none() {
            // Recompute expensive layout
            self.cached_layout = Some(self.compute_layout());
            self.last_text = self.current_text.clone();
        }
        // Use cached layout
    }
}
```

### Interaction Handling

#### Proper Event Bubbling

```rust
fn paint(&mut self, .., window: &mut Window, cx: &mut App) {
    window.on_mouse_event({
        let hitbox = self.hitbox.clone();
        move |event: &MouseDownEvent, phase, window, cx| {
            // Always check phase and bounds
            if !phase.bubble() || !hitbox.is_hovered(window) {
                return;
            }

            // Handle event
            cx.stop_propagation(); // If you handle it
        }
    });
}
```

#### Hitbox Management

```rust
// Create hitboxes in prepaint phase
fn prepaint(&mut self, .., bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Hitbox {
    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

    // For transparent elements that still need mouse events
    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Transparent);

    hitbox
}
```

### Layout Strategies

#### Fixed Size Elements

```rust
fn request_layout(&mut self, .., window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
    let layout_id = window.request_layout(
        Style {
            size: size(px(200.), px(100.)), // Fixed size
            ..default()
        },
        vec![], // No children
        cx
    );
    (layout_id, ())
}
```

#### Content-Based Sizing

```rust
fn request_layout(&mut self, .., window: &mut Window, cx: &mut App) -> (LayoutId, Size<Pixels>) {
    // Measure content
    let text_bounds = self.measure_text(window);
    let padding = Edges::all(px(8.));

    let layout_id = window.request_layout(
        Style {
            size: size(
                text_bounds.width() + padding.left + padding.right,
                text_bounds.height() + padding.top + padding.bottom,
            ),
            ..default()
        },
        vec![],
        cx
    );
    (layout_id, text_bounds.size())
}
```

#### Flexbox-Style Layout

```rust
fn request_layout(&mut self, .., window: &mut Window, cx: &mut App) -> (LayoutId, Vec<Bounds<Pixels>>) {
    let mut child_layout_ids = Vec::new();
    let mut child_sizes = Vec::new();

    for child in &mut self.children {
        let (layout_id, _) = child.request_layout(window, cx);
        child_layout_ids.push(layout_id);
        // Assume child provides size info
    }

    // Calculate flex layout
    let total_flex = self.children.len() as f32;
    let flex_size = size(self.bounds.width() / total_flex, self.bounds.height());

    let layout_id = window.request_layout(
        Style {
            flex: Flex::Row,
            size: self.bounds.size(),
            ..default()
        },
        child_layout_ids,
        cx
    );

    (layout_id, child_sizes)
}
```

### Error Handling

#### Graceful Degradation

```rust
fn request_layout(&mut self, .., window: &mut Window, cx: &mut App) -> (LayoutId, Option<TextLayout>) {
    match StyledText::new(self.text.clone()).request_layout(None, None, window, cx) {
        Ok((layout_id, _)) => (layout_id, Some(text_layout)),
        Err(_) => {
            // Fallback to simple text
            let fallback_text = StyledText::new("Error loading text".into());
            let (layout_id, _) = fallback_text.request_layout(None, None, window, cx);
            (layout_id, None)
        }
    }
}
```

### Testing Element Implementations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn test_element_layout() {
        let mut cx = TestAppContext::new();
        cx.update(|cx| {
            let mut element = MyElement::new();
            let (layout_id, layout_state) = element.request_layout(cx);

            // Assert layout properties
            assert!(layout_id.is_valid());
            // ... more assertions
        });
    }

    #[test]
    fn test_element_interaction() {
        let mut cx = TestAppContext::new();
        cx.update(|cx| {
            let mut element = MyElement::new();
            // Simulate mouse event
            let event = MouseDownEvent {
                position: point(px(10.), px(10.)),
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
            };

            // Test interaction handling
            // ... assertions
        });
    }
}
```

## Common Patterns

### Text Rendering Elements

- Use `StyledText` for text layout
- Handle text selection in paint phase
- Create hitboxes for text interaction
- Support text highlighting and styling

### Container Elements

- Manage child element layouts
- Handle scrolling and clipping
- Implement flex/grid-like layouts
- Coordinate child interactions

### Interactive Elements

- Create appropriate hitboxes
- Handle mouse/keyboard events
- Manage focus and cursor styles
- Support accessibility features

### Composite Elements

- Combine multiple child elements
- Manage complex state across children
- Coordinate animations and transitions
- Handle focus delegation

## Advanced Patterns

### Custom Layout Algorithms

```rust
// Implement custom layout not supported by GPUI's built-in layouts
pub struct MasonryLayout {
    columns: usize,
    children: Vec<Box<dyn Element>>,
}

impl Element for MasonryLayout {
    fn request_layout(&mut self, .., window: &mut Window, cx: &mut App) -> (LayoutId, MasonryState) {
        // Implement Pinterest-style masonry layout
        let mut columns = vec![Vec::new(); self.columns];
        let mut column_heights = vec![px(0.); self.columns];

        for child in &mut self.children {
            let (child_layout_id, _) = child.request_layout(window, cx);

            // Find shortest column
            let min_column = column_heights.iter().enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap().0;

            columns[min_column].push(child_layout_id);
            // Update column height...
        }

        // Create layout...
    }
}
```

### Element Composition with Traits

```rust
// Create reusable behaviors via traits
pub trait Hoverable {
    fn on_hover<F>(&mut self, f: F) -> &mut Self
    where F: Fn(&mut Window, &mut App) + 'static;
}

pub trait Clickable {
    fn on_click<F>(&mut self, f: F) -> &mut Self
    where F: Fn(&MouseUpEvent, &mut Window, &mut App) + 'static;
}

// Implement on your elements
impl Hoverable for MyElement {
    fn on_hover<F>(&mut self, f: F) -> &mut Self {
        self.hover_handler = Some(Box::new(f));
        self
    }
}
```

### Async Element Updates

```rust
pub struct AsyncElement {
    state: Entity<AsyncState>,
}

impl Element for AsyncElement {
    fn paint(&mut self, .., window: &mut Window, cx: &mut App) {
        // Spawn async task that updates the element
        cx.spawn(async move |this, cx| {
            let result = some_async_operation().await;
            this.update(cx, |element, cx| {
                element.handle_async_result(result);
                cx.notify();
            });
        }).detach();
    }
}
```

### Element Memoization

```rust
pub struct MemoizedElement<T> {
    value: T,
    cached_element: Option<Box<dyn Element>>,
    last_value: Option<T>,
}

impl<T: PartialEq + Clone> Element for MemoizedElement<T> {
    fn request_layout(&mut self, .., window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        if self.last_value.as_ref() != Some(&self.value) {
            // Recompute element
            self.cached_element = Some(Box::new(self.create_element()));
            self.last_value = Some(self.value.clone());
        }

        self.cached_element.as_mut().unwrap().request_layout(window, cx)
    }
}
```

The low-level `Element` API provides maximum flexibility for creating complex, high-performance UI components, but requires careful management of the three rendering phases and state transitions.</content>
<parameter name="filePath">.claude/skills/gpui-element/SKILL.md

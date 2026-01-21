---
name: gpui-style-guide
description: GPUI Component project style guide based on gpui-component code patterns. Use when writing new components, reviewing code, or ensuring consistency with existing gpui-component implementations. Covers component structure, trait implementations, naming conventions, and API patterns observed in the actual codebase.
---

## Overview

Code style guide derived from gpui-component implementation patterns.

**Based on**: Analysis of Button, Checkbox, Input, Select, and other components in crates/ui

## Component Structure

### Basic Component Pattern

```rust
use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, Div, ElementId,
    InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, StyleRefinement, Styled, Window,
};

#[derive(IntoElement)]
pub struct MyComponent {
    id: ElementId,
    base: Div,
    style: StyleRefinement,

    // Configuration fields
    size: Size,
    disabled: bool,
    selected: bool,

    // Content fields
    label: Option<Text>,
    children: Vec<AnyElement>,

    // Callbacks (use Rc for Clone)
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl MyComponent {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            size: Size::default(),
            disabled: false,
            selected: false,
            label: None,
            children: Vec::new(),
            on_click: None,
        }
    }

    // Builder methods
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl InteractiveElement for MyComponent {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for MyComponent {}

impl Styled for MyComponent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Implementation
        self.base
    }
}
```

### Stateful Component Pattern

```rust
#[derive(IntoElement)]
pub struct Select {
    state: Entity<SelectState>,
    style: StyleRefinement,
    size: Size,
    // ...
}

pub struct SelectState {
    open: bool,
    selected_index: Option<usize>,
    // ...
}

impl Select {
    pub fn new(state: &Entity<SelectState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}
```

## Trait Implementations

### Sizable

```rust
impl Sizable for MyComponent {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
```

### Selectable

```rust
impl Selectable for MyComponent {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}
```

### Disableable

```rust
impl Disableable for MyComponent {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn is_disabled(&self) -> bool {
        self.disabled
    }
}
```

## Variant Patterns

### Enum Variants

```rust
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Danger,
    Success,
    Warning,
    Ghost,
    Link,
}
```

### Trait-Based Variant API

```rust
pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;

    /// With the primary style for the Button.
    fn primary(self) -> Self {
        self.with_variant(ButtonVariant::Primary)
    }

    /// With the danger style for the Button.
    fn danger(self) -> Self {
        self.with_variant(ButtonVariant::Danger)
    }

    // ... more variants
}
```

### Custom Variant Pattern

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ButtonCustomVariant {
    color: Hsla,
    foreground: Hsla,
    border: Hsla,
    hover: Hsla,
    active: Hsla,
    shadow: bool,
}

impl ButtonCustomVariant {
    pub fn new(cx: &App) -> Self {
        Self {
            color: cx.theme().transparent,
            foreground: cx.theme().foreground,
            // ...
            shadow: false,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    // ... more builder methods
}
```

## Action and Keybinding Patterns

### Context Constant

```rust
const CONTEXT: &str = "Select";
```

### Init Function

```rust
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ])
}
```

### Action Usage

```rust
use crate::actions::{Cancel, Confirm, SelectDown, SelectUp};

div()
    .key_context(CONTEXT)
    .on_action(cx.listener(Self::on_action_select_up))
    .on_action(cx.listener(Self::on_action_confirm))
```

## Trait Definitions

### Item Traits

```rust
pub trait SelectItem: Clone {
    type Value: Clone;

    fn title(&self) -> SharedString;

    fn display_title(&self) -> Option<AnyElement> {
        None
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.title().into_element()
    }

    fn value(&self) -> &Self::Value;

    fn matches(&self, query: &str) -> bool {
        self.title().to_lowercase().contains(&query.to_lowercase())
    }
}
```

### Implement for Common Types

```rust
impl SelectItem for String {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        &self
    }
}

impl SelectItem for SharedString { /* ... */ }
impl SelectItem for &'static str { /* ... */ }
```

## Icon Pattern

### IconNamed Trait

```rust
pub trait IconNamed {
    fn path(self) -> SharedString;
}

impl<T: IconNamed> From<T> for Icon {
    fn from(value: T) -> Self {
        Icon::build(value)
    }
}
```

### IconName Enum

```rust
#[derive(IntoElement, Clone)]
pub enum IconName {
    ArrowDown,
    ArrowUp,
    Check,
    Close,
    // ... all icon names
}
```

## Documentation Patterns

### Component Documentation

```rust
/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox { }
```

### Method Documentation

```rust
impl Checkbox {
    /// Create a new Checkbox with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self { }

    /// Set the label for the checkbox.
    pub fn label(mut self, label: impl Into<Text>) -> Self { }

    /// Set the click handler for the checkbox.
    ///
    /// The `&bool` parameter indicates the new checked state after the click.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self { }
}
```

## Import Organization Pattern

```rust
// 1. External crate imports
use std::rc::Rc;

// 2. Crate imports
use crate::{
    ActiveTheme, Disableable, FocusableExt, Icon, IconName,
    Selectable, Sizable, Size, StyledExt,
};

// 3. GPUI imports
use gpui::{
    div, prelude::FluentBuilder as _, px, relative, rems,
    AnyElement, App, Div, ElementId, InteractiveElement,
    IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, StyleRefinement, Styled, Window,
};
```

## Field Organization

```rust
pub struct Component {
    // 1. Identity
    id: ElementId,
    base: Div,
    style: StyleRefinement,

    // 2. Configuration
    size: Size,
    disabled: bool,
    selected: bool,
    tab_stop: bool,
    tab_index: isize,

    // 3. Content/children
    label: Option<Text>,
    children: Vec<AnyElement>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,

    // 4. Callbacks (last)
    on_click: Option<Rc<dyn Fn(Args, &mut Window, &mut App) + 'static>>,
}
```

## Common Patterns

### Optional Elements

```rust
pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
    self.prefix = Some(prefix.into_any_element());
    self
}
```

### Callback Patterns

```rust
// Pattern 1: Event parameter first
pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
    self.on_click = Some(Rc::new(handler));
    self
}

// Pattern 2: State parameter
pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
    self.on_change = Some(Rc::new(handler));
    self
}
```

### Static Handler Functions

```rust
fn handle_click(
    on_click: &Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    checked: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let new_checked = !checked;
    if let Some(f) = on_click {
        (f)(&new_checked, window, cx);
    }
}
```

### Boolean Methods

```rust
// Enable/disable patterns
pub fn cleanable(mut self, cleanable: bool) -> Self {
    self.cleanable = cleanable;
    self
}

// Toggle methods (no parameter)
pub fn mask_toggle(mut self) -> Self {
    self.mask_toggle = true;
    self
}
```

## Size Methods

### Size Trait

```rust
impl Sizable for Component {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
```

### Convenience Size Methods (from StyleSized trait)

Components get `.xsmall()`, `.small()`, `.medium()`, `.large()` automatically via `StyleSized` trait.

## Rendering Patterns

### RenderOnce Pattern

```rust
impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (width, height) = self.size.input_size();

        self.base
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(8.))
            .min_w(width)
            .h(height)
            .when(self.disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .children(self.children)
    }
}
```

## Theme Usage

```rust
// Access theme colors
cx.theme().surface
cx.theme().foreground
cx.theme().border
cx.theme().primary
cx.theme().transparent

// Use in components
div()
    .bg(cx.theme().surface)
    .text_color(cx.theme().foreground)
    .border_color(cx.theme().border)
```

## Reference Documentation

- **Component Examples**: See [component-examples.md](references/component-examples.md)
  - Full component implementations
  - Common patterns in action

- **Trait Patterns**: See [trait-patterns.md](references/trait-patterns.md)
  - Detailed trait implementation guides
  - Custom trait design patterns

## Quick Checklist

When creating a new component in crates/ui:

- [ ] `#[derive(IntoElement)]` on struct
- [ ] Include `id: ElementId`, `base: Div`, `style: StyleRefinement`
- [ ] Implement `InteractiveElement`, `StatefulInteractiveElement`, `Styled`
- [ ] Implement `RenderOnce` trait
- [ ] Implement `Sizable` if component has sizes
- [ ] Implement `Selectable` if component can be selected
- [ ] Implement `Disableable` if component can be disabled
- [ ] Use `Rc<dyn Fn>` for callbacks
- [ ] Use `Option<AnyElement>` for optional child elements
- [ ] Import `prelude::FluentBuilder as _`
- [ ] Use theme colors via `cx.theme()`
- [ ] Follow field organization pattern

# GPUI Component Code Style Guide

Based on analysis of `Button`, `Checkbox`, `Input`, `Select`, and other components in `crates/ui/src`.

**Contents:** [Component Structure](#component-structure) · [Required Traits](#required-trait-implementations) · [Optional Traits](#optional-traits) · [Variants Pattern](#variants-pattern) · [Callback Signatures](#callback-signatures) · [Import Organization](#import-organization) · [Doc Comments](#doc-comments) · [Applying User Style Overrides](#applying-user-style-overrides) · [FluentBuilder Conditionals](#fluentbuilder-for-conditionals) · [Theme Colors](#theme-colors) · [Size Handling](#size-handling) · [Checklist](#checklist-for-new-components)

## Component Structure

### Standard Stateless Component

```rust
use std::rc::Rc;

use crate::{ActiveTheme, Disableable, Sizable, Size, StyledExt as _, /* ... */};
use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

/// A MyComponent element.
#[derive(IntoElement)]
pub struct MyComponent {
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

    // 3. Content
    label: Option<SharedString>,
    children: Vec<AnyElement>,

    // 4. Callbacks (last)
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl MyComponent {
    /// Create a new MyComponent with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            size: Size::default(),
            disabled: false,
            selected: false,
            tab_stop: true,
            tab_index: 0,
            label: None,
            children: Vec::new(),
            on_click: None,
        }
    }

    /// Set the label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the click handler.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}
```

### Stateful Component (Interactive, Needs `.id()`)

Components with mouse interactions (hover, click tracking) use `Stateful<Div>`:

```rust
use gpui::{Stateful, StatefulInteractiveElement as _, /* ... */};

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,  // Not Div — needs stateful for interaction tracking
    // ...
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: div().flex_shrink_0().id(id),  // .id() makes it Stateful<Div>
            // ...
        }
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}
```

---

## Required Trait Implementations

```rust
// All components that accept children
impl ParentElement for MyComponent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

// All components with styleable outer div
impl Styled for MyComponent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

// For interactive components (mouse events, hover, click)
impl InteractiveElement for MyComponent {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

// Required if InteractiveElement is implemented
impl StatefulInteractiveElement for MyComponent {}

// Rendering
impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .id(self.id)
            // Apply user style overrides last
            .refine_style(&self.style)
            .children(self.children)
    }
}
```

---

## Optional Traits

```rust
impl Disableable for MyComponent {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for MyComponent {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for MyComponent {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
```

Implementing `Sizable` gives `.xsmall()`, `.small()`, `.medium()`, `.large()` for free via `StyleSized`.

---

## Variants Pattern

Use a `Variants` trait with default method impls:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AlertVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

pub trait AlertVariants: Sized {
    fn with_variant(self, variant: AlertVariant) -> Self;

    fn info(self) -> Self { self.with_variant(AlertVariant::Info) }
    fn success(self) -> Self { self.with_variant(AlertVariant::Success) }
    fn warning(self) -> Self { self.with_variant(AlertVariant::Warning) }
    fn error(self) -> Self { self.with_variant(AlertVariant::Error) }
}

impl AlertVariants for MyAlert {
    fn with_variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }
}
```

---

## Callback Signatures

```rust
// Click event (ClickEvent first)
on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>

// State change (state value first)
on_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>
on_change: Option<Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>>
on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>
```

Always `Rc<dyn Fn>` — components are cloned and called multiple times.

---

## Import Organization

```rust
// 1. std
use std::rc::Rc;

// 2. crate imports (project internals)
use crate::{
    ActiveTheme, Disableable, Icon, IconName,
    Selectable, Sizable, Size, StyledExt as _,
    h_flex, v_flex,
};

// 3. gpui imports
use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
    px, rems, relative,
};
```

---

## Doc Comments

```rust
/// A Checkbox element.           ← struct: one-line with capital, period
#[derive(IntoElement)]
pub struct Checkbox { ... }

impl Checkbox {
    /// Create a new Checkbox with the given id.   ← constructor
    pub fn new(id: impl Into<ElementId>) -> Self { ... }

    /// Set the label for the checkbox.            ← setter
    pub fn label(mut self, label: impl Into<Text>) -> Self { ... }

    /// Set the click handler for the checkbox.
    ///
    /// The `&bool` parameter indicates the new checked state after the click.
    pub fn on_click(mut self, ...) -> Self { ... }
}
```

- Struct doc: `/// A {Name} element.`
- Constructor: `/// Create a new {Name} with the given id.`
- Setters: `/// Set the {field}.`
- No redundant comments — only document non-obvious behavior

---

## Applying User Style Overrides

Use `refine_style` to merge user's `Styled` calls onto the root element:

```rust
impl RenderOnce for MyComponent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            // Apply component defaults first, then user overrides
            .refine_style(&self.style)
            .children(self.children)
    }
}
```

---

## FluentBuilder for Conditionals

```rust
div()
    .when(self.disabled, |this| this.opacity(0.5).cursor_not_allowed())
    .when(self.selected, |this| this.bg(cx.theme().primary))
    .when_some(self.label.as_ref(), |this, label| {
        this.child(div().child(label.clone()))
    })
```

Always `use gpui::prelude::FluentBuilder as _;` for `.when()` / `.when_some()`.

---

## Theme Colors

```rust
// In render, access via cx.theme() (requires ActiveTheme import)
use crate::ActiveTheme;

div()
    .bg(cx.theme().surface)
    .text_color(cx.theme().foreground)
    .border_color(cx.theme().border)
    .when(is_active, |el| el.bg(cx.theme().primary))
```

---

## Size Handling

```rust
// Get pixel values based on Size
let (width, height) = self.size.input_size();

// Or use match
let font_size = match self.size {
    Size::XSmall => rems(0.75),
    Size::Small => rems(0.875),
    Size::Medium | Size::Size(_) => rems(1.0),
    Size::Large => rems(1.125),
};
```

---

## Checklist for New Components

- [ ] `#[derive(IntoElement)]`
- [ ] Fields: `id: ElementId`, `base: Div` (or `Stateful<Div>`), `style: StyleRefinement`
- [ ] `impl RenderOnce` — calls `.refine_style(&self.style)` on root element
- [ ] `impl Styled` returning `&mut self.style`
- [ ] `impl ParentElement` if accepts children
- [ ] `impl InteractiveElement` + `StatefulInteractiveElement` if interactive
- [ ] `impl Sizable` if has size variants
- [ ] `impl Disableable` if can be disabled
- [ ] `impl Selectable` if can be selected
- [ ] Callbacks as `Option<Rc<dyn Fn(...)>>`
- [ ] Doc comment on struct and public methods
- [ ] Import `prelude::FluentBuilder as _`

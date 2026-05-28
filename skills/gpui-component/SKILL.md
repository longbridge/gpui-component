---
name: gpui-component
description: How to use the gpui-component UI library in GPUI applications. Use when building UIs with gpui-component components (Button, Input, Select, Dialog, etc.), setting up the library, handling component state, theming, or following the component API patterns.
---

## Navigation

| Topic | File | When to load |
|-------|------|--------------|
| Using components (patterns, setup, common APIs) | [usage.md](references/usage.md) | Using Button, Input, Select, Dialog, theme, stateful/stateless patterns |
| Component style guide | [style-guide.md](references/style-guide.md) | Code conventions, trait implementations (for contributors) |

## Quick Reference

**Setup** — always required:
```rust
gpui_component::init(cx);  // in app.run()
Root::new(view, window, cx) // first-level view in every window
```

**Stateless** (use directly in render):
```rust
Button::new("id").primary().label("OK").on_click(|_, _, _| {})
Icon::new(IconName::Check).small()
```

**Stateful** (hold `Entity<State>` in your view):
```rust
let input = cx.new(|cx| InputState::new(window, cx).placeholder("..."));
// in render:
Input::new(&self.input)
```

**Theme colors**: `cx.theme().primary`, `cx.theme().background`, `cx.theme().foreground`

**Sizes**: `.xsmall()` / `.small()` / `.medium()` (default) / `.large()`

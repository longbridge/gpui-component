# Units, Phases, and Contexts

GPUI is retained state with declarative rendering. An entity survives across frames; the element tree returned by `render` is a fresh description of the current frame. Keep that distinction explicit.

## Context Types and Lifecycles

- `Context<Self>` mutates the current entity, creates listeners tied to it, emits its events, and notifies its observers.
- `App` gives access to application globals and entity reads/updates without implying ownership by the rendered element.
- `Window` owns focus, actions, input dispatch, element-keyed state, measurement, and animation-frame requests for that window.
- Layout, prepaint, and paint are later phases; use their hooks only when resolved geometry is genuinely required.

Never retain `&mut Window`, `&mut App`, or `&mut Context<_>` beyond the call in which it is provided. Retain typed handles—`Entity`, `WeakEntity`, `FocusHandle`, scroll handles, or domain IDs—instead.

---

## Choose the Right Unit

### 1. Use `RenderOnce` for value-like elements

Use a `RenderOnce`/`IntoElement` component when all inputs can be supplied by the caller and the element does not need to retain application state between frames. This is the normal choice for presentational wrappers and small controls.

```rust
#[derive(IntoElement)]
struct EmptyState {
    title: SharedString,
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .items_center()
            .text_color(cx.theme().muted_foreground)
            .child(self.title)
    }
}
```

### 2. Use `Entity<T>` for retained behavior

Use an entity-backed `Render` view when behavior spans frames or needs observation, subscriptions, focus, async work, history, measurement, or incremental updates. Store entities in an owning view rather than recreating them in `render`.

```rust
struct SearchView {
    query: Entity<InputState>,
}

impl SearchView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
        Self { query }
    }
}
```

Do not turn every visual fragment into an entity. Entity boundaries have lifecycle and coordination costs; use them where retained identity matters.

### 3. Elements, views, and behavior systems are different

Do not force every component into one template:
- **semantic elements** such as Button, Checkbox, Link, and Tabs;
- **compound behavior roots** such as Dialog, Popover, Select, and Combobox;
- **entity-backed systems** such as Input, Table, Tree, Dock, and notifications;
- **infrastructure** such as positioning, virtualization, scrolling, focus traps, motion, history, and measurement.

An element may be internally complex and still be value-like to its caller. A stateful system may expose render callbacks so applications own presentation without reimplementing behavior. Choose the public seam from the behavior, not from how many `div`s appear in its renderer.

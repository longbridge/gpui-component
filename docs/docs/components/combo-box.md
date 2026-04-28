---
title: ComboBox
description: An autocomplete input paired with a searchable dropdown list.
---

# ComboBox

A combobox component that allows users to select one (or many) values from a searchable list.

Compared to [Select](select), `ComboBox` adds support for custom trigger rendering and custom item rendering, making it easy to build rich selection UIs without forking the underlying list behaviour.

`MultiComboBox` is the multi-select variant — it toggles items in the selection and keeps the dropdown open until the user dismisses it.

## Import

```rust
use gpui_component::combo_box::{
    ComboBox, ComboBoxState, ComboBoxEvent,
    MultiComboBox, MultiComboBoxState, MultiComboBoxEvent,
    TriggerCtx, MultiTriggerCtx,
};
use gpui_component::searchable_list::{
    SearchableListItem, SearchableVec, SearchableGroup,
};
```

## Usage

### Basic Single-Select

```rust
let state = cx.new(|cx| {
    ComboBoxState::new(
        SearchableVec::new(vec!["Next.js", "SvelteKit", "Nuxt.js"]),
        None, // no initial selection
        window,
        cx,
    )
    .searchable(true)
});

ComboBox::new(&state)
    .placeholder("Select framework...")
    .search_placeholder("Search...")
    .w_full()
```

### Pre-selected Item

Pass the index path of the item to pre-select:

```rust
let state = cx.new(|cx| {
    ComboBoxState::new(items, Some(IndexPath::default()), window, cx)
});
```

### Grouped Items

Use `SearchableGroup` to group items under a heading:

```rust
let grouped = SearchableVec::new(vec![
    SearchableGroup::new("Fruits").items(vec![
        FoodItem::new("Apples"),
        FoodItem::new("Bananas"),
    ]),
    SearchableGroup::new("Vegetables").items(vec![
        FoodItem::new("Carrots"),
        FoodItem::new("Spinach"),
    ]),
]);

let state = cx.new(|cx| {
    ComboBoxState::new(grouped, None, window, cx).searchable(true)
});

ComboBox::new(&state)
```

### Implementing `SearchableListItem`

Built-in implementations of `SearchableListItem` exist for `String`, `SharedString`, and `&'static str`. For custom types implement the trait:

```rust
#[derive(Clone)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl SearchableListItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &SharedString {
        &self.code
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(query)
            || self.code.to_lowercase().contains(query)
    }
}
```

### Disabled Items

Return `true` from `disabled()` on items that should not be selectable:

```rust
impl SearchableListItem for MyItem {
    // ...
    fn disabled(&self) -> bool {
        self.is_unavailable
    }
}
```

### Custom Check Icon

```rust
ComboBox::new(&state)
    .check_icon(Icon::new(IconName::CircleCheck))
```

### Footer Action

Render a persistent action at the bottom of the dropdown (e.g. an "Add new" button):

```rust
ComboBox::new(&state)
    .footer(|_, cx| {
        Button::new("add-new")
            .ghost()
            .label("New item")
            .icon(Icon::new(IconName::Plus))
            .w_full()
            .justify_start()
            .into_any_element()
    })
```

### Custom Trigger

Override the entire trigger element. You control the label, icons, and layout. `TriggerCtx` exposes selection state, open/disabled flags, and the current size:

```rust
ComboBox::new(&state)
    .render_trigger(|ctx, _, cx| {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .when_some(ctx.selected_item, |this, item| {
                this.child(
                    div()
                        .bg(cx.theme().accent)
                        .rounded_sm()
                        .px_1p5()
                        .py_0p5()
                        .text_sm()
                        .child(item.title()),
                )
            })
            .when(ctx.selected_item.is_none(), |this| {
                this.text_color(cx.theme().muted_foreground)
                    .child("Select...")
            })
            .into_any_element()
    })
```

### Custom Item Renderer

Override how each item row is drawn. When set, the automatic trailing check icon is suppressed — your closure controls the full row:

```rust
ComboBox::new(&state)
    .render_item(|item: &MyItem, is_selected, _, cx| {
        h_flex()
            .w_full()
            .gap_2()
            .items_center()
            .child(Icon::new(item.icon.clone()).small())
            .child(div().child(item.title()))
            .into_any_element()
    })
```

### Sizes

```rust
ComboBox::new(&state).large()
ComboBox::new(&state)  // medium (default)
ComboBox::new(&state).small()
```

### Cleanable

```rust
ComboBox::new(&state).cleanable(true) // show clear button when value is selected
```

### Disabled

```rust
ComboBox::new(&state).disabled(true)
```

### Events

```rust
cx.subscribe_in(&state, window, |view, _, event, window, cx| {
    match event {
        ComboBoxEvent::Confirm(value) => {
            // value is Option<Value>
        }
    }
});
```

### Mutating

```rust
// Set by index
state.update(cx, |s, cx| {
    s.set_selected_index(Some(IndexPath::default()), window, cx);
});

// Set by value (requires Value: PartialEq)
state.update(cx, |s, cx| {
    s.set_selected_value(&"my-value".into(), window, cx);
});

// Read current value
let value = state.read(cx).selected_value(); // Option<&Value>
```

## Multi-Select

### Basic Multi-Select

`MultiComboBoxState` holds a `Vec<Value>` selection. Selecting an item toggles it; the dropdown stays open until dismissed.

```rust
let state = cx.new(|cx| {
    MultiComboBoxState::new(
        SearchableVec::new(vec!["React", "Vue", "Angular"]),
        vec!["React"], // pre-selected
        window,
        cx,
    )
    .searchable(true)
});

MultiComboBox::new(&state)
    .placeholder("Select frameworks")
```

### Custom Multi-Select Trigger

`MultiTriggerCtx` exposes `selected_values: &[Value]`:

```rust
MultiComboBox::new(&state)
    .render_trigger(|ctx, _, cx| {
        if ctx.selected_values.is_empty() {
            return div()
                .text_color(cx.theme().muted_foreground)
                .child("Select...")
                .into_any_element();
        }

        h_flex()
            .flex_wrap()
            .gap_1()
            .children(ctx.selected_values.iter().map(|val| {
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(cx.theme().border)
                    .px_1p5()
                    .py_0p5()
                    .text_sm()
                    .child(*val)
            }))
            .into_any_element()
    })
```

### Multi-Select Events

```rust
cx.subscribe_in(&state, window, |view, _, event, window, cx| {
    match event {
        MultiComboBoxEvent::Change(values) => {
            // fired on every toggle
        }
        MultiComboBoxEvent::Confirm(values) => {
            // fired when the dropdown closes
        }
    }
});
```

### Mutating Multi-Select

```rust
state.update(cx, |s, cx| {
    s.add_value("Vue", cx);
    s.remove_value(&"React", cx);
    s.clear_selection(cx);
    s.set_selected_values(vec!["Angular", "Svelte"], cx);
});

let values = state.read(cx).selected_values(); // &[Value]
```

## Keyboard Shortcuts

| Key       | Action                                   |
| --------- | ---------------------------------------- |
| `Tab`     | Focus trigger                            |
| `Enter`   | Open menu or confirm highlighted item    |
| `Up/Down` | Navigate options (opens menu if closed)  |
| `Escape`  | Close menu                               |

## Theming

- `background` — Dropdown input background
- `input` — Trigger border color
- `foreground` — Text color
- `muted_foreground` — Placeholder and disabled text
- `border` — Menu border
- `radius` — Border radius

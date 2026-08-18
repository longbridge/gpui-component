---
title: DropdownButton
description: A DropdownButton is a combination of a button and a trigger button. It allows us to display a dropdown menu when the trigger is clicked, but the left Button can still respond to independent events.
---

# DropdownButton

A [DropdownButton] is a combination of a button and a trigger button. It allows us to display a dropdown menu when the trigger is clicked, but the left Button can still respond to independent events.

The variant set with [ButtonVariants] and the size set with [Sizable] apply to both halves. Everything else — label, icon, tooltip, loading state, click handler — belongs to the inner [Button].

## Import

```rust
use gpui_component::button::{Button, DropdownButton};
```

## Usage

```rust
use gpui::Anchor;

DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
            .menu("Option 2", Box::new(MyAction))
            .separator()
            .menu("Option 3", Box::new(MyAction))
    })
```

### Variants

Same as [Button], DropdownButton supports different variants.

```rust
DropdownButton::new("dropdown")
    .primary()
    .button(Button::new("btn").label("Primary"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

A `ghost` button that is not selected renders as two separate buttons instead of a joined pair, which reads better in a toolbar.

### Inner button options

Options that belong to the action itself go on the inner [Button]:

```rust
DropdownButton::new("dropdown")
    .button(
        Button::new("btn")
            .label("Save")
            .compact()
            .loading(is_saving)
            .tooltip("Save the current view")
            .on_click(|_, _, _| println!("Saved")),
    )
    .dropdown_menu(|menu, _, _| {
        menu.menu("Save as…", Box::new(MyAction))
    })
```

Leaving the variant unset on the DropdownButton uses the inner button's variant for both halves. Leaving the size unset keeps the inner button's own size.

### With custom anchor

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Anchor::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### Composing with ButtonGroup

A DropdownButton is a thin wrapper over [ButtonGroup], which holds a button that opens a menu directly. Use the group when the split needs more than two members, or when the halves need unrelated styling:

```rust
ButtonGroup::new("save")
    .child(Button::new("save").label("Save").on_click(|_, _, _| {}))
    .child(
        Button::new("save-options")
            .dropdown_caret(true)
            .dropdown_menu(|menu, _, _| menu.menu("Save as…", Box::new(MyAction))),
    )
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[ButtonGroup]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonGroup.html
[ButtonVariants]: https://docs.rs/gpui-component/latest/gpui_component/button/trait.ButtonVariants.html
[DropdownButton]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.DropdownButton.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

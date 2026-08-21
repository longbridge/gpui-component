---
title: Editor
description: Source-code editor with syntax highlighting, gutter, folding, and decorations.
---

# Editor

`Editor` is the styled source-code control. Use [`Input`](./input.md) for
single-line values and [`Textarea`](./textarea.md) for ordinary multi-line text.

## Import

```rust
use gpui_component::input::{Editor, EditorState, TabSize};
```

## Basic usage

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .line_number(true)
        .folding(true)
        .tab_size(TabSize {
            tab_size: 4,
            hard_tabs: false,
        })
        .default_value("fn main() {\n    println!(\"Hello\");\n}")
});

Editor::new(&editor).h(px(320.))
```

The language set via `.language()` selects syntax highlighting. Enable the
matching Cargo feature, such as `tree-sitter-rust` or `tree-sitter-markdown`;
use `tree-sitter-languages` to bundle all built-in grammars.

## Editor options

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("json")
        .line_number(true)
        .folding(true)
        .show_whitespaces(true)
        .default_value(source)
});
```

## Decorations

```rust
let decorations = editor.update(cx, |state, cx| {
    state.create_decorations_collection(initial_decorations, cx)
});
```

Keep the returned `TextDecorationCollection` alive while the decorations are
needed. Its ranges follow subsequent text edits.

## Value and events

```rust
let source = editor.read(cx).value();

editor.update(cx, |state, cx| {
    state.set_value(new_source, window, cx);
});

cx.subscribe(&editor, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.source = state.read(cx).value();
        cx.notify();
    }
});
```

## Font

The editor paints its code in the theme's monospace font — `mono_font_family` at
`mono_font_size` — with rows 1.5 times the font size. That is only the default:
a text style set on the editor refines over it, and the gutter and row height
follow the size.

```rust
Editor::new(&editor).text_sm()

Editor::new(&editor)
    .font_family("JetBrains Mono")
    .text_size(px(15.))
```

These are the ordinary [`Styled`](https://docs.rs/gpui/latest/gpui/trait.Styled.html)
methods every element has, so `font_weight` and `line_height` work the same way.

## Appearance

```rust
Editor::new(&editor)
    .h(px(480.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("Rust source")
```

Use `readonly` to preview a file without allowing changes. Unlike `disabled`,
a read-only editor keeps the normal appearance and still can be focused,
selected, copied and searched, it only rejects the changes made by the user.
The programmatic APIs such as `set_value` keep working.

```rust
Editor::new(&editor).readonly(true)
```

Editor focus does not add the single-line Input focus-border treatment. The
gutter, current-line background, and scrollbars are painted as one aligned
editor surface.

Input-only adornments such as `prefix`, `suffix`, mask toggle, and clear button
are intentionally absent. Compose toolbars and actions around `Editor`.

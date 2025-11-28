---
title: Editor
description: Multi-line text input component with auto-resize, validation, and advanced editing features.
---

# Editor

A powerful multi-line text input component that extends the basic input functionality with support for multiple lines, auto-resizing, syntax highlighting, line numbers, and code editing features. Perfect for forms, code editors, and content editing.

## Import

```rust
use gpui_component::input::{InputState, Input};
```

## Usage

### Textarea

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .placeholder("Enter your message...")
);

Input::new(&state)
```

With fixed height Textarea:

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .rows(10) // Set number of rows
        .placeholder("Enter text here...")
);

Input::new(&state)
    .h(px(320.)) // Set explicit height
```

### AutoGrow

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .auto_grow(1, 5) // min_rows: 1, max_rows: 5
        .placeholder("Type here and watch it grow...")
);

Input::new(&state)
```

### CodeEditor

GPUI Component's `InputState` supports a code editor mode with syntax highlighting, line numbers, and search functionality.

It design for high performance and can handle large files efficiently. We
used [tree-sitter](https://tree-sitter.github.io/tree-sitter/) for syntax highlighting, and [ropey](https://github.com/cessen/ropey) for text storage and manipulation.

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .code_editor("rust") // Language for syntax highlighting
        .line_number(true) // Show line numbers
        .searchable(true) // Enable search functionality
        .default_value("fn main() {\n    println!(\"Hello, world!\");\n}")
);

Input::new(&state)
    .h_full() // Full height
```

#### Single Line Mode

Sometimes you may want to use the code editor features but restrict input to a single line, for example for code snippets or commands.

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .code_editor("rust")
        .multi_line(false) // Single line
        .default_value("println!(\"Hello, world!\");")
);

Input::new(&state)
```

### TabSize

```rust
use gpui_component::input::TabSize;

let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .tab_size(TabSize {
            tab_size: 4,
            hard_tabs: false, // Use spaces instead of tabs
        })
);

Input::new(&state)
```

### Searchable

The search feature allows for all multi-line inputs to support searching through the content using `Ctrl+F` (or `Cmd+F` on Mac).

It provides a search bar with options to navigate between matches and highlight them.

Use `searchable` method to enable:

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .searchable(true) // Enable Ctrl+F search
        .rows(15)
        .default_value("Search through this content...")
);

Input::new(&state)
```

### SoftWrap

By default multi-line inputs have soft wrapping enabled, meaning long lines will wrap to fit the width of the textarea.

You can disable soft wrapping to allow horizontal scrolling instead:

```rust
// With soft wrap (default)
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .soft_wrap(true)
        .rows(6)
);

// Without soft wrap (horizontal scrolling)
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .soft_wrap(false)
        .rows(6)
        .default_value("This is a very long line that will not wrap automatically but will show horizontal scrollbar instead.")
);
```

### Text Manipulation

```rust
// Insert text at cursor position
state.update(cx, |state, cx| {
    state.insert("inserted text", window, cx);
});

// Replace all content
state.update(cx, |state, cx| {
    state.replace("new content", window, cx);
});

// Set cursor position
state.update(cx, |state, cx| {
    state.set_cursor_position(Position { line: 2, character: 5 }, window, cx);
});

// Get cursor position
let position = state.read(cx).cursor_position();
println!("Line: {}, Column: {}", position.line, position.character);
```

### Validation

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .validate(|text, _| {
            // Validate that content is not empty and under 1000 chars
            !text.trim().is_empty() && text.len() <= 1000
        })
);

Input::new(&state)
```

### Handle Events

```rust
cx.subscribe_in(&state, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let content = state.read(cx).value();
            println!("Content changed: {} characters", content.len());
        }
        InputEvent::PressEnter { secondary } => {
            if secondary {
                println!("Shift+Enter pressed - insert line break");
            } else {
                println!("Enter pressed - could submit form");
            }
        }
        InputEvent::Focus => println!("Textarea focused"),
        InputEvent::Blur => println!("Textarea blurred"),
    }
});
```

### Disabled State

```rust
Input::new(&state)
    .disabled(true)
    .h(px(200.))
```

### Custom Styling

```rust
// Without default appearance
Input::new(&state)
    .appearance(false)
    .h(px(200.))

// Custom container styling
div()
    .bg(cx.theme().background)
    .border_2()
    .border_color(cx.theme().input)
    .rounded_lg()
    .p_4()
    .child(
        Input::new(&state)
            .appearance(false)
            .h(px(150.))
    )
```

## Examples

### Comment Box

```rust
struct CommentBox {
    state: Entity<InputState>,
    char_limit: usize,
}

impl CommentBox {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx|
            InputState::new(window, cx)
                .auto_grow(3, 8)
                .placeholder("Write your comment...")
                .validate(|text, _| text.len() <= 500)
        );

        Self {
            state,
            char_limit: 500,
        }
    }
}

impl Render for CommentBox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.state.read(cx).value();
        let char_count = content.len();
        let remaining = self.char_limit.saturating_sub(char_count);

        v_flex()
            .gap_2()
            .child(Input::new(&self.state))
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} characters remaining", remaining))
                    )
                    .child(
                        Button::new("submit")
                            .primary()
                            .disabled(char_count == 0 || char_count > self.char_limit)
                            .label("Post Comment")
                    )
            )
    }
}
```

### Code Editor with Language Selection

```rust
struct CodeEditor {
    editor: Entity<InputState>,
    language: String,
}

impl CodeEditor {
    fn set_language(&mut self, language: String, window: &mut Window, cx: &mut Context<Self>) {
        self.language = language.clone();
        self.editor.update(cx, |editor, cx| {
            editor.set_highlighter(language, cx);
        });
    }
}

impl Render for CodeEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child("Language:")
                    .child(
                        // Language selector dropdown would go here
                        div().child(self.language.clone())
                    )
            )
            .child(
                Input::new(&self.editor)
                    .h(px(400.))
                    .bordered(true)
            )
    }
}
```

### Text Editor with Toolbar

```rust
struct TextEditor {
    editor: Entity<InputState>,
}

impl TextEditor {
    fn format_bold(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| {
            if !editor.selected_range.is_empty() {
                let selected = editor.selected_text().to_string();
                editor.replace(&format!("**{}**", selected), window, cx);
            }
        });
    }
}

impl Render for TextEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_1()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("bold")
                            .ghost()
                            .icon(IconName::Bold)
                            .on_click(cx.listener(Self::format_bold))
                    )
                    .child(
                        Button::new("italic")
                            .ghost()
                            .icon(IconName::Italic)
                    )
            )
            .child(
                Input::new(&self.editor)
                    .h(px(300.))
            )
    }
}
```

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

### Basic Textarea

```rust
let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .placeholder("Enter your message...")
);

Input::new(&textarea)
```

### Fixed Height Textarea

```rust
let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .rows(10) // Set number of rows
        .placeholder("Enter text here...")
);

Input::new(&textarea)
    .h(px(320.)) // Set explicit height
```

### Auto-Resizing Textarea

```rust
let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .auto_grow(1, 5) // min_rows: 1, max_rows: 5
        .placeholder("Type here and watch it grow...")
);

Input::new(&textarea)
```

### With Default Content

```rust
let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .rows(6)
        .default_value("Hello World!\n\nThis is a multi-line textarea with default content.")
);

Input::new(&textarea)
```

### Code Editor Mode

#### Multi Line

```rust
let code_editor = cx.new(|cx|
    InputState::new(window, cx)
        .code_editor("rust") // Language for syntax highlighting
        .line_number(true) // Show line numbers
        .searchable(true) // Enable search functionality
        .default_value("fn main() {\n    println!(\"Hello, world!\");\n}")
);

Input::new(&code_editor)
    .h_full() // Full height
```

#### Single Line

```rust
let code_editor = cx.new(|cx|
    InputState::new(window, cx)
        .code_editor("rust") // Language for syntax highlighting
        .line_number(false) // Hide line numbers
        .indent_guides(false) // Hide indent guides
        .multi_line(false) // Single line
        .default_value("println!(\"Hello, world!\");")
);

Input::new(&code_editor)
```

### Textarea with Custom Tab Size

```rust
use gpui_component::input::TabSize;

let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .tab_size(TabSize {
            tab_size: 4,
            hard_tabs: false, // Use spaces instead of tabs
        })
);

Input::new(&textarea)
```

### Searchable Textarea

```rust
let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .searchable(true) // Enable Ctrl+F search
        .rows(15)
        .default_value("Search through this content...")
);

Input::new(&textarea)
```

### Soft Wrap Control

```rust
// With soft wrap (default)
let textarea_wrap = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .soft_wrap(true)
        .rows(6)
);

// Without soft wrap (horizontal scrolling)
let textarea_no_wrap = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .soft_wrap(false)
        .rows(6)
        .default_value("This is a very long line that will not wrap automatically but will show horizontal scrollbar instead.")
);

v_flex()
    .gap_4()
    .child(Input::new(&textarea_wrap))
    .child(Input::new(&textarea_no_wrap))
```

### Character Counting

```rust
struct TextareaView {
    textarea: Entity<InputState>,
}

impl Render for TextareaView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let char_count = self.textarea.read(cx).value().len();
        let max_chars = 500;

        v_flex()
            .gap_2()
            .child(
                Input::new(&self.textarea)
                    .h(px(120.))
            )
            .child(
                div()
                    .text_right()
                    .text_sm()
                    .text_color(if char_count > max_chars {
                        cx.theme().destructive
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(format!("{}/{}", char_count, max_chars))
            )
    }
}
```

### Text Manipulation

```rust
// Insert text at cursor position
textarea.update(cx, |input, cx| {
    input.insert("inserted text", window, cx);
});

// Replace all content
textarea.update(cx, |input, cx| {
    input.replace("new content", window, cx);
});

// Set cursor position
textarea.update(cx, |input, cx| {
    input.set_cursor_position(Position { line: 2, character: 5 }, window, cx);
});

// Get cursor position
let position = textarea.read(cx).cursor_position();
println!("Line: {}, Column: {}", position.line, position.character);
```

### Validation

```rust
let textarea = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .validate(|text, _| {
            // Validate that content is not empty and under 1000 chars
            !text.trim().is_empty() && text.len() <= 1000
        })
);

Input::new(&textarea)
```

### Handle Events

```rust
cx.subscribe_in(&textarea, window, |view, state, event, window, cx| {
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
Input::new(&textarea)
    .disabled(true)
    .h(px(200.))
```

### Custom Styling

```rust
// Without default appearance
Input::new(&textarea)
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
        Input::new(&textarea)
            .appearance(false)
            .h(px(150.))
    )
```

## API Reference

### InputState (Multi-line Methods)

| Method                                 | Description                                      |
| -------------------------------------- | ------------------------------------------------ |
| `multi_line()`                         | Enable multi-line mode with 2 rows default       |
| `auto_grow(min, max)`                  | Enable auto-resize between min and max rows      |
| `code_editor(language)`                | Enable code editor mode with syntax highlighting |
| `rows(count)`                          | Set number of visible rows                       |
| `tab_size(TabSize)`                    | Configure tab behavior                           |
| `searchable(bool)`                     | Enable/disable search (Ctrl+F)                   |
| `soft_wrap(bool)`                      | Enable/disable text wrapping                     |
| `line_number(bool)`                    | Show/hide line numbers (code editor only)        |
| `cursor_position()`                    | Get current cursor position as `Position`        |
| `set_cursor_position(pos, window, cx)` | Set cursor to specific line/column               |
| `insert(text, window, cx)`             | Insert text at cursor                            |
| `replace(text, window, cx)`            | Replace all content                              |

### Input (Multi-line Methods)

| Method      | Description                |
| ----------- | -------------------------- |
| `h(height)` | Set explicit height        |
| `h_full()`  | Take full available height |

### Position

| Field       | Description                        |
| ----------- | ---------------------------------- |
| `line`      | 0-based line number                |
| `character` | 0-based character position in line |

### TabSize

| Field       | Description                                |
| ----------- | ------------------------------------------ |
| `tab_size`  | Number of spaces per tab (default: 2)      |
| `hard_tabs` | Use actual tab characters (default: false) |

### Keyboard Shortcuts

| Shortcut      | Action                      |
| ------------- | --------------------------- |
| `Enter`       | Insert new line             |
| `Shift+Enter` | Insert new line (secondary) |
| `Tab`         | Indent line/selection       |
| `Shift+Tab`   | Outdent line/selection      |
| `Ctrl/Cmd+A`  | Select all                  |
| `Ctrl/Cmd+Z`  | Undo                        |
| `Ctrl/Cmd+Y`  | Redo                        |
| `Ctrl/Cmd+F`  | Open search (if enabled)    |
| `Ctrl/Cmd+[`  | Outdent block               |
| `Ctrl/Cmd+]`  | Indent block                |

## Examples

### Comment Box

```rust
struct CommentBox {
    textarea: Entity<InputState>,
    char_limit: usize,
}

impl CommentBox {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let textarea = cx.new(|cx|
            InputState::new(window, cx)
                .auto_grow(3, 8)
                .placeholder("Write your comment...")
                .validate(|text, _| text.len() <= 500)
        );

        Self {
            textarea,
            char_limit: 500,
        }
    }
}

impl Render for CommentBox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.textarea.read(cx).value();
        let char_count = content.len();
        let remaining = self.char_limit.saturating_sub(char_count);

        v_flex()
            .gap_2()
            .child(Input::new(&self.textarea))
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

## Performance Notes

- Optimized for large text content (up to 200K lines in code editor mode)
- Efficient text wrapping and line measurement
- Virtual scrolling for long documents
- Minimal re-renders on text changes
- Efficient syntax highlighting with tree-sitter
- Smart auto-grow calculations

## Best Practices

1. **Auto-resize**: Use `auto_grow()` for dynamic content like comments or messages
2. **Fixed size**: Use `multi_line().rows(n)` for consistent layouts like forms
3. **Code editing**: Use `code_editor()` for syntax-aware editing
4. **Validation**: Always validate long-form content on the client side
5. **Character limits**: Show character counters for user guidance
6. **Search**: Enable search for long content areas
7. **Soft wrap**: Disable for code, enable for prose

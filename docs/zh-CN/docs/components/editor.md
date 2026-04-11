---
title: Editor
description: 支持自动增高、校验和高级编辑能力的多行文本输入组件。
---

# Editor

Editor 是一个功能更强的多行文本输入组件，在基础输入能力之上增加了多行编辑、自动增高、语法高亮、行号和代码编辑功能，适合表单、代码编辑器和内容编辑场景。

## 导入

```rust
use gpui_component::input::{InputState, Input};
```

## 用法

### Textarea

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .placeholder("Enter your message...")
);

Input::new(&state)
```

固定高度的 Textarea：

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .rows(10)
        .placeholder("Enter text here...")
);

Input::new(&state)
    .h(px(320.))
```

### AutoGrow

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .auto_grow(1, 5)
        .placeholder("Type here and watch it grow...")
);

Input::new(&state)
```

### CodeEditor

GPUI Component 的 `InputState` 支持代码编辑器模式，可提供语法高亮、行号和搜索功能。

它面向高性能场景，能够高效处理大文件。语法高亮基于 [tree-sitter](https://tree-sitter.github.io/tree-sitter/)，文本存储和编辑基于 [ropey](https://github.com/cessen/ropey)。

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .code_editor("rust")
        .line_number(true)
        .searchable(true)
        .show_whitespaces(true)
        .default_value("fn main() {\n    println!(\"Hello, world!\");\n}")
);

Input::new(&state)
    .h_full()
```

#### 单行模式

有时你希望保留代码编辑能力，但只允许输入一行，例如命令或代码片段：

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .code_editor("rust")
        .multi_line(false)
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
            hard_tabs: false,
        })
);

Input::new(&state)
```

### Searchable

所有多行输入都可以通过 `searchable(true)` 开启搜索能力，并支持 `Ctrl+F` 或 macOS 上的 `Cmd+F`。

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .searchable(true)
        .rows(15)
        .default_value("Search through this content...")
);

Input::new(&state)
```

### SoftWrap

默认情况下，多行输入会启用软换行，长文本会自动换到下一行。你也可以关闭软换行，改为横向滚动：

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .soft_wrap(true)
        .rows(6)
);

let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .soft_wrap(false)
        .rows(6)
        .default_value("This is a very long line that will not wrap automatically but will show horizontal scrollbar instead.")
);
```

### 文本操作

```rust
state.update(cx, |state, cx| {
    state.insert("inserted text", window, cx);
});

state.update(cx, |state, cx| {
    state.replace("new content", window, cx);
});

state.update(cx, |state, cx| {
    state.set_cursor_position(Position { line: 2, character: 5 }, window, cx);
});

let position = state.read(cx).cursor_position();
println!("Line: {}, Column: {}", position.line, position.character);
```

### 校验

```rust
let state = cx.new(|cx|
    InputState::new(window, cx)
        .multi_line(true)
        .validate(|text, _| {
            !text.trim().is_empty() && text.len() <= 1000
        })
);

Input::new(&state)
```

### 处理事件

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

### 禁用状态

```rust
Input::new(&state)
    .disabled(true)
    .h(px(200.))
```

### 自定义样式

```rust
Input::new(&state)
    .appearance(false)
    .h(px(200.))

div()
    .bg(cx.theme().background)
    .border_2()
    .border_color(cx.theme().input)
    .rounded(cx.theme().radius_lg)
    .p_4()
    .child(
        Input::new(&state)
            .appearance(false)
            .h(px(150.))
    )
```

## 示例

### 评论框

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

### 带语言选择的代码编辑器

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

### 带工具栏的文本编辑器

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

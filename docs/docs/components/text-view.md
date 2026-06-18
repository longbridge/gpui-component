---
title: TextView
description: Renders Markdown and HTML text with optional custom Markdown plugins.
---

# TextView

`TextView` renders formatted text in GPUI. It supports Markdown and simple HTML, text selection, code block actions, and custom Markdown plugins for project-specific syntax.

## Import

```rust
use gpui_component::text::{markdown, TextView};
```

## Usage

### Markdown

Use the `markdown` helper when you only need to render Markdown text:

```rust
use gpui_component::text::markdown;

markdown("# Hello\n\nThis is **Markdown**.")
    .selectable(true)
    .scrollable(true)
```

You can also construct a `TextView` directly when you need a stable id:

```rust
use gpui_component::text::TextView;

TextView::markdown("preview", markdown_source)
    .selectable(true)
```

### HTML

```rust
TextView::html("html-preview", "<strong>Hello</strong>")
```

## Markdown Plugins

Use `.plugin(...)` to support custom Markdown formats. A plugin owns both parsing and rendering, so callers only need to attach it to the `TextView`:

```rust
markdown(source)
    .plugin(TickerPlugin::new())
```

A Markdown plugin implements `MarkdownPlugin`:

```rust
use gpui::{App, IntoElement, ParentElement as _, Window};
use gpui_component::text::{
    markdown_ast, MarkdownNode, MarkdownParseContext, MarkdownParseOptions,
    MarkdownPlugin,
};

struct TickerNode {
    symbol: String,
}

struct TickerPlugin;

impl TickerPlugin {
    fn new() -> Self {
        Self
    }
}

impl MarkdownPlugin for TickerPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "ticker"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Paragraph(paragraph) = node else {
            return None;
        };
        let [markdown_ast::Node::Text(text)] = paragraph.children.as_slice() else {
            return None;
        };
        let symbol = text.value.strip_prefix('$')?;

        Some(
            MarkdownNode::new(
                "ticker",
                TickerNode {
                    symbol: symbol.to_string(),
                },
            )
            .text(format!("${symbol}"))
            .markdown(cx.node_source(node).unwrap_or(text.value.as_str())),
        )
    }

    fn render(
        &self,
        node: &MarkdownNode,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        let ticker = node.data::<TickerNode>().expect("ticker node data");

        gpui::div().child(format!("${}", ticker.symbol))
    }
}
```

Then attach it to a Markdown `TextView`:

```rust
markdown("$AAPL.US")
    .plugin(TickerPlugin::new())
```

## MarkdownNode

`MarkdownNode` is the neutral data passed between `parse` and `render`.

```rust
MarkdownNode::new("ticker", TickerNode { symbol })
    .text("$AAPL.US")
    .markdown("$AAPL.US")
```

- `name` is the stable node name used to match the renderer.
- `data` is typed parser output read with `node.data::<T>()`.
- `text` is the plain text representation used by selection and fallback rendering.
- `markdown` is the Markdown representation used when the document is serialized back to Markdown.

## Block And Inline Plugins

Markdown plugins can produce block nodes, inline nodes, or both. Return `true` from `is_block()` for block rendering, and override `is_inline()` when the same plugin also handles inline syntax:

```rust
fn is_block(&self) -> bool {
    true
}

fn is_inline(&self) -> bool {
    true
}
```

Plugins can also adjust parser constructs before parsing:

```rust
fn parse_options(&self, options: &mut MarkdownParseOptions) {
    options.constructs.math_flow = true;
    options.constructs.math_text = true;
}
```

## Code Block Actions

You can render controls for Markdown code blocks:

```rust
markdown(source)
    .code_block_actions(|code_block, _window, _cx| {
        gpui::div().child(format!("Run {}", code_block.lang().unwrap_or_default()))
    })
```

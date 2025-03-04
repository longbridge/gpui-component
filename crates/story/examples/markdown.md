# Hello, **World**!

This is first paragraph, there have **BOLD**, _italic_, and ~strikethrough~, `code` text.

> Blockquote: More complex nested inline style like **bold: _italic_**.
> This is second paragraph, it includes a block quote.

This is an additional demonstration paragraph in English demonstrating more content for [Markdown GFM](https://github.github.com/gfm/). It includes various stylistic elements and plain text.

![Img](https://miro.medium.com/v2/resize:fit:1400/format:webp/1*WgEz5f3n3lD7MfC7NeQGOA.jpeg)

这是一个中文演示段落，用于展示更多的 [Markdown GFM](https://github.github.com/gfm/) 内容。您可以在此尝试使用使用**粗体**、*斜体*和`代码`等样式。これは日本語のデモ段落です。Markdown の多言語サポートを示すためのテキストが含まれています。例えば、、**ボールド**、_イタリック_、および`コード`のスタイルなどを試すことができます。

### Code block

```rust
struct Repository {
    /// Name of the repository.
    name: String,
}

fn main() {
    let _ = Repository {
        name: "GPUI Component".to_string(),
    };

    println!("Hello, World!");
}
```

---

## Heading for [Links](https://www.google.com)

Here is a link to [Google](https://www.google.com), and another to [Rust](https://www.rust-lang.org).

### Images

![](https://miro.medium.com/v2/resize:fit:1400/format:webp/0*fCBw3AjH4o9SO03D)

#### SVG

![Rust](https://www.rust-lang.org/logos/rust-logo-blk.svg)

### Table

| Header 1 | Centered | Header 3                             | Align Right |
| -------- | :------: | ------------------------------------ | ----------: |
| Cell 0   |  Cell 1  | This is a long cell with line break. |      Cell 3 |
| Row 2    |  Row 2   | Row 2<br>[Link](https://github.com)  |       Row 2 |
| Row 3    | **Bold** | Row 3                                |       Row 3 |

#### Lists

##### Bulleted List

- Bullet 1, this is very long and needs to be wrapped to the next line, display should be wrapped to the next line as well.
- Bullet 2, the second bullet item is also long and needs to be wrapped to the next line.
  - Bullet 2.1
    - Bullet 2.1.1
      - Bullet 2.1.1.1
    - Bullet 2.1.2
  - Bullet 2.2
- Bullet 3

##### Numbered List

1. Numbered item 1
   1. Numbered item 1.1
      1. Numbered item 1.1.1
   1. Numbered item 1.2
2. Numbered item 2
3. Numbered item 3

##### To-Do List

- [x] Task 1, a long long text task, this line is very long and needs to be wrapped to the next line, display should be wrapped to the next line as well.
- [ ] Task 2, going to do something if there is a long text that needs to be wrapped to the next line.
- [ ] Task 3

### HTML

#### Paragraph and Text

<div>
    Here is a test in div.
    <p>This is a paragraph inside a div element, have <a href="https://google.com">link</a>, <strong>bold</strong>, <em>italic</em>, and <code>code</code> text.</p>
    <div>
        <p>This is second paragraph.</p>
    </div>
    A text after div.
</div>

#### List

<ol>
<li>Numbered item 1</li>
<li>Numbered item 2</li>
</ol>

<ul>
<li>Bullet 1</li>
<li>Bullet 2</li>
</ul>

#### Table

<table>
<thead>
<tr>
<td>Head 1</td>
<td>Head 2</td>
</tr>
</thead>
<tbody>
<tr>
<td><strong>Cell</strong> 1</td>
<td>Cell 2</td>
</tr>
<tr>
<td>Cell 3</td>
<td>Cell 4</td>
</tr>
</tbody>
</table>

#### Image

<img src="https://www.rust-lang.org/logos/rust-logo-blk.svg" alt="Rust" width="100" height="100" />
<img src="https://www.rust-lang.org/logos/rust-logo-blk.svg" alt="Rust" width="100%" />
<img src="https://www.rust-lang.org/logos/rust-logo-blk.svg" alt="Rust" style="width:100%" />

## Unsupported

### HTML

<details>
<summary>Click to expand</summary>
<div>
    <p>This is a paragraph <a href="https://google.com">inside</a> a details element.</p>
    <p>This is second paragraph.</p>
</div>
</details>

### Math

This is an inline math $x^2 + y^2 = z^2$.

This is a block math:

$$
\begin{aligned}
x^2 + y^2 &= z^2 \\
x^3 + y^3 &= z^3
\end{aligned}
$$

This is final paragraph, it includes a code block and a list of items.

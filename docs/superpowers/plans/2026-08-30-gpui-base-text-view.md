# gpui-base TextView and Selectable Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `gpui-base` independently provide selectable plain text and the complete HTML/Markdown `TextView`, including usable default styling, selection, and copying without enabling syntax highlighting by default.

**Architecture:** `gpui-base::text` becomes the canonical owner of parsing, document state, rendering, style, and the `TextSelection` adapter. `SelectableText` is a smaller Base element built directly on `TextSelection`; `gpui-component::text` becomes a compatibility re-export plus a component-theme adapter. Code blocks have readable neutral defaults, while syntax highlighting is injected by consumers and keeps language/highlighter dependencies out of Base.

**Tech Stack:** Rust, GPUI `Element`/`Entity`, `gpui-base::TextSelection`, `markdown-rs`, `html5ever`, `markup5ever_rcdom`, Cargo workspace tests.

**Spec:** `docs/superpowers/specs/2026-08-30-gpui-base-text-view-design.md`

## Global Constraints

- `TextViewStyle::default()` must be complete and usable in an application that depends only on `gpui-base`.
- Keep the existing `gpui_base::Selectable` controlled-state trait unchanged; name the text element `SelectableText`.
- `gpui-base` must never depend on or import `gpui-component`.
- Preserve HTML, Markdown, tables, images, links, code blocks, plugins, selection formats, scrolling, and max-line clamping.
- Do not enable syntax highlighting by default or move tree-sitter language features into `gpui-base`; expose injection hooks instead.
- Preserve the current `gpui_component::text` public constructors and primary builder APIs through re-exports.
- Keep one window-scoped selection coordinator: `TextSelection`.
- Work in the current checkout; do not create a worktree.

---

## File structure

- Create `crates/base/src/selectable_text.rs`: focused plain-text `Element` and selection painting.
- Create `crates/base/src/text/` by moving the current `crates/ui/src/text/` module tree: canonical rich-text implementation.
- Modify `crates/base/src/lib.rs`: initialize and export selectable/rich-text APIs.
- Modify `crates/base/Cargo.toml`: own Markdown and HTML parser dependencies only; do not add tree-sitter dependencies or features.
- Replace `crates/ui/src/text/mod.rs` with compatibility re-exports; delete its moved implementation files only after tests pass from Base.
- Modify `crates/ui/Cargo.toml`: remove parser ownership while retaining existing highlighter features for component/editor consumers.
- Modify `crates/ui/src/lib.rs` and theme initialization only where needed for the component-style adapter.
- Modify `crates/base/examples/showcase/components/text_selection.rs`: consume `SelectableText` instead of maintaining a private reference implementation.

### Task 1: Add the Base SelectableText reference component

**Files:**
- Create: `crates/base/src/selectable_text.rs`
- Modify: `crates/base/src/lib.rs`
- Modify: `crates/base/examples/showcase/components/text_selection.rs`
- Test: `crates/base/src/selectable_text.rs`

**Interfaces:**
- Consumes: `TextSelectionHandle`, `TextSelectionRegistration`, `TextSelectionRun`, and `TextSelectionProjection` from `crates/base/src/text_selection.rs`.
- Produces: `SelectableText::new(id, text)`, `SelectableText::with_handle(id, handle, text)`, `SelectableText::document_order(u64)`, `SelectableText::text_style(TextStyle)`, and root re-export `gpui_base::SelectableText`.

- [ ] **Step 1: Write failing constructor and projection tests**

Add tests in `crates/base/src/selectable_text.rs` that construct a visual root with `TextSelectionLayer`, shape `"first selectable text"`, drag from inside `first` through `selectable`, and assert:

```rust
assert_eq!(
    cx.update(|window, cx| TextSelection::selected_text(window, cx)),
    "first selectable"
);
```

Add a second root with two elements sharing a handle and document orders `10` and `20`; drag across both and assert the selected text is joined in visual/document order. Add a copy test that focuses the root, dispatches Base's copy action, and asserts `cx.read_from_clipboard().unwrap().text()` matches `TextSelection::selected_text`.

- [ ] **Step 2: Run the focused test and verify the API is absent**

Run: `cargo test -p gpui-base selectable_text --lib`

Expected: compilation fails because `SelectableText` has not been defined/exported.

- [ ] **Step 3: Implement the element around TextSelection**

Implement this public shape in `crates/base/src/selectable_text.rs`:

```rust
pub struct SelectableText {
    id: ElementId,
    handle: Option<TextSelectionHandle>,
    text: SharedString,
    styled_text: StyledText,
    document_order: u64,
    selection_color: Option<Hsla>,
}

impl SelectableText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self;
    pub fn with_handle(
        id: impl Into<ElementId>,
        handle: TextSelectionHandle,
        text: impl Into<SharedString>,
    ) -> Self;
    pub fn document_order(mut self, order: u64) -> Self;
    pub fn text_style(mut self, style: TextStyle) -> Self;
    pub fn selection_color(mut self, color: Hsla) -> Self;
}
```

Use `Window::with_element_state` keyed by `id` to retain a generated `TextSelectionHandle` for `new`; `with_handle` bypasses this local state. During prepaint, register the hitbox, bounds, text bounds, and document order. During paint, update one `TextSelectionRun`, paint every projected range before painting glyphs, and request refresh when selected text changes. When `selection_color` is absent, use the same neutral translucent blue defined by `TextViewStyle::default().selection`; do not depend on component theme state.

Handle the existing Base copy action on the focus-tracked wrapper and write only non-empty `TextSelection::selected_text(window, cx)` to the clipboard. Reuse the tested multi-line selection-quad geometry from the showcase implementation, moving it into this module rather than duplicating it.

- [ ] **Step 4: Replace the showcase's private element**

Delete `PlainSelectableText` and `selection_quad_bounds` from `crates/base/examples/showcase/components/text_selection.rs`. Render the exported component with the existing retained handles:

```rust
SelectableText::with_handle(
    ("selection-paragraph", document_order),
    selection.clone(),
    text,
)
.document_order(document_order)
```

Keep the showcase's window-level `TextSelectionLayer` and selection-status display so it remains executable documentation.

- [ ] **Step 5: Run Base tests and commit**

Run: `cargo test -p gpui-base selectable_text --lib`

Run: `cargo check -p gpui-base --example components`

Expected: both pass; copy and cross-run tests assert real selected output.

Commit:

```bash
git add crates/base/src/selectable_text.rs crates/base/src/lib.rs crates/base/examples/showcase/components/text_selection.rs
git commit -m "feat(base): add selectable text element"
```

### Task 2: Move parsing, document state, and selection adaptation to Base

**Files:**
- Move: `crates/ui/src/text/document.rs` → `crates/base/src/text/document.rs`
- Move: `crates/ui/src/text/format/` → `crates/base/src/text/format/`
- Move: `crates/ui/src/text/markdown_ext.rs` → `crates/base/src/text/markdown_ext.rs`
- Move: `crates/ui/src/text/selection.rs` → `crates/base/src/text/selection.rs`
- Move: `crates/ui/src/text/selection_adapter.rs` → `crates/base/src/text/selection_adapter.rs`
- Move: `crates/ui/src/text/state.rs` → `crates/base/src/text/state.rs`
- Move: `crates/ui/src/text/utils.rs` → `crates/base/src/text/utils.rs`
- Create: `crates/base/src/text/mod.rs`
- Modify: `crates/base/Cargo.toml`
- Modify: `crates/base/src/lib.rs`
- Test: parsing/state tests moved with their owning modules

**Interfaces:**
- Consumes: existing Base `TextSelection` types.
- Produces: `gpui_base::text::{TextViewState, SelectionFormat, MarkdownExtensions, MarkdownNode, MarkdownPlugin, TableData}` and internal `ParsedDocument`/`TextViewSelectionAdapter`.

- [ ] **Step 1: Add Base-only parser tests**

Move the existing Markdown/HTML tests with their modules. Add one test per format that constructs `TextViewState`, parses representative input, and asserts the plain selection/document output includes:

```text
Markdown: heading, emphasized text, link label, list item, fenced code, table cells
HTML: h1, strong text, anchor label, ul/li, pre/code, table cells, img alt text
```

Add the current word-boundary tests unchanged under Base so double/triple-click behavior retains UTF-8 coverage.

- [ ] **Step 2: Run the new Base test target and verify failure**

Run: `cargo test -p gpui-base text:: --lib`

Expected: compilation fails because `gpui_base::text` is not defined.

- [ ] **Step 3: Move parser dependencies and source modules**

Add to Base and remove from UI:

```toml
markdown = { version = "1.0.0", features = ["serde"] }
html5ever = "0.27"
markup5ever_rcdom = "0.3.0"
```

Create `crates/base/src/text/mod.rs` declaring only the moved modules at this stage. Re-export the public parser/plugin/state types. Change former `crate::text` paths to the new Base module and direct `gpui_base::TextSelection*` imports to `crate::TextSelection*`. Do not move any `crate::highlighter` import in this task; Task 3 removes those imports when it moves `node.rs`.

Move color parsing used by HTML inline styles into a focused private helper in `crates/base/src/text/format/html.rs`, preserving accepted syntax from the current `try_parse_color` tests. Use GPUI `Hsla` results and do not import UI theme helpers such as `yellow(...)`.

- [ ] **Step 4: Register text state in Base initialization**

Expose `pub mod text;` from `crates/base/src/lib.rs` and call `text::init(cx)` from `gpui_base::init`. Ensure repeated `init` remains safe, matching current UI initialization behavior.

- [ ] **Step 5: Run parsing and state tests and commit**

Run: `cargo test -p gpui-base text::format --lib`

Run: `cargo test -p gpui-base text::selection --lib`

Run: `cargo test -p gpui-base text::state --lib`

Expected: representative Markdown/HTML and UTF-8 word-selection tests pass without linking `gpui-component`.

Commit:

```bash
git add crates/base/src/text crates/base/src/lib.rs crates/base/Cargo.toml crates/ui/Cargo.toml
git commit -m "refactor(base): move rich text parsing and state"
```

### Task 3: Move TextView rendering and provide complete Base defaults

**Files:**
- Move: `crates/ui/src/text/inline.rs` → `crates/base/src/text/inline.rs`
- Move: `crates/ui/src/text/inline_flow.rs` → `crates/base/src/text/inline_flow.rs`
- Move: `crates/ui/src/text/node.rs` → `crates/base/src/text/node.rs`
- Move: `crates/ui/src/text/style.rs` → `crates/base/src/text/style.rs`
- Move: `crates/ui/src/text/text_view.rs` → `crates/base/src/text/text_view.rs`
- Modify: `crates/base/src/text/mod.rs`
- Modify: `crates/base/src/theme.rs`
- Test: `crates/base/src/text/text_view.rs` and moved node/layout tests

**Interfaces:**
- Consumes: Task 2 parser/state, Base theme tokens, Base scrolling, and TextSelection.
- Produces: `TextView`, `TextViewStyle`, `TextViewPlugin`, `Text`, `markdown(source)`, `html(source)`, existing action/link/plugin builders, and an opt-in `code_block_highlighter` callback.

- [ ] **Step 1: Write Base-only default-style rendering tests**

Create a `TextViewTestRoot` in Base that calls only `gpui_base::init`, wraps content with `TextSelectionLayer`, and renders both:

```rust
TextView::markdown("markdown", "# Heading\n\nA [link](https://example.com).\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |")
    .selectable(true)
```

and:

```rust
TextView::html("html", "<h1>Heading</h1><p><strong>body</strong> <a href='https://example.com'>link</a></p><pre><code>code</code></pre>")
    .selectable(true)
```

After drawing, assert non-zero layout bounds and that `TextViewStyle::default()` has explicit non-empty/default values for paragraph gap, heading base size, code-block background/border refinement, inline-code style, table/head/cell refinement, link color, and selection color. Add a code-block test that records the styled runs used during paint and asserts the default list is empty.

- [ ] **Step 2: Run Base text-view tests and verify failure**

Run: `cargo test -p gpui-base text_view --lib`

Expected: compilation fails because rendering modules and public `TextView` do not yet live in Base.

- [ ] **Step 3: Define the complete default style contract**

Expand the moved `TextViewStyle` so it owns all presentation inputs currently read directly from `cx.theme()` in `node.rs`, `inline.rs`, and `text_view.rs`. The public fields/builders must include semantic values for:

```rust
pub struct TextViewStyle {
    pub paragraph_gap: Rems,
    pub heading_base_font_size: Pixels,
    pub heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync>>,
    pub foreground: Hsla,
    pub muted_foreground: Hsla,
    pub link: Hsla,
    pub selection: Hsla,
    pub block_quote_border: Hsla,
    pub inline_code: HighlightStyle,
    pub code_block: StyleRefinement,
    pub table: StyleRefinement,
    pub table_head: StyleRefinement,
    pub table_cell: StyleRefinement,
    pub horizontal_rule: Hsla,
    pub is_dark: bool,
}
```

Implement `Default` with stable neutral light values inside Base: foreground `hsla(0.62, 0.20, 0.16, 1.0)`, muted foreground `hsla(0.62, 0.10, 0.46, 1.0)`, link `hsla(0.60, 0.75, 0.48, 1.0)`, selection `hsla(0.58, 0.85, 0.62, 0.35)`, code background `hsla(0.62, 0.12, 0.95, 1.0)`, and border/rule `hsla(0.62, 0.10, 0.86, 1.0)`. Use Base typography tokens for monospace family/size and Base spacing/radius defaults for code/table refinements. `Default` must not inspect an `App`, highlighter registry, or component global. Add `TextViewStyle::from_theme(&gpui_base::Theme)` for callers that want Base semantic tokens, but constructors continue to store and use `TextViewStyle::default()` so Base-only rendering is readable even when the global Base theme has not been customized.

- [ ] **Step 4: Move rendering and remove component-only dependencies**

Move the remaining files and update imports. Replace:

- `crate::tooltip::Tooltip` with an optional title renderer callback; absent renderer means no overlay.
- `crate::{Icon, IconName}` code-block decorations with no built-in action; preserve `code_block_actions` for caller-provided elements.
- component `ScrollableElement` with the existing Base/GPUI scroll handle and list behavior.
- every `cx.theme()` component token lookup with the resolved `TextViewStyle` value.
- `crate::input::Copy` with the Base copy action already used by Input/SelectableText.
- `LanguageRegistry`, `SyntaxHighlighter`, and cached `HighlightTheme` state with an optional callback stored on `TextView` and passed through `NodeContext`.

Keep existing `code_block_actions`, `table_actions`, `on_link_click`, Markdown extension/plugin, max-lines, selection-format, and selectable builder signatures. Add these two explicit opt-in hooks:

```rust
pub fn link_title_renderer<F, E>(self, renderer: F) -> Self
where
    F: Fn(&SharedString, &mut Window, &mut App) -> E + Send + Sync + 'static,
    E: IntoElement;

pub fn code_block_highlighter<F>(self, highlighter: F) -> Self
where
    F: Fn(&CodeBlock) -> Vec<(Range<usize>, HighlightStyle)> + Send + Sync + 'static;
```

The highlighter callback receives the code and language through `CodeBlock::{code, lang}` and returns byte ranges relative to `CodeBlock::code()`. Validate ranges before passing them to `StyledText`; discard ranges whose start exceeds end or whose end exceeds the code length. With no callback, pass `Vec::new()` and render unhighlighted monospace code.

- [ ] **Step 5: Export helpers and run rendering/selection tests**

Finish `crates/base/src/text/mod.rs` with the current `markdown`, `html`, and `Text` APIs. Re-export `html`, `markdown`, `CodeBlock`, `MarkdownExtensions`, `MarkdownNode`, `MarkdownPlugin`, `SelectionFormat`, `TableData`, `Text`, `TextView`, `TextViewPlugin`, `TextViewState`, and `TextViewStyle` from `crates/base/src/lib.rs`.

Run: `cargo test -p gpui-base text_view --lib`

Run: `cargo test -p gpui-base text::node --lib`

Run: `cargo test -p gpui-base text_selection --lib`

Expected: Base-only rich text renders using defaults and selection/copy tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/base/src/text crates/base/src/theme.rs crates/base/src/lib.rs
git commit -m "feat(base): add HTML and Markdown TextView"
```

### Task 4: Replace gpui-component TextView with a compatibility facade

**Files:**
- Replace: `crates/ui/src/text/mod.rs`
- Delete after replacement: remaining implementation files under `crates/ui/src/text/`
- Modify: `crates/ui/src/lib.rs`
- Modify: `crates/ui/src/theme/mod.rs` or a new focused `crates/ui/src/text/theme_adapter.rs`
- Test: `crates/ui/src/text/mod.rs`

**Interfaces:**
- Consumes: Task 3 `gpui_base::text` API.
- Produces: source-compatible `gpui_component::text::*`, `gpui_component::{markdown, html, TextView, TextViewState, TextViewStyle}`, and component-theme style adaptation.

- [ ] **Step 1: Add compatibility compile tests before deleting UI code**

Add tests that assign component-path values to Base-path types:

```rust
let component: gpui_component::TextView =
    gpui_component::text::markdown("# compatible");
let _: gpui_base::text::TextView = component;

let style: gpui_component::text::TextViewStyle = Default::default();
let _: gpui_base::text::TextViewStyle = style;
```

Exercise `.selectable(true)`, `.selection_format(SelectionFormat::Source)`, `.scrollable(true)`, `.max_lines(3)`, `.code_block_actions(...)`, `.table_actions(...)`, `.on_link_click(...)`, and Markdown plugin builders from the component path so the facade proves the principal API surface.

- [ ] **Step 2: Install re-exports and verify failures identify missing compatibility**

Replace the implementation module with:

```rust
pub use gpui_base::text::*;
```

Update `crates/ui/src/lib.rs` root re-exports to source the same Base types. Run `cargo test -p gpui-component text:: --lib` and use compile errors as the exhaustive list of crate-private paths that must become Base-private or explicit adapter APIs; do not restore duplicate UI implementation files.

- [ ] **Step 3: Add the component theme adapter**

Implement a focused adapter function without changing Base ownership:

```rust
pub fn text_view_style(theme: &crate::Theme) -> gpui_base::text::TextViewStyle {
    gpui_base::text::TextViewStyle::default()
        .foreground(theme.foreground)
        .muted_foreground(theme.muted_foreground)
        .link(theme.link)
        .selection(theme.selection)
}
```

Do not install syntax highlighting in this adapter. A component consumer that wants it explicitly calls `.code_block_highlighter(...)` using `gpui_component::highlighter`; Base and its default remain independent of that module.

- [ ] **Step 4: Move window-selection integration tests to their owner**

Move Base-only tests from `crates/ui/src/text/window_selection.rs` into `crates/base/src/text/window_selection.rs`. Keep only tests that specifically exercise `Root`/`WindowExt` integration in UI. Update those imports to the re-exported Base types and verify selection can cross a `SelectableText` and component-path `TextView` in the same window.

- [ ] **Step 5: Run compatibility tests and commit**

Run: `cargo test -p gpui-component text:: --lib`

Run: `cargo test -p gpui-component window_selection --lib`

Run: `cargo check -p gpui-component`

Expected: all pass with one canonical Base implementation.

Commit:

```bash
git add crates/ui/src/text crates/ui/src/lib.rs crates/ui/src/theme crates/base/src/text/window_selection.rs
git commit -m "refactor(ui): re-export TextView from gpui-base"
```

### Task 5: Prove Base-only usability and complete repository migration

**Files:**
- Modify: `crates/base/examples/showcase/mod.rs`
- Modify: `crates/base/examples/showcase/components/text_selection.rs`
- Create: `crates/base/examples/showcase/components/text_view.rs`
- Modify: `crates/base/examples/showcase/components/mod.rs`
- Modify: documentation comments in `crates/base/src/selectable_text.rs` and `crates/base/src/text/mod.rs`
- Modify: `crates/ui/src/global_state.rs`

**Interfaces:**
- Consumes: all prior task APIs.
- Produces: executable Base-only reference examples and a repository with no stale UI implementation/import ownership.

- [ ] **Step 1: Add a Base showcase page for default Markdown and HTML**

Add a showcase section rendering `TextView::markdown(...).selectable(true)` and `TextView::html(...).selectable(true)` without passing `.style(...)`. Include headings, links, lists, a code block, and a table. Place `TextSelectionLayer` once at the showcase window root, not once per view.

- [ ] **Step 2: Add public documentation examples**

In `selectable_text.rs`, document the one-element form and shared-handle form. In `text/mod.rs`, document:

```rust
let view = gpui_base::text::markdown("# Hello\n\nSelectable **Markdown**")
    .selectable(true);
```

State that applications call `gpui_base::init(cx)` and render one `TextSelectionLayer` at the window/root layer for cross-component selection and copy.

- [ ] **Step 3: Audit ownership and stale dependencies**

Run:

```bash
rg -n "gpui_component|gpui-component" crates/base
rg -n "crate::text::(document|format|inline|node|selection|state)" crates/ui/src
find crates/ui/src/text -type f -maxdepth 3 -print
```

Expected: the first command returns no Base source/dependency imports; the second returns no UI implementation references; the third shows only the compatibility facade and any focused UI-only integration test/adapter.

- [ ] **Step 4: Run formatting and focused verification**

Run: `cargo fmt --all -- --check`

Run: `cargo test -p gpui-base`

Run: `cargo test -p gpui-component text:: --lib`

Run: `cargo check -p gpui-base --all-features`

Run: `cargo check -p gpui-component --all-features`

Expected: all commands pass.

- [ ] **Step 5: Run workspace regression checks**

Run: `cargo check --workspace --all-targets`

Run any repository-prescribed validation command discovered in `AGENTS.md`, `Makefile`, or CI configuration that covers Rust formatting/lints for these crates.

Expected: no downstream import, feature-forwarding, WASM-stub, example, or story compilation regressions.

- [ ] **Step 6: Commit the examples and cleanup**

```bash
git add crates/base crates/ui Cargo.toml Cargo.lock
git commit -m "docs(base): demonstrate selectable rich text"
```

## Completion audit

Before declaring the objective complete, collect direct evidence for every requirement:

- `rg` proves the canonical TextView implementation exists only in Base.
- Base-only tests instantiate Markdown and HTML with `TextViewStyle::default()` and draw them.
- Base tests perform pointer selection and clipboard copying for `SelectableText`.
- A cross-registration test selects through `SelectableText` and `TextView`.
- Component compatibility tests use the old public paths and builders.
- `rg` proves Base has no `LanguageRegistry`, `SyntaxHighlighter`, tree-sitter feature, or component highlighter import; the default code-block styled-run test is empty.
- Full focused tests and workspace checks pass from a clean diff.

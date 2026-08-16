# Text Selection Showcase Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the text-selection status visible below an independently scrolling document and give the embedded gpui-base example enough vertical space for drag-to-scroll selection.

**Architecture:** Render one fixed-height column with a flexing scroll viewport and a non-scrolling footer sibling. Continue driving only the existing document `ScrollHandle`; increase the base iframe height in the website theme so neither region is compressed.

**Tech Stack:** Rust, GPUI, gpui-base showcase, Vue/VitePress CSS, Cargo, Bun.

## Global Constraints

- The selectable document is the only scrollable region.
- The status footer is never a text-selection participant.
- The document remains long enough to overflow its viewport.
- The outer example remains borderless.
- Native and WASM showcases use the same Rust component.

---

### Task 1: Separate the document viewport and status footer

**Files:**
- Modify: `crates/base/examples/showcase/components/text_selection.rs`
- Modify: `website/.vitepress/theme/style.css`

**Interfaces:**
- Consumes: `BaseShowcase::text_selection`, `ScrollHandle`, `TextSelection::clear`.
- Produces: a fixed-height outer column whose first child is the scrolling document and whose second child is the persistent status footer.

- [x] **Step 1: Add a failing layout regression test**

Add stable element IDs to the outer layout, document viewport, and footer. Render the real showcase in a GPUI test and assert that scrolling changes the document viewport offset while the footer bounds remain unchanged.

- [x] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test -p gpui-base --example base_components text_selection_footer_stays_fixed_when_document_scrolls
```

Expected: FAIL because the footer currently belongs to the scrolling element and moves with its content.

- [x] **Step 3: Implement the two-region layout**

Change `BaseShowcase::text_selection` to return an outer `flex_col` container. Move the title and three paragraphs into a `flex_1`, `min_h_0`, `overflow_y_scroll` document child that owns `track_scroll`. Move the status, selected-text preview, and clear button into a non-shrinking footer sibling. Preserve the existing selection handles and document order.

- [x] **Step 4: Increase the embedded base-example height**

Raise `.component-example--base iframe` from `420px` to `600px`, and its small-screen value from `380px` to `520px`. Do not change component-gallery iframe sizing or zoomed-window behavior.

- [x] **Step 5: Verify GREEN and integration gates**

Run:

```bash
cargo test -p gpui-base --example base_components text_selection_footer_stays_fixed_when_document_scrolls
cargo test -p gpui-base --example base_components
cargo check -p gpui-base --example base_components
RUSTC_BOOTSTRAP=1 cargo check --manifest-path crates/base/examples/wasm/Cargo.toml --target wasm32-unknown-unknown
cd website && bun run build
cargo fmt --all -- --check
git diff --check
```

Expected: all commands exit successfully; the focused test proves the footer does not move with the document scroll offset.

- [x] **Step 6: Commit**

```bash
git add crates/base/examples/showcase/components/text_selection.rs website/.vitepress/theme/style.css docs/superpowers/plans/2026-08-16-text-selection-showcase-layout.md
git commit -m "fix(text-selection): stabilize showcase scrolling"
```

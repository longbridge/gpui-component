# Command Composition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add custom filtering, optional search, header/footer slots, and independently sized Command rows.

**Architecture:** Keep `CommandState` as the interaction/data entity and `Command` as its presentation element. Store construction-time search policy and filter behavior in `CommandState`, store header/footer render callbacks in `CommandOptions`, and retain `v_virtual_list` with independently measured per-row sizes.

**Tech Stack:** Rust, GPUI, gpui-component, `v_virtual_list`, cargo test.

**Spec:** `docs/superpowers/specs/2026-08-18-command-composition-design.md`

## Global Constraints

- Preserve the public `CommandState` plus `Command` split.
- `searchable` defaults to `true`; false hides the input and disables filtering.
- The custom filter signature is `Fn(&CommandItem, &str) -> bool + 'static`.
- Header and footer remain outside the scrolling list and impose no specialized content model.
- Do not add `CommandDialog`, `WindowExt::open_command_dialog`, `CommandFooter`, or shadcn-style child primitives.
- Do not optimize lowercase/string allocation as part of this work.

---

### Task 1: Search policy and custom filter

**Files:**
- Modify: `crates/ui/src/command/state.rs`
- Modify: `crates/ui/src/command/item.rs`

**Interfaces:**
- Produces: `CommandState::searchable(self, bool) -> Self`
- Produces: `CommandState::filter<F>(self, F) -> Self where F: Fn(&CommandItem, &str) -> bool + 'static`
- Preserves: `CommandItem::matches(&self, query: &str) -> bool` as the default filter

- [ ] **Step 1: Write failing search-policy tests**

Add GPUI tests beside the existing Command tests:

```rust
#[gpui::test]
fn custom_filter_controls_visible_items(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let cx = cx.add_empty_window();
    cx.update(|window, cx| {
        let state = cx.new(|cx| {
            CommandState::new(window, cx)
                .filter(|item, query| item.value().starts_with(query))
                .item(CommandItem::new("alpha"))
                .item(CommandItem::new("beta-alpha"))
        });
        state.update(cx, |state, cx| {
            state.set_query("alpha", window, cx);
            assert_eq!(state.matched_count(), 1);
            assert_eq!(state.selected_value(), Some("alpha".into()));
        });
    });
}

#[gpui::test]
fn non_searchable_command_keeps_every_item(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let cx = cx.add_empty_window();
    cx.update(|window, cx| {
        let state = cx.new(|cx| {
            CommandState::new(window, cx)
                .searchable(false)
                .item(CommandItem::new("alpha"))
                .item(CommandItem::new("beta"))
        });
        state.update(cx, |state, cx| {
            state.set_query("missing", window, cx);
            assert_eq!(state.matched_count(), 2);
        });
    });
}
```

- [ ] **Step 2: Run tests and verify the missing APIs fail compilation**

Run: `cargo test -p gpui-component command::state::tests --lib`

Expected: compilation fails because `filter` and `searchable` do not exist.

- [ ] **Step 3: Implement search policy in `CommandState`**

Add:

```rust
type CommandFilter = dyn Fn(&CommandItem, &str) -> bool;

pub struct CommandState {
    // existing fields
    searchable: bool,
    filter: Option<Rc<CommandFilter>>,
}

pub fn searchable(mut self, searchable: bool) -> Self {
    self.searchable = searchable;
    self.needs_update = true;
    self
}

pub fn filter<F>(mut self, filter: F) -> Self
where
    F: Fn(&CommandItem, &str) -> bool + 'static,
{
    self.filter = Some(Rc::new(filter));
    self.needs_update = true;
    self
}

fn item_matches(&self, item: &CommandItem, query: &str) -> bool {
    if !self.searchable || query.is_empty() {
        true
    } else if let Some(filter) = &self.filter {
        filter(item, query)
    } else {
        item.matches(query)
    }
}
```

Initialize `searchable: true` and `filter: None`, and route both ungrouped and grouped filtering through `item_matches`.

- [ ] **Step 4: Run focused and existing matching tests**

Run: `cargo test -p gpui-component command:: --lib`

Expected: all Command tests pass.

- [ ] **Step 5: Commit the search API**

```bash
git add crates/ui/src/command/state.rs crates/ui/src/command/item.rs
git commit -m "command: Add custom filtering and search visibility"
```

### Task 2: Hide the search input and preserve keyboard focus

**Files:**
- Modify: `crates/ui/src/command/state.rs`

**Interfaces:**
- Consumes: `CommandState::searchable(bool)` from Task 1
- Preserves: `Focusable for CommandState`, `CommandState::focus`

- [ ] **Step 1: Write a failing render/focus test**

Create a test harness rendering a `CommandState::new(...).searchable(false)` and assert after drawing that the Command frame owns focus and Down/Confirm actions can still advance and confirm a row. Use an event subscription to capture `CommandEvent::Confirm("beta")` after selecting down once.

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p gpui-component command::state::tests::non_searchable_command_uses_frame_focus --lib`

Expected: failure because rendering and `Focusable` still target the input.

- [ ] **Step 3: Implement conditional input rendering and focus**

In `Render for CommandState`, wrap the input child with `when(self.searchable, ...)`. Change `Focusable` and `focus` to return/focus `query_input` when searchable and the state's own `focus_handle` otherwise. Keep `.track_focus(&self.focus_handle)` and the Command key context on the outer frame.

Do not emit `CommandEvent::Query` for the hidden input. `set_query` may still update the stored input value programmatically, but `update_matches` must retain all entries while search is disabled.

- [ ] **Step 4: Run the focused test and all Command tests**

Run: `cargo test -p gpui-component command:: --lib`

Expected: all Command tests pass.

- [ ] **Step 5: Commit optional search rendering**

```bash
git add crates/ui/src/command/state.rs
git commit -m "command: Support palettes without a search input"
```

### Task 3: Provide independent row sizes to `v_virtual_list`

**Files:**
- Modify: `crates/ui/src/command/state.rs`

**Interfaces:**
- Consumes: flattened `Vec<CommandRow>` and each `MatchedItem::row_ix`
- Produces: internal `row_sizes` with one measured size for each flattened row
- Preserves: `VirtualListScrollHandle` and its existing keyboard navigation

- [ ] **Step 1: Replace the old uniform-height test with a failing variable-height test**

Render two custom items with intrinsic heights `px(32.)` and `px(72.)`. Draw the harness and assert the sizes supplied to `v_virtual_list` retain both final row heights independently.

- [ ] **Step 2: Run the variable-height test and verify failure**

Run: `cargo test -p gpui-component command::state::tests::custom_rows_keep_independent_heights --lib`

Expected: failure because the current virtual list assigns both rows the first row's height.

- [ ] **Step 3: Measure each flattened row**

Retain `VirtualListScrollHandle`, `row_sizes`, and `needs_measure`. Replace the
single sampled `RowHeights` with a measurement pass over flattened rows. Keep
fixed separator sizing, and use each heading or item row's root layout size for
its corresponding `row_sizes` entry.

- [ ] **Step 4: Rebuild existing virtual-list sizes**

When a match rebuild invalidates sizing, replace `row_sizes` with the result of
the per-row measurement pass before rendering. Keep the existing
`v_virtual_list`, `VirtualListScrollHandle`, `Scrollbar::vertical`, and
`scroll_handle.scroll_to_item(row_ix, ScrollStrategy::Top)` paths unchanged.

- [ ] **Step 5: Run variable-height, scrolling, and all Command tests**

Run: `cargo test -p gpui-component command:: --lib`

Expected: independent row-size and keyboard scrolling tests pass with all existing tests.

- [ ] **Step 6: Commit independent row sizing**

```bash
git add crates/ui/src/command/state.rs
git commit -m "command: Support variable-height rows"
```

### Task 4: Add header and footer render slots

**Files:**
- Modify: `crates/ui/src/command/command.rs`
- Modify: `crates/ui/src/command/state.rs`

**Interfaces:**
- Produces: `Command::header<F, E>(self, F) -> Self`
- Produces: `Command::footer<F, E>(self, F) -> Self`
- Callback contract: `F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static`, `E: IntoElement`

- [ ] **Step 1: Write failing header/footer tests**

Add a render test whose header and footer callbacks each increment a separate `Rc<Cell<usize>>` and read `state.matched_count()`. Draw once and assert both callbacks ran and both observed the literal expected count.

- [ ] **Step 2: Run the test and verify missing builder failure**

Run: `cargo test -p gpui-component command::state::tests::header_and_footer_render_with_current_state --lib`

Expected: compilation fails because `header` and `footer` do not exist.

- [ ] **Step 3: Store type-erased render callbacks**

Add to `CommandOptions`:

```rust
type CommandSlot = dyn Fn(&CommandState, &mut Window, &mut App) -> AnyElement;

header: Option<Rc<CommandSlot>>,
footer: Option<Rc<CommandSlot>>,
```

Implement each builder by wrapping its `IntoElement` return with `into_any_element`. Keep these callbacks presentation-only; do not move them into the public state API.

- [ ] **Step 4: Render slots outside the scroll container**

In `Render for CommandState`, invoke header before the conditional input and footer after the list container. Do not add padding, borders, status models, or keybinding-specific types; callers own all slot presentation.

- [ ] **Step 5: Run focused and full Command tests**

Run: `cargo test -p gpui-component command:: --lib`

Expected: all Command tests pass.

- [ ] **Step 6: Commit composition slots**

```bash
git add crates/ui/src/command/command.rs crates/ui/src/command/state.rs
git commit -m "command: Add header and footer slots"
```

### Task 5: Story examples and documentation

**Files:**
- Modify: `crates/story/src/stories/command_story.rs`
- Modify: `website/docs/components/command.md`
- Modify: `website/zh-CN/docs/components/command.md`

**Interfaces:**
- Consumes: `CommandState::searchable`, `CommandState::filter`, `Command::header`, and `Command::footer`

- [ ] **Step 1: Extend the Command story**

Add focused demonstrations:

- A no-search quick-actions palette built with `.searchable(false)`.
- A custom filter that matches stock symbols before names.
- A dialog Command with a header showing match count and a footer showing `↑↓`, Enter, and Escape hints.
- Two custom rows with visibly different heights to demonstrate independent
  `v_virtual_list` row sizes.

Reuse existing story helpers and theme tokens. Do not reintroduce a Command-specific dialog API.

- [ ] **Step 2: Update English documentation**

Document exact signatures, defaults, focus behavior, default versus custom matching, slot placement, and variable-height constraints. Explain that Command measures each flattened row when rebuilding its `v_virtual_list` sizes, so custom rows may have independent intrinsic heights.

- [ ] **Step 3: Mirror the changes in Chinese documentation**

Keep examples and API tables structurally identical to the English page. Translate explanations without changing signatures or defaults.

- [ ] **Step 4: Run formatting and full relevant verification**

Run:

```bash
cargo fmt -- crates/ui/src/command/command.rs crates/ui/src/command/state.rs crates/story/src/stories/command_story.rs
cargo test -p gpui-component command:: --lib
cargo test -p gpui-component-story --lib
cargo check -p gpui-component-story
git diff --check
```

Expected: every command exits 0; Command and story tests report zero failures; diff check prints nothing.

- [ ] **Step 5: Commit examples and docs**

```bash
git add crates/story/src/stories/command_story.rs website/docs/components/command.md website/zh-CN/docs/components/command.md
git commit -m "docs: Show Command composition options"
```

### Task 6: Final compatibility review

**Files:**
- Review: `crates/ui/src/command/command.rs`
- Review: `crates/ui/src/command/item.rs`
- Review: `crates/ui/src/command/state.rs`
- Review: `crates/story/src/stories/command_story.rs`
- Review: `website/docs/components/command.md`
- Review: `website/zh-CN/docs/components/command.md`

**Interfaces:**
- Verifies all interfaces produced by Tasks 1–5

- [ ] **Step 1: Inspect the complete branch diff**

Run: `git diff origin/main...HEAD -- crates/ui/src/command crates/story/src/stories/command_story.rs website/docs/components/command.md website/zh-CN/docs/components/command.md`

Check that existing `CommandState::new`, item/group/separator builders, events, entry replacement, and `Command` style builders remain source compatible.

- [ ] **Step 2: Run final verification from the completed tree**

Run:

```bash
cargo test -p gpui-component command:: --lib
cargo test -p gpui-component-story --lib
cargo check -p gpui-component-story
git diff --check
```

Expected: all commands exit 0 with no failed tests or diff errors.

- [ ] **Step 3: Record the final state**

Run `git status -sb` and `git log -6 --oneline`. Confirm only the intended branch commits exist and the working tree is clean before publishing.

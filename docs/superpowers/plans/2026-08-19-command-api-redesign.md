# Command API Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Command structure and behavior from `CommandState` to a simple `Command` builder API while retaining lazy virtual-list rendering, variable-height rows, Action-derived keybindings, and direct callbacks.

**Architecture:** `CommandState` remains a non-generic interaction entity. `Command` installs a private type-erased render model containing entries, filtering, callbacks, and visual options; the state flattens that model into indexed rows and asks each `CommandItem` lazy child factory to render during measurement or visible-range rendering. Public callers use `Command::item/group/separator` and `Command::on_*`, without a delegate or event subscription.

**Tech Stack:** Rust, GPUI, gpui-component, `v_virtual_list`, `gpui::Action`, `Kbd`, cargo test.

**Spec:** `docs/superpowers/specs/2026-08-19-command-api-redesign.md`

## Global Constraints

- Keep `CommandState` public, non-generic, and limited to interaction state and imperative interaction methods.
- Put `item`, `group`, `separator`, `searchable`, and `filter` builders on `Command`.
- Remove `CommandState::item`, `group`, `separator`, `searchable`, `filter`, and `set_entries`.
- Keep `v_virtual_list`, `VirtualListScrollHandle`, independent row heights, and the existing width/rem/typography measurement invalidation.
- `CommandItem::child` accepts a repeatable lazy factory; it must not store a one-shot `AnyElement`.
- Replace string shortcuts and item selection closures with `gpui::Action`; resolve displayed bindings through `Kbd`.
- Replace `CommandEvent` subscriptions with `Command::on_query`, `on_select`, `on_confirm`, and `on_cancel`.
- Confirm dispatches the item Action before invoking the palette-level `on_confirm` callback.
- Cancel invokes `on_cancel` and then propagates; Dialog owners must not close again from `on_cancel`.
- English and Chinese documentation must remain structurally aligned.

---

### Task 1: Move the entry model and lazy item rendering to `Command`

**Files:**
- Modify: `crates/ui/src/command/item.rs`
- Modify: `crates/ui/src/command/command.rs`
- Modify: `crates/ui/src/command/state.rs`
- Modify: `crates/ui/src/command/mod.rs`

**Interfaces:**
- Produces: `Command::item(self, CommandItem) -> Self`
- Produces: `Command::group(self, CommandGroup) -> Self`
- Produces: `Command::separator(self) -> Self`
- Produces: `Command::items(self, impl IntoIterator<Item = CommandItem>) -> Self`
- Produces: `Command::searchable(self, bool) -> Self`
- Produces: `Command::filter<F>(self, F) -> Self where F: Fn(&CommandItem, &str) -> bool + 'static`
- Produces: `CommandItem::child<F, E>(self, F) -> Self where F: Fn(&mut Window, &mut App) -> E + 'static, E: IntoElement`
- Removes: corresponding construction methods from `CommandState`, `CommandState::set_entries`, and `CommandItem::element`
- Preserves: `CommandState::new`, query/focus/selection/loading methods, `CommandGroup`, and `CommandEntry`

- [ ] **Step 1: Write compile-time and rendering regressions for the new construction API**

Replace state-builder setup in the Command tests with an owner harness that renders:

```rust
Command::new(&state)
    .searchable(false)
    .item(CommandItem::new("alpha"))
    .group(CommandGroup::new("Settings").item(CommandItem::new("beta")))
    .separator()
    .item(
        CommandItem::new("custom")
            .child(|_, _| div().h(px(72.)).child("Custom")),
    )
```

Assert after drawing that all three values are matched, the group heading and separator are represented in flattened rows, and the custom row retains its independent measured height.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test -p gpui-component command::state::tests::command_owns_entries_and_lazy_item_content --lib`

Expected: compilation fails because the builders are still on `CommandState` and `CommandItem::child` does not exist.

- [ ] **Step 3: Add a private render model owned by the state after installation**

Add private structures equivalent to:

```rust
pub(crate) type CommandFilter = dyn Fn(&CommandItem, &str) -> bool;
pub(crate) type CommandItemContent = dyn Fn(&mut Window, &mut App) -> AnyElement;

pub(crate) struct CommandModel {
    pub(crate) entries: Vec<CommandEntry>,
    pub(crate) searchable: bool,
    pub(crate) filter: Option<Rc<CommandFilter>>,
}
```

Store `CommandModel` privately in `CommandState`. `Command` owns the entries
while being built and moves them into the model during `RenderOnce`.
`CommandState::install_model` preserves selection by value if that value
remains enabled, otherwise selects the first enabled match, then rebuilds
matches and invalidates row measurement. Owner-view renders reinstall the
model; CommandState's own redraws reuse it.

- [ ] **Step 4: Implement Command entry builders and lazy child content**

Add `entries: Vec<CommandEntry>`, `searchable`, and `filter` to the private Command options/model construction. Implement `Command::item/items/group/separator/searchable/filter`. Replace `CommandItem::element` with:

```rust
pub fn child<F, E>(mut self, builder: F) -> Self
where
    F: Fn(&mut Window, &mut App) -> E + 'static,
    E: IntoElement,
{
    self.content = Some(Rc::new(move |window, cx| {
        builder(window, cx).into_any_element()
    }));
    self
}
```

Use the same content factory in detached row measurement and visible-range rendering. Document that it may run multiple times and must be side-effect-free.

- [ ] **Step 5: Remove state construction APIs and migrate all Command unit-test fixtures**

Delete state entry/search/filter builders and `set_entries`. Convert every test harness to own `Vec<CommandEntry>` or construct entries on `Command` during render. Keep tests for custom filtering, hidden search, disabled selection, grouped scrolling, typography invalidation, outer padding, slots, and variable heights behaviorally unchanged.

- [ ] **Step 6: Run Command tests and commit**

Run:

```bash
cargo test -p gpui-component command:: --lib
cargo fmt --all -- --check
git diff --check
```

Expected: all Command tests pass and checks exit zero.

Commit:

```bash
git add crates/ui/src/command
git commit -m "command: move entries to Command"
```

### Task 2: Replace shortcut strings, item handlers, and events with Actions and callbacks

**Files:**
- Modify: `crates/ui/src/command/item.rs`
- Modify: `crates/ui/src/command/command.rs`
- Modify: `crates/ui/src/command/state.rs`
- Modify: `crates/ui/src/command/mod.rs`

**Interfaces:**
- Produces: `CommandItem::action(self, Box<dyn gpui::Action>) -> Self`
- Produces: `Command::on_query`, `on_select`, `on_confirm`, `on_cancel`
- Removes: `CommandItem::shortcut`, `CommandItem::on_select`, `CommandEvent`, and `EventEmitter<CommandEvent>`
- Consumes: Task 1's installed `CommandModel` and lazy item rendering

- [ ] **Step 1: Write failing Action and callback tests**

Define a test Action and keybinding, then render:

```rust
Command::new(&state)
    .item(CommandItem::new("open").action(Box::new(OpenTestItem)))
    .on_query(record_query)
    .on_select(record_selection)
    .on_confirm(record_confirmation)
    .on_cancel(record_cancel)
```

Assert that the default row renders the resolved binding, Enter and click dispatch `OpenTestItem` before recording `on_confirm("open")`, highlight movement records `on_select`, query changes record `on_query`, and Escape records `on_cancel` before propagation.

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test -p gpui-component command::state::tests::command_actions_and_callbacks_follow_defined_order --lib`

Expected: compilation fails because Action and callback builders do not exist.

- [ ] **Step 3: Store Actions and callback slots**

Replace item shortcut/handler fields with `Option<Box<dyn Action>>`. Add private callback aliases to the installed model/options:

```rust
type OnQuery = dyn Fn(&str, &mut Window, &mut App);
type OnValue = dyn Fn(&SharedString, &mut Window, &mut App);
type OnCancel = dyn Fn(&mut Window, &mut App);
```

Use `Rc` for callbacks so the state can retain them across its own redraws.

- [ ] **Step 4: Resolve Kbd from the real Action and dispatch on confirmation**

In the default row, resolve the first binding with `Kbd::binding_for_action_in` using the Command focus handle, falling back to `Kbd::binding_for_action(action, None, window)`. Do not reserve a trailing slot when no binding exists. On click or Enter, clone and dispatch the Action, then invoke `on_confirm` with the item value.

- [ ] **Step 5: Route query, selection, and cancel directly to callbacks**

Invoke `on_query` only for actual searchable query changes. Invoke `on_select` only when highlight value changes. Invoke `on_cancel` before `cx.propagate()`. Remove every `cx.emit(CommandEvent::...)`, the event enum, and EventEmitter implementation.

- [ ] **Step 6: Run Command tests and commit**

Run:

```bash
cargo test -p gpui-component command:: --lib
cargo fmt --all -- --check
git diff --check
```

Commit:

```bash
git add crates/ui/src/command
git commit -m "command: use Actions and direct callbacks"
```

### Task 3: Migrate the Command Story and asynchronous stock search

**Files:**
- Modify: `crates/story/src/stories/command_story.rs`

**Interfaces:**
- Consumes: `Command::item/group/separator/searchable/filter/on_*` and `CommandItem::action/child`
- Removes: all CommandState entry construction and Command event subscriptions in this story
- Preserves: inline, dialog, quick-actions, scrollable, variable-height, and stock-search demonstrations

- [ ] **Step 1: Write or adapt Story behavior tests before migration**

Add focused tests that verify stock results live in `CommandStory`, a query callback replaces those results, confirmation clears/closes only through `on_confirm`, and Escape relies on Dialog dismissal rather than an explicit cancel close.

- [ ] **Step 2: Run Story tests and capture the expected compile failure after switching one fixture**

Run: `cargo test -p gpui-component-story command_story --lib`

Expected during the first migrated fixture: compilation fails until all removed CommandEvent/state-builder usages in the story are converted.

- [ ] **Step 3: Move static entries to Command builders**

Keep only interaction entities in `CommandStory`. Convert helper functions to return `Vec<CommandEntry>` or iterators, and apply them with `Command::items`/`group` during render. Replace `.element` with lazy `.child`.

- [ ] **Step 4: Move asynchronous stock data into the owner view**

Add `stock_entries: Vec<CommandEntry>` to `CommandStory`. `on_query` starts/cancels the existing task and updates that field through the owner entity, then calls `cx.notify()`. Re-render the stock Command with the latest entries; do not mutate entries through `CommandState`.

- [ ] **Step 5: Replace subscriptions with local callbacks and Actions**

Remove Command subscriptions from `_subscriptions`. Use `on_select`, `on_confirm`, and `on_cancel` on each rendered Command. Define Story-only Actions for rows that demonstrate real keybindings; let `Kbd` render those bindings instead of string shortcuts. Preserve Dialog's sole ownership of Escape dismissal.

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo test -p gpui-component-story --lib
cargo check -p gpui-component-story
cargo fmt --all -- --check
git diff --check
```

Commit:

```bash
git add crates/story/src/stories/command_story.rs
git commit -m "story: migrate Command composition API"
```

### Task 4: Migrate Story component and theme palettes

**Files:**
- Modify: `crates/story/src/lib.rs`
- Modify: `crates/story/src/gallery.rs`
- Modify: `crates/story/src/themes.rs`

**Interfaces:**
- Consumes: revised Command builders and callbacks
- Preserves: Ctrl-Shift-P/Go-to palette, theme preview rollback, repeated-open ownership guards, and component navigation
- Removes: `CommandEvent` subscriptions and state `set_entries` calls

- [ ] **Step 1: Preserve existing real-window regression coverage**

Keep the current tests for component navigation, non-empty Escape, stacked dialogs, repeated component opens, theme rollback, theme confirmation, and repeated theme opens. Convert their setup only as required by the new API; do not weaken assertions.

- [ ] **Step 2: Store entry collections with StoryRoot**

Add owner fields for component and theme `Vec<CommandEntry>`. Refresh them before opening their palettes, then notify and render them through `Command::items`. Interaction entities retain query, focus, selection, and scroll only.

- [ ] **Step 3: Replace event subscriptions with callbacks**

Remove `cx.subscribe_in` calls for both Command states. Bind `on_select` to theme preview, `on_confirm` to theme commit/component navigation, and `on_cancel` only to non-dismissal cleanup that cannot double-pop. Keep Dialog `on_close` rollback/finalization and both repeated-open guards.

- [ ] **Step 4: Convert entry actions and custom rows**

Use `CommandItem::action` for executable commands where an Action exists. Component/theme values without a dispatchable Action continue through palette-level callbacks. Replace any custom `.element` use with `.child`.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo test -p gpui-component-story --lib
cargo check -p gpui-component-story
cargo fmt --all -- --check
git diff --check
```

Commit:

```bash
git add crates/story/src/lib.rs crates/story/src/gallery.rs crates/story/src/themes.rs
git commit -m "story: migrate Command palettes"
```

### Task 5: Update documentation and run final compatibility verification

**Files:**
- Modify: `website/docs/components/command.md`
- Modify: `website/zh-CN/docs/components/command.md`
- Review: `crates/ui/src/command/*.rs`
- Review: `crates/story/src/**/*.rs`

**Interfaces:**
- Documents all revised public interfaces from Tasks 1–4
- Verifies removed APIs have no remaining call sites

- [ ] **Step 1: Rewrite English examples and API tables**

Show entries built directly on `Command`, lazy item children, Action-derived Kbd hints, direct callbacks, dynamic owner-held entries, groups, filtering, optional search, header/footer, and variable heights. Remove all CommandEvent subscriptions, state entry builders, set_entries, manual shortcut strings, and item on_select examples.

- [ ] **Step 2: Mirror the English structure in Chinese**

Keep headings, examples, method tables, signatures, defaults, callback ordering, and Dialog cancellation guidance identical while translating prose.

- [ ] **Step 3: Search for removed API usage**

Run:

```bash
rg -n "CommandEvent|\.set_entries\(|CommandState::new[^;]*\.(item|group|separator|searchable|filter)|\.shortcut\(|\.element\(" crates/story website crates/ui/src/command
```

Expected: no Command call site uses a removed API; unrelated components may still legitimately define similarly named methods and must be inspected rather than mechanically changed.

- [ ] **Step 4: Run fresh full verification**

Run:

```bash
cargo test -p gpui-component command:: --lib
cargo test -p gpui-component-story --lib
cargo check -p gpui-component-story
cargo fmt --all -- --check
git diff --check
```

Expected: all commands exit zero on the final tree.

- [ ] **Step 5: Review the complete feature diff and commit docs**

Inspect `git diff origin/main...HEAD` for public API consistency, callback ownership, Action dispatch ordering, virtual-list performance, and documentation parity.

Commit:

```bash
git add website/docs/components/command.md website/zh-CN/docs/components/command.md
git commit -m "docs: update Command composition API"
```

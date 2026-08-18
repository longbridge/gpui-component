# Story Command Palette Report

## Scope

- Added a Story Gallery component palette opened by `ctrl-shift-p`.
- Reused `CommandState`, `Command`, and the existing dialog APIs.
- Kept non-Gallery `StoryRoot` views unchanged by storing an optional downcast Gallery handle.
- Selecting a component clears the sidebar search, navigates to the original unfiltered story index, and closes the dialog. Escape closes the dialog. Every open resets the palette query.

## TDD evidence

Red:

```text
cargo test -p gpui-component-story gallery::tests --no-default-features
error[E0432]: unresolved imports `super::component_command`, `super::find_story_index`
```

Green:

```text
cargo test -p gpui-component-story gallery::tests --no-default-features
2 passed; 0 failed

cargo test -p gpui-component-story
6 passed; 0 failed

cargo check -p gpui-component-story
Finished `dev` profile

git diff --check
clean
```

The attempted full no-default-features test also builds examples and failed on pre-existing example assumptions about tree-sitter-only `Language` variants (`Html`, `Markdown`, `Rust`) and `LanguageConfig::new`. The focused no-default tests and the normal full Story test both pass.

## Files

- `crates/story/src/gallery.rs`
- `crates/story/src/lib.rs`

## Commit

- `c0fa1b29 story: Add component command palette`

## Concerns

- The shortcut is intentionally only `ctrl-shift-p`, including macOS, per the approved requirement.
- Component lookup is case-insensitive and currently performs small temporary name-vector allocations on confirmation; the Gallery contains only dozens of stories, so this is outside a meaningful hot path.

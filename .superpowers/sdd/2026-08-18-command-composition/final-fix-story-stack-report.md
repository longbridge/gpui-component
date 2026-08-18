# Story confirmation test stack-overflow fix

## Reproduction

The default command aborted while running only
`tests::confirm_navigates_and_clears_sidebar_search`:

```text
thread 'tests::confirm_navigates_and_clears_sidebar_search' (...) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

The same focused test passed with the process-local diagnostic setting
`RUST_MIN_STACK=33554432`; no repository-wide stack setting was added.

Temporary progress markers placed the original overflow during
`gallery.set_sidebar_query_for_test("button", ...)`, before the component
palette opened. The two neighboring palette tests passed with the default
stack because they do not mutate the Gallery sidebar input.

## Root cause

The acceptance test used the real showcase Gallery as its fixture. Updating
its sidebar input enters GPUI's update/render path for the full showcase. That
fixture materializes and renders heavyweight component stories, exhausting the
default test-thread stack. The palette confirmation handler was not the
trigger.

## Fix

`Gallery::test_view` is a `#[cfg(test)]` metadata-only Gallery containing
Button and Command StoryContainers. It preserves the real Gallery search
input, change subscription, command-entry lookup, selection logic, StoryRoot
subscription, and dialog close behavior while avoiding component-story
rendering. Only `confirm_navigates_and_clears_sidebar_search` opts into this
fixture; neighboring tests retain the full Gallery fixture.

## Verification

- `cargo test -p gpui-component-story --lib confirm_navigates_and_clears_sidebar_search -- --nocapture` passed with the default stack.
- Final integration verification with
  `cargo test -p gpui-component-story --lib` passed all 14 tests with the
  default stack and no `RUST_MIN_STACK` override.
- `cargo fmt --all -- --check` passed.
- `cargo check -p gpui-component-story` passed.
- `git diff --check` and `git diff --cached --check` passed.

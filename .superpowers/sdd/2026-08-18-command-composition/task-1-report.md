# Task 1 report: Search policy and custom filter

## Implementation

- Added `CommandState::searchable(bool) -> Self`, defaulting to searchable.
- Added `CommandState::filter(F) -> Self` with an `Rc`-owned predicate.
- Centralized matching in `item_matches`, preserving `CommandItem::matches` as the default filter.
- Applied the policy to both ungrouped and grouped command items. Non-searchable commands retain every item regardless of query.
- Added GPUI tests for custom filtering and non-searchable commands.

## RED evidence

After adding the two tests, `cargo test -p gpui-component command::state::tests --lib` failed to compile with the expected missing-method errors for `CommandState::filter` and `CommandState::searchable`.

## GREEN evidence

- `cargo test -p gpui-component command::state::tests --lib`: 10 passed.
- `cargo test -p gpui-component command:: --lib`: 12 passed.
- `cargo fmt --all -- --check`: passed.

## Files

- `crates/ui/src/command/state.rs`
- `.superpowers/sdd/2026-08-18-command-composition/task-1-report.md`

## Self-review

The default path still delegates to `CommandItem::matches`; custom predicates only run for non-empty queries when search is enabled. Matching uses the same helper for grouped and ungrouped entries, and builder calls mark the state for update. No rendering, list, header, or footer work was changed.

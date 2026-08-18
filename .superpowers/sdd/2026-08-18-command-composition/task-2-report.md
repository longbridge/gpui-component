# Task 2 report: hide the search input and preserve keyboard focus

## RED

Added `non_searchable_command_uses_frame_focus`, rendering a non-searchable
command through the existing `Command` harness, focusing it, dispatching Down
and Confirm, and subscribing for `CommandEvent::Confirm("beta")`.

Before the production change:

```text
cargo test -p gpui-component command::state::tests::non_searchable_command_uses_frame_focus --lib
...
test command::state::tests::non_searchable_command_uses_frame_focus ... FAILED
assertion failed: state.read(cx).focus_handle.is_focused(window)
```

## GREEN

Implemented the minimal behavior in `crates/ui/src/command/state.rs`:

- The search input wrapper is rendered only when `searchable` is true.
- `Focusable for CommandState` returns the query input handle when searchable,
  otherwise the command state's frame handle.
- `CommandState::focus` follows the same conditional behavior.
- The outer frame retains `track_focus`, the Command key context, and action
  handlers; matching already retains all entries when searching is disabled.

Focused test:

```text
test command::state::tests::non_searchable_command_uses_frame_focus ... ok
```

All Command tests:

```text
cargo test -p gpui-component command:: --lib
13 passed; 0 failed
```

## Self-review

`git diff --check` passed. The public `CommandState` / `Command` split is
preserved, and no list or header/footer code was changed. Hidden input cannot
generate user input events because it is not rendered; programmatic query
updates remain supported.

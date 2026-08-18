# Theme palette dialog lifecycle fix

## Scope

- Fixed theme preview cleanup when the theme Command palette is dismissed.
- Kept Dialog as the sole owner of Escape/Cancel dismissal.
- Preserved Confirm behavior: the selected theme is committed and exactly one dialog is closed.

## Root cause

`CommandState` and Dialog share the Cancel action. With an empty query,
`CommandState` emits `CommandEvent::Cancel` and then propagates the action to
Dialog. The Story subscriber also called `window.close_dialog`, so the
subscriber popped the theme palette and Dialog then popped the modal below it.

A non-empty query follows a different route: the first Escape clears the query
without emitting `CommandEvent::Cancel`; the eventual Dialog dismissal could
therefore bypass the subscriber cleanup. That left the preview theme applied
and `AppState::previewing_theme` set.

The component palette already uses the correct Cancel ownership: its subscriber
does nothing and the hosting Dialog performs the single dismissal.

## Fix

- `CommandEvent::Cancel` no longer closes the dialog or owns cleanup.
- The theme Dialog has an `on_close` callback that weakly updates `StoryRoot`,
  restores the original theme when the preview was not committed, and finishes
  the preview.
- `CommandEvent::Confirm` clears the rollback marker, finishes the preview, and
  calls `window.close_dialog` once.

The Confirm path deliberately retains explicit finalization. The current
imperative `WindowExt::close_dialog` API pops the Root dialog directly and does
not invoke `Dialog::on_close`; `CommandState` also consumes Confirm rather than
propagating it to Dialog. Expanding the Dialog API is outside this Story fix, so
Escape/Cancel cleanup uses `on_close`, while Confirm performs its one explicit
commit finalization and close.

## TDD evidence

The real window/Dialog regression tests cover:

- non-empty-query Escape clearing followed by dismissal, rollback, and preview
  finalization;
- empty-query Escape with a stacked background dialog, proving only the theme
  palette is popped;
- Confirm with a stacked background dialog, proving the selected theme is
  committed, preview mode ends, and exactly one dialog is popped.

Before the production change, the focused run failed as expected:

```text
tests::escape_with_non_empty_theme_query_restores_theme_and_finishes_preview ... FAILED
tests::empty_query_escape_closes_only_theme_palette_when_dialogs_are_stacked ... FAILED
```

The Confirm test was then tightened to assert the stacked one-pop route; with
subscriber and Dialog both closing, it failed because no background dialog
remained.

After the fix:

```text
cargo test -p gpui-component-story --lib --no-default-features theme -- --nocapture
7 passed; 0 failed
```

## Verification

```text
cargo test -p gpui-component-story --lib
14 passed; 0 failed

cargo check -p gpui-component-story
Finished `dev` profile

cargo fmt --all -- --check
passed

git diff --check && git diff --cached --check
passed
```

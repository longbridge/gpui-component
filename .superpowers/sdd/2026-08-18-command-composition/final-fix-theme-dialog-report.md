# Theme palette dialog lifecycle fix

## Scope

- Fixed theme preview cleanup when the theme Command palette is dismissed.
- Prevented repeated SelectTheme actions from stacking theme dialogs or
  replacing the original rollback owner.
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

A follow-up review found a second ownership path: invoking SelectTheme again
while the palette was open pushed another Dialog and replaced
`theme_before_preview` with the currently previewed theme. Closing the top
Dialog then restored that preview instead of the original theme, cleared the
preview flag, and left the first palette underneath without a rollback owner.

## Fix

- `CommandEvent::Cancel` no longer closes the dialog or owns cleanup.
- The theme Dialog has an `on_close` callback that weakly updates `StoryRoot`,
  restores the original theme when the preview was not committed, and finishes
  the preview.
- `CommandEvent::Confirm` clears the rollback marker, finishes the preview, and
  calls `window.close_dialog` once.
- SelectTheme now detects the existing rollback marker, refocuses the current
  palette, and returns without resetting its state or opening another Dialog.

The Confirm path deliberately retains explicit finalization. The current
imperative `WindowExt::close_dialog` API pops the Root dialog directly and does
not invoke `Dialog::on_close`; `CommandState` also consumes Confirm rather than
propagating it to Dialog. Expanding the Dialog API is outside this Story fix, so
Escape/Cancel cleanup uses `on_close`, while Confirm performs its one explicit
commit finalization and close.

Generic imperative dialog closure bypassing `Dialog::on_close` remains the
existing global Dialog contract. The theme palette has no production
imperative-close route except Confirm, which finalizes explicitly before its
single close. Changing the global Dialog API is therefore residual and outside
this Story-scoped correction.

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

The repeated-open follow-up was also written before its guard. Its red run
showed the rollback owner being overwritten by the previewed theme:

```text
tests::repeated_select_theme_keeps_one_dialog_and_the_original_rollback_owner ... FAILED
left: Some("Default Dark")
right: Some("Default Light")
```

The green test invokes SelectTheme twice through the real window, verifies one
Escape removes the only palette and restores the original theme, verifies the
ownership marker and preview flag are cleared, then opens and closes the palette
again successfully. The Confirm regression also asserts that its commit path
clears the same marker.

After the fix:

```text
cargo test -p gpui-component-story --lib --no-default-features theme -- --nocapture
7 passed; 0 failed
```

## Verification

```text
cargo test -p gpui-component-story --lib
15 passed; 0 failed

cargo check -p gpui-component-story
Finished `dev` profile

cargo fmt --all -- --check
passed

git diff --check && git diff --cached --check
passed
```

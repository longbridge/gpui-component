# Undo Manager Design Research

Research date: 2026-08-15

## Conclusion

The `gpui-base` input history should not keep using the rule “merge every edit that occurs within the same time window.” An undoable operation should first be represented as an explicit **editing transaction**, after which a separate policy decides whether compatible transactions may coalesce. Wall-clock time does not define a boundary; edit intent, spatial continuity, and explicit structural boundaries do.

The recommended model is:

1. Every text mutation first creates a transaction. Adjacent ordinary typing, Backspace, or Forward Delete transactions of the same kind may coalesce.
2. Coalescing does not use wall-clock time. A pause neither ends nor extends a transaction.
3. Cursor or selection movement creates no text transaction, but it breaks coalescing. A newline is always an independent structural transaction.
4. One IME composition produces one undo unit. Intermediate marked-text updates do not create separate history entries.
5. Any new recorded edit after undo immediately clears the redo branch.
6. Formatting, auto-indentation, or other commands that produce several changes must explicitly wrap those changes in one transaction.
7. Both retained transaction count and changes within one coalesced transaction are bounded, preventing either stack layer from growing without limit.

Platform text-input callbacks do not map one-to-one to undo transactions. Several adjacent callbacks may coalesce, an input method may commit several characters in one callback, and one composition may span several callbacks while remaining one explicit transaction.

## Problems in the Previous Implementation

Before this change, [`InputBaseState`](../../crates/base/src/input/base/state.rs) initialized history with `History::new().group_interval(Duration::from_secs(1))`. The generic [`History`](../../crates/base/src/history.rs) implementation only used elapsed time in `inc_version`; unrelated typing, deletion, paste, or replacement operations received the same version whenever they occurred within the configured interval.

Source inspection also identified two independent problems:

- `History::push` did not clear `redos`, so a new edit after undo did not create a new linear history branch.
- `start_grouping` and `end_grouping` only prevented version increments. They did not express edit intent such as typing, backward deletion, or paste, and could not validate spatial continuity.

Changing one second to 500 milliseconds therefore could not fix the underlying model.

## Primary-Source Comparison

### Browser Input Events Semantics

W3C Input Events Level 2 distinguishes intents including `insertText`, `insertCompositionText`, `insertFromPaste`, `insertReplacementText`, `insertParagraph` / `insertLineBreak`, `deleteContentBackward` / `deleteContentForward`, word- and line-based deletion, and `historyUndo` / `historyRedo`. Platform events therefore expose richer semantics than the time at which text happened to change. [Input Events Level 2: `inputType` table](https://www.w3.org/TR/input-events-2/#interface-InputEvent-Attributes)

The specification defines `historyUndo` as undoing the previous editing action and `historyRedo` as redoing the previous undone editing action, but it does not prescribe how a browser must divide keystrokes into actions. No universal timing threshold can be derived from the Web specification; the useful part is its intent taxonomy, not unspecified browser grouping behavior. [Input Events Level 2: `inputType` table](https://www.w3.org/TR/input-events-2/#interface-InputEvent-Attributes)

During IME input, composition events and `insertCompositionText` input events form a sequence with explicit start and end boundaries. The specification also notes that `beforeinput` events within a composition are not cancelable. History should not save every temporary candidate update as ordinary typing. [Input Events Level 2: event order during composition](https://www.w3.org/TR/input-events-2/#events-composition)

Paste has its own `insertFromPaste` intent and event sequence. It should be an independent undo unit rather than merge into nearby typing merely because the events are close in time. [Input Events Level 2: paste event order](https://www.w3.org/TR/input-events-2/#event-order-when-using-insertfrompaste)

### macOS AppKit Text System

`NSTextView` exposes `isCoalescingUndo` and provides `breakUndoCoalescing()` so subsequent typing begins a new undo grouping. The native macOS model is therefore “coalesce successive typing, then break on explicit events,” not fixed time slicing. [Apple `NSTextView`](https://developer.apple.com/documentation/appkit/nstextview); [Apple `breakUndoCoalescing()`](https://developer.apple.com/documentation/appkit/nstextview/breakundocoalescing())

A minimal native `NSTextView` probe confirmed that rapidly typing `abc` is undone in one operation, and a 2.5-second pause does not split it. Inserting after cursor movement creates a new undo item. AppKit also coalesces consecutive Return presses with typing by default and may include deletion immediately following typing in the same item. This design therefore adopts AppKit’s explicit coalescing lifecycle without copying all of its boundary choices. Editor newlines, deletion-direction changes, and structural commands use finer explicit boundaries.

The same probe compared a single-line `NSTextField` with a multiline `NSTextView`. They behave alike for continuous typing, pauses, keyboard movement, and mouse-driven insertion-point changes. Their editing sessions differ: Return commits the single-line field-editor session, and the next edit uses the committed value as its baseline, so Undo returns to that value. In a multiline view, Return is a text mutation and AppKit may coalesce it with surrounding typing. Accordingly, `gpui-base` breaks transaction coalescing on single-line Return, `submit_on_enter`, and blur, while an ordinary multiline Return is an independent structural transaction. It does not copy the `NSTextField` selection behavior that selects the old value when a new editing session begins.

### ProseMirror

ProseMirror defaults `newGroupDelay` to 500 milliseconds, but its documentation states that non-adjacent changes always start a new group. Its `applyTransaction` implementation checks both time and `isAdjacentTo`, so time is only the maximum interval for otherwise adjacent edits. [Official API](https://prosemirror.net/docs/ref/#history.history); [official `applyTransaction` source](https://github.com/ProseMirror/prosemirror-history/blob/445409bc99c88550c2312f5610829ecb25105a5f/src/history.ts#L260-L312)

It also provides `closeHistory(transaction)` to force a boundary and `addToHistory: false` for programmatic transactions that should not enter the undo stack. [Official API](https://prosemirror.net/docs/ref/#history.closeHistory); [official source](https://github.com/ProseMirror/prosemirror-history/blob/445409bc99c88550c2312f5610829ecb25105a5f/src/history.ts#L360-L394)

Transactions with the same composition ID receive special handling so ordinary time and adjacency checks do not split one composition. When a normal new edit enters the done branch, the undone branch is replaced with `Branch.empty`, clearing redo. [Official source](https://github.com/ProseMirror/prosemirror-history/blob/445409bc99c88550c2312f5610829ecb25105a5f/src/history.ts#L277-L287)

### Lexical

Lexical’s default delay is 300 milliseconds, but automatic merging also requires an identical change type, the same editor, and a change within the window. Its change classification distinguishes single-character insertion, Backspace, Forward Delete, composition, and other edits. Non-collapsed selections, incompatible nodes or positions, and complex mutations become `OTHER` and create a new history entry. [Official change-classification source](https://github.com/facebook/lexical/blob/3359e9c6f0cc48d95355b25f42dc5b6eaca4489b/packages/lexical-history/src/index.ts#L105-L205); [official merge and delay source](https://github.com/facebook/lexical/blob/3359e9c6f0cc48d95355b25f42dc5b6eaca4489b/packages/lexical-history/src/index.ts#L251-L375)

Lexical explicitly classifies paste and cut tags as `OTHER`, preventing clipboard operations from merging with typing on either side. It also provides `HISTORY_PUSH_TAG` and `HISTORY_MERGE_TAG` for explicit transaction control. [Official source](https://github.com/facebook/lexical/blob/3359e9c6f0cc48d95355b25f42dc5b6eaca4489b/packages/lexical-history/src/index.ts#L300-L345)

Intermediate composition states do not each create candidate history entries. At composition end, Lexical re-evaluates the pre-composition state, timestamp, and change type so the complete input process is handled as a unit. [Official source](https://github.com/facebook/lexical/blob/3359e9c6f0cc48d95355b25f42dc5b6eaca4489b/packages/lexical-history/src/index.ts#L263-L305)

Lexical clears `redoStack` when it pushes a new history entry. [Official source](https://github.com/facebook/lexical/blob/3359e9c6f0cc48d95355b25f42dc5b6eaca4489b/packages/lexical-history/src/index.ts#L518-L546)

### Slate

Slate’s default rules use no timing window. They merge only strictly continuous `insert_text` operations on the same path and strictly continuous `remove_text` operations in the Backspace direction. Other operations naturally start a new batch. `set_selection` is not itself saved to history, but after cursor movement the next text operation normally cannot satisfy spatial adjacency. [Official source](https://github.com/ianstormtaylor/slate/blob/ec793483ada7f7e21ebc82c2b3aa9ea674605ce3/packages/slate-history/src/with-history.ts#L68-L157)

Slate provides `withNewBatch`, `withoutMerging`, `withMerging`, and `withoutSaving`, demonstrating that explicit transaction boundaries are necessary history controls. [Official documentation](https://docs.slatejs.org/libraries/slate-history/history-editor)

After saving each new operation, Slate assigns `history.redos = []`. Undo and redo move batches inside `withoutSaving` so those operations are not recorded as new edits. [Official source](https://github.com/ianstormtaylor/slate/blob/ec793483ada7f7e21ebc82c2b3aa9ea674605ce3/packages/slate-history/src/with-history.ts#L20-L115)

### CodeMirror 6

CodeMirror defaults `newGroupDelay` to 500 milliseconds, but automatic merging is limited to `input.type*` and `delete*` user events and also requires adjacent change ranges. Paste, structural commands, and other user events are not automatically merged. [Official API](https://codemirror.net/docs/ref/#commands.history); [official source](https://github.com/codemirror/history/blob/3c41743067bc405faa0b9cbe0c81ef1e6f7cd627/src/history.ts#L302-L328)

Selection history is recorded separately. An existing selection event prevents an ordinary change from merging into an earlier change. `isolateHistory("before" | "after" | "full")` establishes an explicit boundary on either side. [Official selection and change-merging source](https://github.com/codemirror/history/blob/3c41743067bc405faa0b9cbe0c81ef1e6f7cd627/src/history.ts#L302-L342); [official explicit-boundary source](https://github.com/codemirror/history/blob/3c41743067bc405faa0b9cbe0c81ef1e6f7cd627/src/history.ts#L10-L16)

`input.type.compose` always merges into the previous event so one composition is not split. Adding a new change constructs `HistoryState(done, none, ...)`, clearing the undone/redo branch. [Official source](https://github.com/codemirror/history/blob/3c41743067bc405faa0b9cbe0c81ef1e6f7cd627/src/history.ts#L314-L328)

## Recommended Rules for `gpui-base`

“Previous item” means the latest transaction that remains eligible for coalescing. Except for operations marked “not recorded,” every real text edit stores the selection before and after the change so undo and redo can restore the insertion point or selection.

| Current operation | May coalesce with previous? | Required conditions | Notes |
|---|---:|---|---|
| Ordinary text input at a collapsed selection | Yes | Previous item is ordinary input; insertion positions are strictly adjacent; no explicit boundary occurred | Independent of typing speed, whitespace, punctuation, or language |
| Single-character Backspace | Yes | Previous item is Backspace; deletion ranges are strictly adjacent to the left | Separate from ordinary input and Forward Delete |
| Single-character Forward Delete | Yes | Previous item is Forward Delete; deletion occurs at the same logical position | Separate from ordinary input and Backspace |
| IME marked-text update | Not as an independent item | Keep the composition transaction open | Temporary candidates are not separate committed edits |
| IME commit | No; creates one item | Snapshot from before composition through final committed result | One composition, one undo operation; subsequent ordinary typing starts separately |
| Replace a non-empty selection | No | — | The selected range and inserted text form one atomic operation |
| Paste, cut, or drag-and-drop | No | — | Independent user intent; creates boundaries on both sides |
| Enter / newline | No | — | A structural intent in textarea/editor; keeps undo granularity stable and predictable |
| Single-line Return / `submit_on_enter` | Creates no text item, but breaks | — | Commits the current editing session; subsequent input starts another transaction |
| Blur / refocus | Creates no text item, but breaks | — | Input on opposite sides of blur cannot coalesce; existing history remains intact |
| Word- or line-based deletion | No | — | One command is one undo unit and does not merge with character deletion |
| Formatting, auto-indent, or code operation | No, or explicitly merged | Declared by the caller | Derived mutations from one user command should undo atomically |
| Programmatic replacement such as `replace_all` | No | — | If undoable, it is an independent item; ordinary `set_value` may explicitly reset history |
| Cursor or selection movement only | Creates no text item | — | Does not clear redo; subsequent input naturally starts a new transaction |
| Undo / redo | Not recorded as a new edit | — | Moves existing transactions between branches and restores their selections |
| New edit after undo | No | — | Clears the entire redo branch when the new edit is recorded |

Frameworks do not fully agree on whether newlines may merge with continuous typing. CodeMirror may merge some newlines based on user-event and adjacency rules, while paragraph splitting is a complex structural change in rich-text frameworks. Because `gpui-base` serves Input, Textarea, and Editor, treating explicit Enter as a boundary is simpler and more predictable. This is a product-design decision, not a Web-platform requirement.

## Recommended Data Model

The generic `History<I>` should not use wall-clock time to assign arbitrary items the same version. The Input layer owns grouping decisions because only it knows edit intent, selection state, composition state, and text ranges.

The private `UndoManager` stores:

- private Input undo and redo transaction stacks, leaving the public `History<T>` API and behavior unchanged;
- an `EditIntent` and one or more `Change` values for each transaction;
- `transaction_open`, indicating whether an explicit compound transaction is being collected;
- `pending_change`, aggregating the state from before a compound transaction through its current result;
- `coalescing_boundary`, indicating that the next item cannot merge with the previous transaction;
- `selection_before` and `selection_after` on every `Change`.

Recording a new change follows these steps:

1. If the change does not mutate text, do not push it or clear redo.
2. If an explicit transaction is open, aggregate the change into `pending_change`.
3. Otherwise, first create a transaction, then compare edit intent, modified ranges, and explicit boundaries. Coalesce compatible transactions; otherwise push a new one.
4. `break_transaction_coalescing` establishes a boundary without creating an empty undo item or clearing redo.
5. `commit_transaction` commits explicit compound operations such as IME composition. No time-based open group exists.
6. Stop coalescing when a transaction reaches the per-transaction change limit, and discard the oldest retained transaction when the stack reaches its outer limit.

Every path that writes a new undo transaction must clear redo. Selection-only movement breaks coalescing but must not clear redo because the document has not created a new history branch.

## Recommended Acceptance Cases

- Rapidly type, or pause while typing, a long single line. One undo removes the continuous input; elapsed time does not change the result.
- Type several long lines. Each Enter is independent from the text on either side, and undo never crosses a newline boundary.
- Continue typing after Return in a single-line input or a `submit_on_enter` textarea. The first undo removes only the post-submit input.
- Blur, refocus, and continue typing. The first undo does not cross the blur boundary.
- Type `ab`, move the cursor, then type `x`. The first undo removes only `x`.
- Type, paste immediately, then continue typing. The three segments undo independently.
- Consecutive Backspace operations may coalesce. Backspace, Forward Delete, and ordinary input never merge with one another.
- Replace a text selection by typing. The replacement is one undo unit and does not merge with typing on either side.
- Enter / newline remains independent from typing on both sides.
- One IME composition containing several marked-text updates requires one undo. Canceling composition leaves no ineffective history item.
- Undo and redo restore both text and selection. Typing after undo immediately invalidates redo.
- `replace_all`, mask rewrites, formatting, and other compound mutations undo atomically without range corruption.
- Selection movement that does not enter history preserves redo; a real new text edit clears it.

## Final Decision

Use a transaction-first model. Every edit first forms a transaction; compatible, spatially continuous ordinary typing or same-direction deletion may coalesce. Composition and other compound operations use an explicit lifecycle. Cursor movement and structural commands explicitly break coalescing, and wall-clock time never defines a boundary. This preserves the familiar feel of continuous macOS typing while giving newlines, clipboard operations, replacements, and command deletion stable boundaries.

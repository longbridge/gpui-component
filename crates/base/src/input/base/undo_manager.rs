use crate::input::change::Change;

const MAX_UNDO_TRANSACTIONS: usize = 1000;
const MAX_CHANGES_PER_TRANSACTION: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditIntent {
    Typing,
    Backspace,
    DeleteForward,
    Atomic,
}

#[derive(Debug)]
struct UndoTransaction {
    intent: EditIntent,
    changes: Vec<Change>,
    /// How many changes the most recently appended batch contributed. A batch
    /// is one logical edit with one change per cursor. Only a following batch
    /// of the same length can coalesce into this transaction.
    last_batch_len: usize,
}

/// A batch of changes being collected between `begin_transaction` and the
/// matching `commit_transaction`.
#[derive(Debug)]
struct PendingTransaction {
    intent: EditIntent,
    changes: Vec<Change>,
}

/// Coordinates undo and redo as explicit editing transactions.
///
/// Each edit first creates a transaction. Compatible adjacent transactions
/// may then coalesce until an explicit boundary is encountered. Callers that
/// perform one logical edit through several changes (IME composition, and any
/// multi-cursor edit) bracket those changes with `begin_transaction` and
/// `commit_transaction`. The bracket nests, so an outer caller can group
/// several already-bracketed edits into one undo entry.
#[derive(Debug)]
pub(crate) struct UndoManager {
    undo_transactions: Vec<UndoTransaction>,
    redo_transactions: Vec<UndoTransaction>,
    ignoring: bool,
    transaction_depth: usize,
    pending: Option<PendingTransaction>,
    pending_intent: Option<EditIntent>,
    coalescing_boundary: bool,
}

impl UndoManager {
    pub(super) fn new() -> Self {
        Self {
            undo_transactions: Vec::new(),
            redo_transactions: Vec::new(),
            ignoring: false,
            transaction_depth: 0,
            pending: None,
            pending_intent: None,
            coalescing_boundary: false,
        }
    }

    /// The intent requested for the next recorded change, taken by the edit
    /// that records it.
    pub(super) fn take_pending_intent(&mut self) -> Option<EditIntent> {
        self.pending_intent.take()
    }

    /// Request the intent to record the next change with.
    pub(super) fn set_pending_intent(&mut self, intent: EditIntent) {
        self.pending_intent = Some(intent);
    }

    pub(super) fn record_transaction(&mut self, change: Change, intent: EditIntent) {
        if self.ignoring {
            return;
        }
        if change.old_range == change.new_range && change.old_text == change.new_text {
            self.break_transaction_coalescing();
            return;
        }

        match self.pending.as_mut() {
            Some(pending) => pending.changes.push(change),
            None => self.push_batch(vec![change], intent),
        }
    }

    /// Open a transaction whose changes commit as one atomic undo entry.
    pub(super) fn begin_transaction(&mut self) {
        self.begin_transaction_with(EditIntent::Atomic);
    }

    /// Open a transaction that commits with `intent`, so a following batch of
    /// the same intent and shape can coalesce into it. Multi-cursor typing uses
    /// this to group a burst of keystrokes the way single-cursor typing does.
    ///
    /// Only the outermost bracket decides the intent. A nested
    /// `begin_transaction` merely keeps the batch open.
    pub(super) fn begin_transaction_with(&mut self, intent: EditIntent) {
        self.transaction_depth += 1;
        if self.transaction_depth == 1 {
            self.pending = Some(PendingTransaction {
                intent,
                changes: Vec::new(),
            });
        }
    }

    pub(super) fn commit_transaction(&mut self) {
        if self.transaction_depth == 0 {
            return;
        }

        self.transaction_depth -= 1;
        if self.transaction_depth > 0 {
            return;
        }

        let Some(pending) = self.pending.take() else {
            return;
        };
        // A composition that ends where it started (typed then canceled) leaves
        // the document untouched and must not become an undo entry.
        if pending.changes.is_empty() || is_noop_batch(&pending.changes) {
            return;
        }
        self.push_batch(pending.changes, pending.intent);
    }

    /// Push one logical edit, which is one or more changes in application
    /// order, onto the undo stack.
    fn push_batch(&mut self, changes: Vec<Change>, intent: EditIntent) {
        if changes.is_empty() {
            return;
        }

        self.redo_transactions.clear();
        let can_coalesce = !self.coalescing_boundary
            && intent != EditIntent::Atomic
            && self.undo_transactions.last().is_some_and(|previous| {
                previous.intent == intent
                    && previous.last_batch_len == changes.len()
                    && previous.changes.len() + changes.len() <= MAX_CHANGES_PER_TRANSACTION
                    && is_adjacent_batch(intent, previous.trailing_batch(), &changes)
            });

        if can_coalesce {
            let previous = self
                .undo_transactions
                .last_mut()
                .expect("coalescing requires a previous transaction");
            previous.last_batch_len = changes.len();
            previous.changes.extend(changes);
            return;
        }

        if self.undo_transactions.len() >= MAX_UNDO_TRANSACTIONS {
            self.undo_transactions.remove(0);
        }
        self.undo_transactions.push(UndoTransaction {
            intent,
            last_batch_len: changes.len(),
            changes,
        });
        self.coalescing_boundary = intent == EditIntent::Atomic;
    }

    pub(super) fn break_transaction_coalescing(&mut self) {
        // While a batch is open the boundary applies to that batch, so never
        // close a bracket the caller still owns.
        if self.transaction_depth == 0 {
            self.commit_all_transactions();
        }
        self.coalescing_boundary = true;
    }

    /// Close every open bracket, committing whatever they collected.
    fn commit_all_transactions(&mut self) {
        if self.transaction_depth == 0 {
            return;
        }
        self.transaction_depth = 1;
        self.commit_transaction();
    }

    pub(super) fn is_ignoring(&self) -> bool {
        self.ignoring
    }

    pub(super) fn set_ignoring(&mut self, ignoring: bool) {
        self.ignoring = ignoring;
        if ignoring {
            self.commit_all_transactions();
        }
    }

    pub(super) fn clear(&mut self) {
        self.undo_transactions.clear();
        self.redo_transactions.clear();
        self.transaction_depth = 0;
        self.pending = None;
        self.pending_intent = None;
        self.coalescing_boundary = false;
    }

    pub(super) fn undo(&mut self) -> Option<Vec<Change>> {
        self.commit_all_transactions();
        let transaction = self.undo_transactions.pop()?;
        let changes = transaction.changes.iter().rev().cloned().collect();
        self.redo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(changes)
    }

    pub(super) fn redo(&mut self) -> Option<Vec<Change>> {
        self.commit_all_transactions();
        let transaction = self.redo_transactions.pop()?;
        let changes = transaction.changes.clone();
        self.undo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(changes)
    }

    #[cfg(test)]
    pub(super) fn has_undos(&self) -> bool {
        !self.undo_transactions.is_empty()
    }
}

impl UndoTransaction {
    /// The changes contributed by the most recent batch, which are the ones a
    /// following batch has to be adjacent to.
    fn trailing_batch(&self) -> &[Change] {
        &self.changes[self.changes.len() - self.last_batch_len..]
    }
}

/// True when a chain of changes that each rewrite the region the previous one
/// produced leaves the document exactly as it was found.
///
/// Changes that do not form such a chain (multi-cursor batches, for one) always
/// report `false`, so this only ever collapses the single-region case.
fn is_noop_batch(changes: &[Change]) -> bool {
    let Some(first) = changes.first() else {
        return true;
    };

    let mut last = first;
    for change in &changes[1..] {
        if last.new_range.start != change.old_range.start
            || last.new_range.end != change.old_range.end
        {
            return false;
        }
        last = change;
    }

    first.old_range.start == last.new_range.start
        && first.old_range.end == last.new_range.end
        && first.old_text == last.new_text
}

/// True when every change in `current` continues the change at the same
/// position in `previous`, so the two batches are one editing gesture.
fn is_adjacent_batch(intent: EditIntent, previous: &[Change], current: &[Change]) -> bool {
    previous.len() == current.len()
        && previous
            .iter()
            .zip(current)
            .all(|(previous, current)| is_adjacent(intent, previous, current))
}

fn is_adjacent(intent: EditIntent, previous: &Change, current: &Change) -> bool {
    match intent {
        EditIntent::Typing => {
            previous.old_range.is_empty()
                && current.old_range.is_empty()
                && !previous.new_text.contains(['\n', '\r'])
                && !current.new_text.contains(['\n', '\r'])
                && previous.new_range.end == current.old_range.start
        }
        EditIntent::Backspace => {
            previous.new_text.is_empty()
                && current.new_text.is_empty()
                && current.old_range.end == previous.old_range.start
        }
        EditIntent::DeleteForward => {
            previous.new_text.is_empty()
                && current.new_text.is_empty()
                && current.old_range.start == previous.old_range.start
        }
        EditIntent::Atomic => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typing_change(offset: usize, text: &str) -> Change {
        let end = offset + text.len();
        Change::new(
            offset..offset,
            "",
            offset..end,
            text,
            (offset..offset).into(),
            (end..end).into(),
        )
    }

    #[test]
    fn adjacent_typing_transactions_coalesce() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);

        assert_eq!(manager.undo().unwrap().len(), 2);
        assert!(manager.undo().is_none());
    }

    #[test]
    fn explicit_transaction_collects_multiple_changes() {
        let mut manager = UndoManager::new();
        manager.begin_transaction();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);
        manager.commit_transaction();

        // One undo entry that replays its changes in reverse application
        // order.
        let transaction = manager.undo().unwrap();
        assert_eq!(transaction.len(), 2);
        assert_eq!(transaction[0].new_text, "b");
        assert_eq!(transaction[1].new_text, "a");
        assert!(manager.undo().is_none());
    }

    #[test]
    fn nested_transactions_commit_as_one_entry() {
        let mut manager = UndoManager::new();
        manager.begin_transaction();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.begin_transaction();
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);
        manager.commit_transaction();
        // The inner bracket does not close the transaction.
        assert!(!manager.has_undos());
        manager.record_transaction(typing_change(2, "c"), EditIntent::Typing);
        manager.commit_transaction();

        assert_eq!(manager.undo().unwrap().len(), 3);
        assert!(manager.undo().is_none());
    }

    #[test]
    fn batches_of_the_same_shape_and_intent_coalesce() {
        let mut manager = UndoManager::new();

        // Two cursors typing "a" then "b", as a multi-cursor keystroke burst.
        manager.begin_transaction_with(EditIntent::Typing);
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(5, "a"), EditIntent::Typing);
        manager.commit_transaction();
        manager.begin_transaction_with(EditIntent::Typing);
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);
        manager.record_transaction(typing_change(6, "b"), EditIntent::Typing);
        manager.commit_transaction();

        assert_eq!(manager.undo().unwrap().len(), 4);
        assert!(manager.undo().is_none());
    }

    #[test]
    fn a_batch_does_not_coalesce_into_a_differently_shaped_one() {
        let mut manager = UndoManager::new();

        manager.begin_transaction_with(EditIntent::Typing);
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(5, "a"), EditIntent::Typing);
        manager.commit_transaction();
        // One cursor left: a single change cannot continue a two-cursor batch.
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);

        assert_eq!(manager.undo().unwrap().len(), 1);
        assert_eq!(manager.undo().unwrap().len(), 2);
    }

    #[test]
    fn an_atomic_batch_never_coalesces() {
        let mut manager = UndoManager::new();

        manager.begin_transaction();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.commit_transaction();
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);

        assert_eq!(manager.undo().unwrap().len(), 1);
        assert_eq!(manager.undo().unwrap().len(), 1);
    }

    #[test]
    fn limits_the_number_of_retained_transactions() {
        let mut manager = UndoManager::new();

        for offset in 0..1_100 {
            manager.record_transaction(typing_change(offset, "a"), EditIntent::Atomic);
        }

        for _ in 0..MAX_UNDO_TRANSACTIONS {
            assert!(manager.undo().is_some());
        }
        assert!(manager.undo().is_none());
    }

    #[test]
    fn splits_a_coalesced_transaction_before_its_change_list_grows_too_large() {
        let mut manager = UndoManager::new();

        for offset in 0..1_100 {
            manager.record_transaction(typing_change(offset, "a"), EditIntent::Typing);
        }

        assert_eq!(manager.undo().unwrap().len(), 100);
        assert_eq!(manager.undo().unwrap().len(), MAX_CHANGES_PER_TRANSACTION);
        assert!(manager.undo().is_none());
    }
}

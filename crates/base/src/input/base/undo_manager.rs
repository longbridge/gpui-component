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
}

/// Coordinates undo and redo as explicit editing transactions.
///
/// Each edit first creates a transaction. Compatible adjacent transactions
/// may then coalesce until an explicit boundary is encountered. Callers that
/// perform one logical edit through several callbacks (currently IME
/// composition) bracket those changes with `begin_transaction` and
/// `commit_transaction`.
#[derive(Debug)]
pub(crate) struct UndoManager {
    undo_transactions: Vec<UndoTransaction>,
    redo_transactions: Vec<UndoTransaction>,
    ignoring: bool,
    transaction_open: bool,
    pending_change: Option<Change>,
    pub(crate) pending_intent: Option<EditIntent>,
    coalescing_boundary: bool,
}

impl UndoManager {
    pub(super) fn new() -> Self {
        Self {
            undo_transactions: Vec::new(),
            redo_transactions: Vec::new(),
            ignoring: false,
            transaction_open: false,
            pending_change: None,
            pending_intent: None,
            coalescing_boundary: false,
        }
    }

    pub(super) fn record_transaction(&mut self, change: Change, intent: EditIntent) {
        if self.ignoring {
            return;
        }
        if change.old_range == change.new_range && change.old_text == change.new_text {
            self.break_transaction_coalescing();
            return;
        }

        if self.transaction_open {
            if let Some(pending) = self.pending_change.as_mut() {
                pending.new_range = change.new_range;
                pending.new_text = change.new_text;
                pending.selection_after = change.selection_after;
            } else {
                self.pending_change = Some(change);
            }
        } else {
            self.push_transaction(change, intent);
        }
    }

    pub(super) fn begin_transaction(&mut self) {
        if self.transaction_open {
            return;
        }
        self.transaction_open = true;
        self.pending_change = None;
    }

    pub(super) fn commit_transaction(&mut self) {
        if !self.transaction_open {
            return;
        }
        self.transaction_open = false;
        if let Some(change) = self.pending_change.take()
            && (change.old_range != change.new_range || change.old_text != change.new_text)
        {
            self.push_transaction(change, EditIntent::Atomic);
        }
    }

    fn push_transaction(&mut self, change: Change, intent: EditIntent) {
        self.redo_transactions.clear();
        let can_coalesce = !self.coalescing_boundary
            && intent != EditIntent::Atomic
            && self.undo_transactions.last().is_some_and(|previous| {
                previous.intent == intent
                    && previous.changes.len() < MAX_CHANGES_PER_TRANSACTION
                    && previous
                        .changes
                        .last()
                        .is_some_and(|last| is_adjacent(intent, last, &change))
            });

        if can_coalesce {
            self.undo_transactions
                .last_mut()
                .expect("coalescing requires a previous transaction")
                .changes
                .push(change);
            return;
        }

        if self.undo_transactions.len() >= MAX_UNDO_TRANSACTIONS {
            self.undo_transactions.remove(0);
        }
        self.undo_transactions.push(UndoTransaction {
            intent,
            changes: vec![change],
        });
        self.coalescing_boundary = intent == EditIntent::Atomic;
    }

    pub(super) fn break_transaction_coalescing(&mut self) {
        self.commit_transaction();
        self.coalescing_boundary = true;
    }

    pub(super) fn is_ignoring(&self) -> bool {
        self.ignoring
    }

    pub(super) fn set_ignoring(&mut self, ignoring: bool) {
        self.ignoring = ignoring;
        if ignoring {
            self.commit_transaction();
        }
    }

    pub(super) fn clear(&mut self) {
        self.undo_transactions.clear();
        self.redo_transactions.clear();
        self.transaction_open = false;
        self.pending_change = None;
        self.pending_intent = None;
        self.coalescing_boundary = false;
    }

    pub(super) fn undo(&mut self) -> Option<Vec<Change>> {
        self.commit_transaction();
        let transaction = self.undo_transactions.pop()?;
        let changes = transaction.changes.iter().rev().cloned().collect();
        self.redo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(changes)
    }

    pub(super) fn redo(&mut self) -> Option<Vec<Change>> {
        self.commit_transaction();
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
    use crate::input::Selection;

    fn typing_change(offset: usize, text: &str) -> Change {
        let end = offset + text.len();
        Change::new(
            offset..offset,
            "",
            offset..end,
            text,
            Selection::new(offset, offset),
            Selection::new(end, end),
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
        manager.record_transaction(typing_change(0, "ab"), EditIntent::Typing);
        manager.commit_transaction();

        let transaction = manager.undo().unwrap();
        assert_eq!(transaction.len(), 1);
        assert_eq!(transaction[0].new_text, "ab");
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

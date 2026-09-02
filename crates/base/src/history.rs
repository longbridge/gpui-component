use instant::{Duration, Instant};
use std::fmt::Debug;

/// A HistoryItem represents a single change in the history.
/// It must implement Clone and PartialEq to be used in the History.
pub trait HistoryItem: Clone + PartialEq {
    fn version(&self) -> usize;
    fn set_version(&mut self, version: usize);
}

/// A linear history of items with a cursor: what came before can be taken
/// back with `undo`, and taken back again with `redo`.
///
/// The items are whatever a model wants to remember. `TilesState` records
/// tile bounds changes, so `undo` reverts a drag. A workspace can record the
/// locations it visits, so `undo` is back and `redo` is forward. With
/// [`unique`](Self::unique) and a cap, a history is a most-recent-first list
/// — the stocks a user opened, say — where `push` moves a revisited item to
/// the front.
///
/// Pushing after an undo starts a new branch: the undone items are dropped,
/// as a browser drops its forward pages when a new page is opened.
#[derive(Debug)]
pub struct History<I: HistoryItem> {
    undos: Vec<I>,
    redos: Vec<I>,
    last_changed_at: Instant,
    version: usize,
    ignore: bool,
    max_undos: usize,
    group_interval: Option<Duration>,
    grouping: bool,
    unique: bool,
}

impl<I> History<I>
where
    I: HistoryItem,
{
    pub fn new() -> Self {
        Self {
            undos: Default::default(),
            redos: Default::default(),
            ignore: false,
            last_changed_at: Instant::now(),
            version: 0,
            max_undos: 1000,
            group_interval: None,
            grouping: false,
            unique: false,
        }
    }

    /// Set the maximum number of undo steps to keep, defaults to 1000.
    pub fn max_undos(mut self, max_undos: usize) -> Self {
        self.max_undos = max_undos;
        self
    }

    /// Set the history to be unique, defaults to false.
    /// If set to true, the history will only keep unique changes.
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Set the interval in milliseconds to group changes, defaults to None.
    pub fn group_interval(mut self, group_interval: Duration) -> Self {
        self.group_interval = Some(group_interval);
        self
    }

    /// Start grouping changes, this will prevent the version from being incremented until `end_grouping` is called.
    pub fn start_grouping(&mut self) {
        self.grouping = true;
    }

    /// End grouping changes, this will allow the version to be incremented again.
    pub fn end_grouping(&mut self) {
        self.grouping = false;
    }

    /// Increment the version number if the last change was made more than `GROUP_INTERVAL` milliseconds ago.
    fn inc_version(&mut self) -> usize {
        let t = Instant::now();
        if !self.grouping && Some(self.last_changed_at.elapsed()) > self.group_interval {
            self.version += 1;
        }

        self.last_changed_at = t;
        self.version
    }

    /// Get the current version number.
    pub fn version(&self) -> usize {
        self.version
    }

    /// Returns whether history recording is currently ignored.
    pub fn is_ignoring(&self) -> bool {
        self.ignore
    }

    /// Sets whether history recording is currently ignored.
    pub fn set_ignoring(&mut self, ignoring: bool) {
        self.ignore = ignoring;
    }

    /// Pushes an item, dropping anything that had been undone.
    pub fn push(&mut self, item: I) {
        let version = self.inc_version();
        self.redos.clear();

        if self.undos.len() >= self.max_undos {
            self.undos.remove(0);
        }

        if self.unique {
            self.undos.retain(|c| *c != item);
        }

        let mut item = item;
        item.set_version(version);
        self.undos.push(item);
    }

    /// The most recent item, the one `undo` would take back.
    pub fn current(&self) -> Option<&I> {
        self.undos.last()
    }

    /// Replaces the most recent item in place, keeping its version, so the
    /// history does not grow: a location that was recorded before it had
    /// finished loading is corrected rather than followed by a duplicate.
    /// Pushes when there is nothing to replace.
    pub fn replace_current(&mut self, mut item: I) {
        match self.undos.last_mut() {
            Some(current) => {
                item.set_version(current.version());
                *current = item;
            }
            None => self.push(item),
        }
    }

    /// Keeps only the items `keep` accepts, on both sides of the cursor. Use
    /// it when items can stop being valid — a location whose tab was closed.
    pub fn retain(&mut self, mut keep: impl FnMut(&I) -> bool) {
        self.undos.retain(&mut keep);
        self.redos.retain(&mut keep);
    }

    /// Get the undo stack.
    pub fn undos(&self) -> &Vec<I> {
        &self.undos
    }

    /// Get the redo stack.
    pub fn redos(&self) -> &Vec<I> {
        &self.redos
    }

    /// Clear the undo and redo stacks.
    pub fn clear(&mut self) {
        self.undos.clear();
        self.redos.clear();
    }

    /// Undo the last change and return the changes that were undone.
    pub fn undo(&mut self) -> Option<Vec<I>> {
        if let Some(first_change) = self.undos.pop() {
            let mut changes = vec![first_change.clone()];
            // pick the next all changes with the same version
            while self
                .undos
                .iter()
                .filter(|c| c.version() == first_change.version())
                .count()
                > 0
            {
                let change = self.undos.pop().unwrap();
                changes.push(change);
            }

            self.redos.extend(changes.clone());
            Some(changes)
        } else {
            None
        }
    }

    /// Redo the last undone change and return the changes that were redone.
    pub fn redo(&mut self) -> Option<Vec<I>> {
        if let Some(first_change) = self.redos.pop() {
            let mut changes = vec![first_change.clone()];
            // pick the next all changes with the same version
            while self
                .redos
                .iter()
                .filter(|c| c.version() == first_change.version())
                .count()
                > 0
            {
                let change = self.redos.pop().unwrap();
                changes.push(change);
            }
            self.undos.extend(changes.clone());
            Some(changes)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TabIndex {
        tab_index: usize,
        version: usize,
    }

    impl PartialEq for TabIndex {
        fn eq(&self, other: &Self) -> bool {
            self.tab_index == other.tab_index
        }
    }

    impl From<usize> for TabIndex {
        fn from(value: usize) -> Self {
            TabIndex {
                tab_index: value,
                version: 0,
            }
        }
    }

    impl HistoryItem for TabIndex {
        fn version(&self) -> usize {
            self.version
        }
        fn set_version(&mut self, version: usize) {
            self.version = version;
        }
    }

    #[test]
    fn test_history() {
        let mut history: History<TabIndex> = History::new().max_undos(100);
        history.push(0.into());
        history.push(3.into());
        history.push(2.into());
        history.push(1.into());

        assert_eq!(history.version(), 4);
        let changes = history.undo().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tab_index, 1);

        let changes = history.undo().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tab_index, 2);

        history.push(5.into());

        // A push after undo starts a new branch; 2 and 1 are gone.
        assert!(history.redo().is_none());

        let changes = history.undo().unwrap();
        assert_eq!(changes[0].tab_index, 5);

        let changes = history.undo().unwrap();
        assert_eq!(changes[0].tab_index, 3);

        let changes = history.undo().unwrap();
        assert_eq!(changes[0].tab_index, 0);

        assert_eq!(history.undo().is_none(), true);
    }

    #[test]
    fn test_unique_history() {
        let mut history: History<TabIndex> = History::new().max_undos(100).unique();

        // Push some items
        history.push(0.into());
        history.push(1.into());
        history.push(1.into()); // Duplicate, should be ignored
        history.push(2.into());
        history.push(1.into()); // Duplicate, should be remove old, and add new

        // Check the version and undo stack
        assert_eq!(history.version(), 5);
        assert_eq!(history.undos().len(), 3);
        assert_eq!(history.undos().last().unwrap().tab_index, 1);

        // Undo the last change
        let changes = history.undo().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tab_index, 1);

        assert_eq!(history.redos().len(), 1);
        // A revisit moves the item to the front and drops the undone branch
        history.push(2.into());

        assert_eq!(history.undos().len(), 2);
        assert_eq!(history.undos().last().unwrap().tab_index, 2);
        assert!(history.redos().is_empty());
        assert!(history.redo().is_none());

        // Push another item
        history.push(3.into());

        // Check the version and undo stack
        assert_eq!(history.version(), 7);
        assert_eq!(history.undos().len(), 3);

        // Undo all changes
        for _ in 0..3 {
            history.undo();
        }

        // Check the undo stack is empty and redo stack has all changes
        assert_eq!(history.undos().len(), 0);
        assert_eq!(history.redos().len(), 3);
    }

    #[test]
    fn revisits_keep_every_step_without_unique() {
        let mut history: History<TabIndex> = History::new();
        history.push(0.into());
        history.push(1.into());
        history.push(0.into());

        assert_eq!(history.undos().len(), 3);
        assert_eq!(history.undo().unwrap()[0].tab_index, 0);
        assert_eq!(history.undo().unwrap()[0].tab_index, 1);
        assert_eq!(history.current().map(|item| item.tab_index), Some(0));
        assert!(history.undo().is_some());
        assert!(history.undo().is_none());
    }

    #[test]
    fn replace_current_keeps_the_version_and_the_length() {
        let mut history: History<TabIndex> = History::new();
        history.replace_current(7.into());
        assert_eq!(history.undos().len(), 1);

        history.push(1.into());
        let version = history.current().unwrap().version;
        history.replace_current(2.into());

        assert_eq!(history.undos().len(), 2);
        let current = history.current().unwrap();
        assert_eq!(current.tab_index, 2);
        assert_eq!(current.version, version);
    }

    #[test]
    fn retain_prunes_both_sides_of_the_cursor() {
        let mut history: History<TabIndex> = History::new();
        for tab in [0, 1, 2, 3] {
            history.push(tab.into());
        }
        history.undo();
        history.undo();

        history.retain(|item| item.tab_index % 2 == 0);

        assert_eq!(history.current().map(|item| item.tab_index), Some(0));
        assert_eq!(history.redos().len(), 1);
        assert_eq!(history.redo().unwrap()[0].tab_index, 2);
    }
}

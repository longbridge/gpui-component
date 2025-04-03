use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

pub trait HistoryItem: Clone + PartialEq {
    fn version(&self) -> usize;
    fn set_version(&mut self, version: usize);
}

/// The History is used to keep track of changes to a model and to allow undo and redo operations.
///
/// This is now used in Input for undo/redo operations. You can also use this in
/// your own models to keep track of changes, for example to track the tab
/// history for prev/next features.
///
/// ## Use cases
///
/// - Undo/redo operations in Input
/// - Tracking tab history for prev/next features
#[derive(Debug)]
pub struct History<I: HistoryItem> {
    undos: Vec<I>,
    redos: Vec<I>,
    last_changed_at: Instant,
    version: usize,
    pub(crate) ignore: bool,
    max_undo: usize,
    group_interval: Option<Duration>,
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
            max_undo: 1000,
            group_interval: None,
            unique: false,
        }
    }

    /// Set the maximum number of undo steps to keep, defaults to 1000.
    pub fn max_undo(mut self, max_undo: usize) -> Self {
        self.max_undo = max_undo;
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

    /// Increment the version number if the last change was made more than `GROUP_INTERVAL` milliseconds ago.
    fn inc_version(&mut self) -> usize {
        let t = Instant::now();
        if Some(self.last_changed_at.elapsed()) > self.group_interval {
            self.version += 1;
        }

        self.last_changed_at = t;
        self.version
    }

    /// Get the current version number.
    pub fn version(&self) -> usize {
        self.version
    }

    pub fn push(&mut self, item: I) {
        let version = self.inc_version();

        if self.undos.len() >= self.max_undo {
            self.undos.remove(0);
        }

        if self.unique {
            self.undos.retain(|c| *c != item);
            self.redos.retain(|c| *c != item);
        }

        let mut item = item;
        item.set_version(version);
        self.undos.push(item);
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
        let mut history: History<TabIndex> = History::new().max_undo(100);
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

        let changes = history.redo().unwrap();
        assert_eq!(changes[0].tab_index, 2);

        let changes = history.redo().unwrap();
        assert_eq!(changes[0].tab_index, 1);

        let changes = history.undo().unwrap();
        assert_eq!(changes[0].tab_index, 1);

        let changes = history.undo().unwrap();
        assert_eq!(changes[0].tab_index, 2);

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
        let mut history: History<TabIndex> = History::new().max_undo(100).unique();

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
        // Push duplicate, should be ignored
        history.push(2.into());

        assert_eq!(history.undos().len(), 2);
        assert_eq!(history.redos().len(), 1);

        // Redo the last undone change
        let changes = history.redo().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tab_index, 1);

        // Push another item
        history.push(3.into());

        // Check the version and undo stack
        assert_eq!(history.version(), 7);
        assert_eq!(history.undos().len(), 4);

        // Undo all changes
        for _ in 0..4 {
            history.undo();
        }

        // Check the undo stack is empty and redo stack has all changes
        assert_eq!(history.undos().len(), 0);
        assert_eq!(history.redos().len(), 4);
    }
}

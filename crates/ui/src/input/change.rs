use std::{fmt::Debug, ops::Range};

use crate::history::HistoryItem;

#[derive(Debug, PartialEq, Clone)]
pub struct Change {
    pub(crate) old_range: Range<usize>,
    pub(crate) old_text: String,
    pub(crate) new_range: Range<usize>,
    pub(crate) new_text: String,
    version: usize,
}

impl Change {
    pub fn new(
        old_range: Range<usize>,
        old_text: &str,
        new_range: Range<usize>,
        new_text: &str,
    ) -> Self {
        Self {
            old_range,
            old_text: old_text.to_string(),
            new_range,
            new_text: new_text.to_string(),
            version: 0,
        }
    }
}

impl HistoryItem for Change {
    fn version(&self) -> usize {
        self.version
    }

    fn set_version(&mut self, version: usize) {
        self.version = version;
    }
}

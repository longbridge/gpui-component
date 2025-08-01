use std::fmt::Debug;

/// Represents an index path in a list, which consists of a section index,
///
/// The default values for section, row, and column are all set to 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexPath {
    /// The section index.
    pub section: usize,
    /// The item index in the section.
    pub row: usize,
    /// The column index.
    pub column: usize,
}

impl IndexPath {
    /// Create a new index path with the specified section and row.
    ///
    /// The `column` is set to 0 by default.
    pub fn new(section: usize, row: usize) -> Self {
        IndexPath {
            section,
            row,
            ..Default::default()
        }
    }

    /// Set the section for the index path.
    pub fn section(mut self, section: usize) -> Self {
        self.section = section;
        self
    }

    /// Set the row for the index path.
    pub fn row(mut self, row: usize) -> Self {
        self.row = row;
        self
    }

    /// Set the column for the index path.
    pub fn column(mut self, column: usize) -> Self {
        self.column = column;
        self
    }

    /// Check if the self is equal to the given index path (Same section and row).
    pub fn eq_row(&self, index: IndexPath) -> bool {
        self.section == index.section && self.row == index.row
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_index_path() {
        let mut index_path = IndexPath::default();
        assert_eq!(index_path.section, 0);
        assert_eq!(index_path.row, 0);
        assert_eq!(index_path.column, 0);

        index_path = index_path.section(1).row(2).column(3);
        assert_eq!(index_path.section, 1);
        assert_eq!(index_path.row, 2);
        assert_eq!(index_path.column, 3);
    }
}

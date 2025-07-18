#[derive(Debug, Clone, Copy)]
pub struct TableCell {
    /// The number of columns spanned by the cell, default is 1.
    pub col_span: usize,
    /// The number of rows spanned by the cell, default is 1.
    pub row_span: usize,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            col_span: 1,
            row_span: 1,
        }
    }
}

impl TableCell {
    /// Create a new table cell with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of columns spanned by the cell.
    pub fn col_span(mut self, col_span: usize) -> Self {
        self.col_span = col_span;
        self
    }

    /// Set the number of rows spanned by the cell.
    pub fn row_span(mut self, row_span: usize) -> Self {
        self.row_span = row_span;
        self
    }
}

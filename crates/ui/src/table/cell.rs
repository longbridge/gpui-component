#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellOption {
    /// The number of columns spanned by the cell, default is 1.
    pub col_span: usize,
    /// The number of rows spanned by the cell, default is 1.
    pub row_span: usize,
}

impl Default for CellOption {
    fn default() -> Self {
        Self {
            col_span: 1,
            row_span: 1,
        }
    }
}

impl CellOption {
    pub fn col_span(mut self, col_span: usize) -> Self {
        self.col_span = col_span;
        self
    }

    pub fn row_span(mut self, row_span: usize) -> Self {
        self.row_span = row_span;
        self
    }
}

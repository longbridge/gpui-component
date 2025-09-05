pub(crate) trait RopeExt {
    /// Get the index of (line, column) (0-based) from the byte offset (0-based).
    /// If the offset is out of bounds, return the last line and column.
    ///
    /// If the `offset` is out of bounds, it returns (0, 0).
    fn line_column(&self, byte_offset: usize) -> (usize, usize);
    /// Get the byte offset (0-based) from the line, column (0-based).
    ///
    /// Return the last line, if line is out of bounds.
    /// Return the end column of line, if the column is out of bounds.
    fn line_column_to_byte(&self, line_ix: usize, column_ix: usize) -> usize;
}

impl RopeExt for ropey::Rope {
    fn line_column(&self, offset: usize) -> (usize, usize) {
        let Ok(line_ix) = self.try_byte_to_line(offset) else {
            return (0, 0);
        };

        let line = self.line(line_ix);
        let line_start_byte = self.line_to_byte(line_ix);
        let line_offset = offset.saturating_sub(line_start_byte);

        let column_ix = line
            .try_byte_to_char(line_offset)
            .unwrap_or(line.len_chars());

        (line_ix, column_ix)
    }

    fn line_column_to_byte(&self, line_ix: usize, column_ix: usize) -> usize {
        let line_ix = self.len_lines().saturating_sub(1).min(line_ix);
        let line = self.line(line_ix);

        self.line_to_byte(line_ix)
            + line
                .try_char_to_byte(column_ix)
                .unwrap_or(line.len_bytes().saturating_sub(1))
    }
}

pub(crate) trait RopeExt {
    /// Get the index of (line, column) (0-based) from the byte offset (0-based).
    /// If the offset is out of bounds, return the last line and column.
    fn line_column(&self, byte_offset: usize) -> (usize, usize);
    /// Get the byte offset (0-based) from the line, column (0-based).
    fn line_column_to_byte(&self, line_ix: usize, column_ix: usize) -> usize;
}

impl RopeExt for ropey::Rope {
    fn line_column(&self, offset: usize) -> (usize, usize) {
        let line_ix = self.byte_to_line(offset);
        let line_offset = offset.saturating_sub(self.line_to_byte(line_ix));
        let line = self.line(line_ix);
        let column_ix = line.byte_to_char(line_offset);

        (line_ix, column_ix)
    }

    fn line_column_to_byte(&self, line_ix: usize, column_ix: usize) -> usize {
        let line = self.line(line_ix);
        self.line_to_byte(line_ix) + line.char_to_byte(column_ix)
    }
}

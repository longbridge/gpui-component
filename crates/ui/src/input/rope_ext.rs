use rope::{Point, Rope};

use crate::input::Position;

/// An extension trait for `Rope` to provide additional utility methods.
pub trait RopeExt {
    /// Get the line at the given row (0-based) index, including the `\r` at the end, but not `\n`.
    ///
    /// Return empty rope if the row (0-based) is out of bounds.
    fn line(&self, row: usize) -> Rope;

    /// Start offset of the line at the given row (0-based) index.
    fn line_start_offset(&self, row: usize) -> usize;

    /// Line the end offset (including `\n`) of the line at the given row (0-based) index.
    ///
    /// Return the end of the rope if the row is out of bounds.
    fn line_end_offset(&self, row: usize) -> usize;

    /// Return the number of lines in the rope.
    fn lines_len(&self) -> usize;

    /// Return the lines iterator.
    ///
    /// Each line is including the `\n` at the end, but not `\n`.
    fn lines(&self) -> RopeLines;

    /// Check is equal to another rope.
    fn eq(&self, other: &Rope) -> bool;

    /// Total number of characters in the rope.
    fn chars_count(&self) -> usize;

    /// Get char at the given offset (byte).
    ///
    /// If the offset is in the middle of a multi-byte character will panic.
    ///
    /// If the offset is out of bounds, return None.
    fn char_at(&self, offset: usize) -> Option<char>;

    /// Get the byte offset from the given line, column [`Position`] (0-based).
    fn position_to_offset(&self, line_col: &Position) -> usize;

    /// Get the line, column [`Position`] (0-based) from the given byte offset.
    fn offset_to_position(&self, offset: usize) -> Position;
}

/// An iterator over the lines of a `Rope`.
pub struct RopeLines {
    row: usize,
    end_row: usize,
    rope: Rope,
}

impl RopeLines {
    /// Create a new `RopeLines` iterator.
    pub fn new(rope: Rope) -> Self {
        let end_row = rope.lines_len();
        Self {
            row: 0,
            end_row,
            rope,
        }
    }
}

impl Iterator for RopeLines {
    type Item = Rope;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= self.end_row {
            return None;
        }

        let line = self.rope.line(self.row);
        self.row += 1;
        Some(line)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.row = self.row.saturating_add(n);
        self.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end_row - self.row;
        (len, Some(len))
    }
}

impl std::iter::ExactSizeIterator for RopeLines {}
impl std::iter::FusedIterator for RopeLines {}

impl RopeExt for Rope {
    fn line(&self, row: usize) -> Rope {
        let start = self.line_start_offset(row);
        let end = start + self.line_len(row as u32) as usize;
        self.slice(start..end)
    }

    fn line_start_offset(&self, row: usize) -> usize {
        let row = row as u32;
        self.point_to_offset(Point::new(row, 0))
    }

    fn position_to_offset(&self, pos: &Position) -> usize {
        let line = self.line(pos.line);
        self.line_start_offset(pos.line)
            + line
                .chars()
                .take(pos.character)
                .map(|c| c.len_utf8())
                .sum::<usize>()
    }

    fn offset_to_position(&self, offset: usize) -> Position {
        let point = self.offset_to_point(offset);
        let line = self.line(point.row as usize);
        let character = line.slice(0..point.column as usize).chars().count();
        Position::new(point.row as usize, character)
    }

    fn line_end_offset(&self, row: usize) -> usize {
        if row > self.max_point().row as usize {
            return self.len();
        }

        self.line_start_offset(row) + self.line_len(row as u32) as usize
    }

    fn lines_len(&self) -> usize {
        self.max_point().row as usize + 1
    }

    fn lines(&self) -> RopeLines {
        RopeLines::new(self.clone())
    }

    fn eq(&self, other: &Rope) -> bool {
        self.summary() == other.summary()
    }

    fn chars_count(&self) -> usize {
        self.chars().count()
    }

    fn char_at(&self, offset: usize) -> Option<char> {
        if offset > self.len() {
            return None;
        }

        self.slice(offset..self.len()).chars().next()
    }
}

#[cfg(test)]
mod tests {
    use rope::Rope;

    use crate::input::{Position, RopeExt as _};

    #[test]
    fn test_line() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert_eq!(rope.line(0).to_string(), "Hello");
        assert_eq!(rope.line(1).to_string(), "World\r");
        assert_eq!(rope.line(2).to_string(), "This is a test 中文");
        assert_eq!(rope.line(3).to_string(), "Rope");

        // over bounds
        assert_eq!(rope.line(6).to_string(), "");
    }

    #[test]
    fn test_lines_len() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert_eq!(rope.lines_len(), 4);
        let rope = Rope::from("");
        assert_eq!(rope.lines_len(), 1);
        let rope = Rope::from("Single line");
        assert_eq!(rope.lines_len(), 1);
    }

    #[test]
    fn test_eq() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert!(rope.eq(&Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope")));
        assert!(!rope.eq(&Rope::from("Hello\nWorld")));

        let rope1 = rope.clone();
        assert!(rope.eq(&rope1));
    }

    #[test]
    fn test_lines() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        let lines: Vec<_> = rope.lines().map(|r| r.to_string()).collect();
        assert_eq!(
            lines,
            vec!["Hello", "World\r", "This is a test 中文", "Rope"]
        );
    }

    #[test]
    fn test_line_start_end_offset() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert_eq!(rope.line_start_offset(0), 0);
        assert_eq!(rope.line_end_offset(0), 5);

        assert_eq!(rope.line_start_offset(1), 6);
        assert_eq!(rope.line_end_offset(1), 12);

        assert_eq!(rope.line_start_offset(2), 13);
        assert_eq!(rope.line_end_offset(2), 34);

        assert_eq!(rope.line_start_offset(3), 35);
        assert_eq!(rope.line_end_offset(3), 39);

        assert_eq!(rope.line_start_offset(4), 39);
        assert_eq!(rope.line_end_offset(4), 39);
    }

    #[test]
    fn test_chars_count() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文🎉\nRope");
        assert_eq!(rope.chars_count(), 36);
        let rope = Rope::from("");
        assert_eq!(rope.chars_count(), 0);
        let rope = Rope::from("Single line");
        assert_eq!(rope.chars_count(), 11);
    }

    #[test]
    fn test_line_column() {
        let rope = Rope::from("a 中文🎉 test\nRope");
        assert_eq!(rope.position_to_offset(&Position::new(0, 3)), "a 中".len());
        assert_eq!(
            rope.position_to_offset(&Position::new(0, 5)),
            "a 中文🎉".len()
        );
        assert_eq!(
            rope.position_to_offset(&Position::new(1, 1)),
            "a 中文🎉 test\nR".len()
        );

        assert_eq!(
            rope.offset_to_position("a 中文🎉 test\nR".len()),
            Position::new(1, 1)
        );
        assert_eq!(
            rope.offset_to_position("a 中文🎉".len()),
            Position::new(0, 5)
        );
    }

    #[test]
    fn test_char_at() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文🎉\nRope");
        assert_eq!(rope.char_at(0), Some('H'));
        assert_eq!(rope.char_at(5), Some('\n'));
        assert_eq!(rope.char_at(13), Some('T'));
        assert_eq!(rope.char_at(28), Some('中'));
        assert_eq!(rope.char_at(34), Some('🎉'));
        assert_eq!(rope.char_at(38), Some('\n'));
        assert_eq!(rope.char_at(50), None);
    }
}

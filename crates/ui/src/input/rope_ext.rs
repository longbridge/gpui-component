use rope::{Point, Rope};

/// An extension trait for `Rope` to provide additional utility methods.
pub trait RopeExt {
    /// Get the line at the given row index, including the `\r` at the end, but not `\n`.
    ///
    /// Return empty rope if the row is out of bounds.
    fn line(&self, row: usize) -> Rope;

    /// Return the number of lines in the rope.
    fn lines_len(&self) -> usize;

    /// Return the lines iterator.
    ///
    /// Each line is including the `\n` at the end, but not `\n`.
    fn lines(&self) -> impl Iterator<Item = Rope>;

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
}

impl RopeExt for Rope {
    fn line(&self, row: usize) -> Rope {
        let row = row as u32;
        let start = self.point_to_offset(Point::new(row, 0));
        let end = start + self.line_len(row) as usize;
        self.slice(start..end)
    }

    fn lines_len(&self) -> usize {
        self.max_point().row as usize + 1
    }

    fn lines(&self) -> impl Iterator<Item = Rope> {
        (0..self.lines_len()).map(move |row| self.line(row))
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

    use crate::input::RopeExt as _;

    #[test]
    fn test_line() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert_eq!(rope.line(0).to_string(), "Hello");
        assert_eq!(rope.line(1).to_string(), "World\r");
        assert_eq!(rope.line(2).to_string(), "This is a test 中文");
        assert_eq!(rope.line(3).to_string(), "Rope");
        assert_eq!(rope.line(4).to_string(), "");
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
        let lines: Vec<_> = rope.lines().into_iter().map(|r| r.to_string()).collect();
        assert_eq!(
            lines,
            vec!["Hello", "World\r", "This is a test 中文", "Rope"]
        );
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

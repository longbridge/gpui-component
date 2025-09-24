use std::ops::Range;

use ropey::{LineType, Rope, RopeSlice};
use sum_tree::Bias;
use tree_sitter::Point;

use crate::input::Position;

/// An extension trait for `Rope` to provide additional utility methods.
pub trait RopeExt {
    /// Start offset of the line at the given row (0-based) index.
    fn line_start_offset(&self, row: usize) -> usize;

    /// Return a line slice at the given row (0-based) index. including `\r` if present, but not `\n`.
    fn slice_row(&self, row: usize) -> RopeSlice<'_>;

    fn slice_rows(&self, rows_range: Range<usize>) -> RopeSlice<'_>;

    fn rows(&self) -> impl Iterator<Item = RopeSlice<'_>> + '_ {
        (0..self.lines_len()).map(|row| self.slice_row(row))
    }

    /// Line the end offset (including `\n`) of the line at the given row (0-based) index.
    ///
    /// Return the end of the rope if the row is out of bounds.
    fn line_end_offset(&self, row: usize) -> usize;

    /// Return the number of lines in the rope.
    fn lines_len(&self) -> usize;

    /// Return the length of the row (0-based) in characters, including `\r` if present, but not `\n`.
    fn line_len(&self, row: usize) -> usize;

    /// Total number of characters in the rope.
    fn chars_count(&self) -> usize;

    /// Replace the text in the given byte range with new text.
    fn replace(&mut self, range: Range<usize>, new_text: &str);

    /// Get char at the given offset (byte).
    ///
    /// - If the offset is in the middle of a multi-byte character will panic.
    /// - If the offset is out of bounds, return None.
    fn char_at(&self, offset: usize) -> Option<char>;

    /// Get the byte offset from the given line, column [`Position`] (0-based).
    fn position_to_offset(&self, line_col: &Position) -> usize;

    /// Get the line, column [`Position`] (0-based) from the given byte offset.
    fn offset_to_position(&self, offset: usize) -> Position;

    fn offset_to_point(&self, offset: usize) -> Point;
    fn point_to_offset(&self, point: Point) -> usize;

    /// Get the word byte range at the given offset (byte).
    fn word_range(&self, offset: usize) -> Option<Range<usize>>;

    /// Get word at the given offset (byte).
    fn word_at(&self, offset: usize) -> String;

    /// Convert offset_utf16 to offset (byte).
    fn offset_utf16_to_offset(&self, offset_utf16: usize) -> usize;

    /// Convert offset (byte) to offset_utf16.
    fn offset_to_offset_utf16(&self, offset: usize) -> usize;

    /// Get a clipped offset (avoid in a char boundary).
    ///
    /// - If Bias::Left and inside the char boundary, return the ix - 1;
    /// - If Bias::Right and in inside char boundary, return the ix + 1;
    /// - Otherwise return the ix.
    fn clip_offset(&self, offset: usize, bias: Bias) -> usize;
}

impl RopeExt for Rope {
    fn slice_row(&self, row: usize) -> RopeSlice<'_> {
        if row >= self.lines_len() {
            return self.slice(0..0);
        }

        let line = self.line(row, LineType::LF_CR);
        if line.len() > 0 && line.chars().last() == Some('\n') {
            line.slice(..line.len().saturating_sub(1))
        } else {
            line
        }
    }

    fn slice_rows(&self, rows_range: Range<usize>) -> RopeSlice<'_> {
        let start = self.line_start_offset(rows_range.start);
        let end = self.line_end_offset(rows_range.end.saturating_sub(1));
        self.slice(start..end)
    }

    fn line_len(&self, row: usize) -> usize {
        self.slice_row(row).len()
    }

    fn line_start_offset(&self, row: usize) -> usize {
        self.point_to_offset(Point::new(row, 0))
    }

    fn offset_to_point(&self, offset: usize) -> Point {
        let offset = self.clip_offset(offset, Bias::Left);
        let row = self.byte_to_line_idx(offset, LineType::LF_CR);
        let line_start = self.line_to_byte_idx(row, LineType::LF_CR);
        let column = offset.saturating_sub(line_start);
        Point::new(row, column)
    }

    fn point_to_offset(&self, point: Point) -> usize {
        if point.row >= self.lines_len() {
            return self.len();
        }

        let line_start = self.line_to_byte_idx(point.row, LineType::LF_CR);
        line_start + point.column
    }

    fn position_to_offset(&self, pos: &Position) -> usize {
        let line = self.slice_row(pos.line as usize);
        self.line_start_offset(pos.line as usize)
            + line
                .chars()
                .take(pos.character as usize)
                .map(|c| c.len_utf8())
                .sum::<usize>()
    }

    fn offset_to_position(&self, offset: usize) -> Position {
        let point = self.offset_to_point(offset);
        let line = self.slice_row(point.row);
        let offset = line.utf16_to_byte_idx(line.byte_to_utf16_idx(point.column));
        let character = line.slice(..offset).chars().count();
        Position::new(point.row as u32, character as u32)
    }

    fn line_end_offset(&self, row: usize) -> usize {
        if row > self.lines_len() {
            return self.len();
        }

        self.line_start_offset(row) + self.line_len(row)
    }

    fn lines_len(&self) -> usize {
        self.len_lines(LineType::LF_CR)
    }

    fn chars_count(&self) -> usize {
        self.chars().count()
    }

    fn char_at(&self, offset: usize) -> Option<char> {
        if offset > self.len() {
            return None;
        }

        self.get_char(offset).ok()
    }

    fn word_range(&self, offset: usize) -> Option<Range<usize>> {
        if offset >= self.len() {
            return None;
        }

        let mut left = String::new();
        let offset = self.clip_offset(offset, Bias::Left);
        for c in self.chars_at(offset).reversed() {
            if c.is_alphanumeric() || c == '_' {
                left.insert(0, c);
            } else {
                break;
            }
        }
        let start = offset.saturating_sub(left.len());

        let right = self
            .chars_at(offset)
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        let end = offset + right.len();

        if start == end {
            None
        } else {
            Some(start..end)
        }
    }

    fn word_at(&self, offset: usize) -> String {
        if let Some(range) = self.word_range(offset) {
            self.slice(range).to_string()
        } else {
            String::new()
        }
    }

    #[inline]
    fn offset_utf16_to_offset(&self, offset_utf16: usize) -> usize {
        if offset_utf16 > self.len_utf16() {
            return self.len();
        }

        self.utf16_to_byte_idx(offset_utf16)
    }

    #[inline]
    fn offset_to_offset_utf16(&self, offset: usize) -> usize {
        if offset > self.len() {
            return self.len_utf16();
        }

        self.byte_to_utf16_idx(offset)
    }

    fn replace(&mut self, range: Range<usize>, new_text: &str) {
        self.remove(range.clone());
        self.insert(range.start, new_text);
    }

    fn clip_offset(&self, offset: usize, bias: Bias) -> usize {
        if offset > self.len() {
            return self.len();
        }

        if self.is_char_boundary(offset) {
            return offset;
        }

        if bias == Bias::Left {
            self.floor_char_boundary(offset)
        } else {
            self.ceil_char_boundary(offset)
        }
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;
    use sum_tree::Bias;
    use tree_sitter::Point;

    use crate::input::{Position, RopeExt};

    #[test]
    fn test_line() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert_eq!(rope.slice_row(0).to_string(), "Hello");
        assert_eq!(rope.slice_row(1).to_string(), "World\r");
        assert_eq!(rope.slice_row(2).to_string(), "This is a test 中文");
        assert_eq!(rope.slice_row(3).to_string(), "Rope");

        // over bounds
        assert_eq!(rope.slice_row(6).to_string(), "");
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
    fn test_rows() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        let lines: Vec<_> = rope.rows().map(|r| r.to_string()).collect();
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
    fn test_offset_to_point() {
        let rope = Rope::from("a 中文🎉 test\nRope");
        assert_eq!(rope.offset_to_point(0), Point::new(0, 0));
        assert_eq!(rope.offset_to_point(1), Point::new(0, 1));
        assert_eq!(rope.offset_to_point("a 中".len()), Point::new(0, 5));
        assert_eq!(rope.offset_to_point("a 中文🎉".len()), Point::new(0, 12));
        assert_eq!(
            rope.offset_to_point("a 中文🎉 test\nR".len()),
            Point::new(1, 1)
        );
    }

    #[test]
    fn test_point_to_offset() {
        let rope = Rope::from("a 中文🎉 test\nRope");
        assert_eq!(rope.point_to_offset(Point::new(0, 0)), 0);
        assert_eq!(rope.point_to_offset(Point::new(0, 1)), 1);
        assert_eq!(rope.point_to_offset(Point::new(0, 5)), "a 中".len());
        assert_eq!(rope.point_to_offset(Point::new(0, 12)), "a 中文🎉".len());
        assert_eq!(
            rope.point_to_offset(Point::new(1, 1)),
            "a 中文🎉 test\nR".len()
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

    #[test]
    fn test_word_at() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文 世界\nRope");
        assert_eq!(rope.word_at(0), "Hello");
        assert_eq!(rope.word_range(0), Some(0..5));
        assert_eq!(rope.word_at(8), "World");
        assert_eq!(rope.word_range(8), Some(6..11));
        assert_eq!(rope.word_at(12), "");
        assert_eq!(rope.word_range(12), None);
        assert_eq!(rope.word_at(13), "This");
        assert_eq!(rope.word_range(13), Some(13..17));
        assert_eq!(rope.word_at(31), "中文");
        assert_eq!(rope.word_range(31), Some(28..34));
        assert_eq!(rope.word_at(38), "世界");
        assert_eq!(rope.word_range(38), Some(35..41));
        assert_eq!(rope.word_at(44), "Rope");
        assert_eq!(rope.word_range(44), Some(42..46));
        assert_eq!(rope.word_at(45), "Rope");
    }

    #[test]
    fn test_offset_utf16_conversion() {
        let rope = Rope::from("hello 中文🎉 test\nRope");
        assert_eq!(rope.offset_to_offset_utf16("hello".len()), 5);
        assert_eq!(rope.offset_to_offset_utf16("hello 中".len()), 7);
        assert_eq!(rope.offset_to_offset_utf16("hello 中文".len()), 8);
        assert_eq!(rope.offset_to_offset_utf16("hello 中文🎉".len()), 10);
        assert_eq!(rope.offset_to_offset_utf16(100), 20);

        assert_eq!(rope.offset_utf16_to_offset(5), "hello".len());
        assert_eq!(rope.offset_utf16_to_offset(7), "hello 中".len());
        assert_eq!(rope.offset_utf16_to_offset(8), "hello 中文".len());
        assert_eq!(rope.offset_utf16_to_offset(10), "hello 中文🎉".len());
        assert_eq!(rope.offset_utf16_to_offset(100), rope.len());
    }

    #[test]
    fn test_replace() {
        let mut rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        rope.replace(6..11, "Universe");
        assert_eq!(
            rope.to_string(),
            "Hello\nUniverse\r\nThis is a test 中文\nRope"
        );

        rope.replace(0..5, "Hi");
        assert_eq!(
            rope.to_string(),
            "Hi\nUniverse\r\nThis is a test 中文\nRope"
        );

        rope.replace(rope.len() - 4..rope.len(), "String");
        assert_eq!(
            rope.to_string(),
            "Hi\nUniverse\r\nThis is a test 中文\nString"
        );
    }

    #[test]
    fn test_clip_offset() {
        let rope = Rope::from("Hello 中文🎉 test\nRope");
        // Inside multi-byte character '中' (3 bytes)
        assert_eq!(rope.clip_offset(5, Bias::Left), 5);
        assert_eq!(rope.clip_offset(7, Bias::Left), 6);
        assert_eq!(rope.clip_offset(7, Bias::Right), 9);
        assert_eq!(rope.clip_offset(9, Bias::Left), 9);

        // Inside multi-byte character '🎉' (4 bytes)
        assert_eq!(rope.clip_offset(13, Bias::Left), 12);
        assert_eq!(rope.clip_offset(13, Bias::Right), 16);
        assert_eq!(rope.clip_offset(16, Bias::Left), 16);

        // At character boundary
        assert_eq!(rope.clip_offset(5, Bias::Left), 5);
        assert_eq!(rope.clip_offset(5, Bias::Right), 5);

        // Out of bounds
        assert_eq!(rope.clip_offset(26, Bias::Left), 26);
        assert_eq!(rope.clip_offset(100, Bias::Left), 26);
    }
}

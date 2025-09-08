use rope::{Point, Rope};

pub trait RopeExt {
    /// Get the line at the given row index, including the `\n`, `\r` at the end.
    ///
    /// Return empty rope if the row is out of bounds.
    fn line(&self, row: usize) -> Rope;

    /// Return the number of lines in the rope.
    fn lines_len(&self) -> usize;

    /// Return the lines iterator.
    ///
    /// Each line is including the `\n`, `\r` at the end.
    fn lines(&self) -> impl Iterator<Item = Rope>;

    /// Check is equal to another rope.
    fn eq(&self, other: &Rope) -> bool;
}

impl RopeExt for Rope {
    fn line(&self, row: usize) -> Rope {
        let row = row as u32;
        let start = self.point_to_offset(Point::new(row, 0));
        let end = start + self.line_len(row) as usize + 1;
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
}

#[cfg(test)]
mod tests {
    use rope::Rope;

    use crate::input::RopeExt as _;

    #[test]
    fn test_line() {
        let rope = Rope::from("Hello\nWorld\r\nThis is a test 中文\nRope");
        assert_eq!(rope.line(0).to_string(), "Hello\n");
        assert_eq!(rope.line(1).to_string(), "World\r\n");
        assert_eq!(rope.line(2).to_string(), "This is a test 中文\n");
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
            vec!["Hello\n", "World\r\n", "This is a test 中文\n", "Rope"]
        );
    }
}

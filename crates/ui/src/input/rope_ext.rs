use rope::{Point, Rope};

pub trait RopeExt {
    /// Get the line at the given row index, including the `\n`, `\r` at the end.
    ///
    /// Return empty rope if the row is out of bounds.
    fn line(&self, row: usize) -> Rope;
}

impl RopeExt for Rope {
    fn line(&self, row: usize) -> Rope {
        let start = self.point_to_offset(Point::new(row as u32, 0));
        let end = start as u32 + self.line_len(row as u32) + 1;
        self.slice(start..end as usize)
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
}

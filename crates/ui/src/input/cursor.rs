use std::{fmt, ops::Range};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

impl Selection {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Clears the selection, setting start and end to 0.
    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
    }
}

impl From<Range<usize>> for Selection {
    fn from(value: Range<usize>) -> Self {
        Self::new(value.start, value.end)
    }
}
impl From<Selection> for Range<usize> {
    fn from(value: Selection) -> Self {
        value.start..value.end
    }
}

/// Line and column position (1-based) in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineColumn {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl LineColumn {
    pub fn new(line: usize, column: usize) -> Self {
        (line, column).into()
    }
}

impl From<(usize, usize)> for LineColumn {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0.max(1),
            column: value.1.max(1),
        }
    }
}

impl From<rope::Point> for LineColumn {
    fn from(value: rope::Point) -> Self {
        Self {
            line: value.row as usize + 1,
            column: value.column as usize + 1,
        }
    }
}

impl From<LineColumn> for rope::Point {
    fn from(value: LineColumn) -> Self {
        Self {
            row: value.line.saturating_sub(1) as u32,
            column: value.column.saturating_sub(1) as u32,
        }
    }
}

impl From<LineColumn> for tree_sitter::Point {
    fn from(value: LineColumn) -> Self {
        Self {
            row: value.line.saturating_sub(1),
            column: value.column.saturating_sub(1),
        }
    }
}

impl fmt::Display for LineColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use crate::input::LineColumn;

    #[test]
    fn test_line_column_from_to() {
        assert_eq!(LineColumn::new(1, 2), LineColumn { line: 1, column: 2 });

        assert_eq!(LineColumn::from((1, 2)), LineColumn { line: 1, column: 2 });
        assert_eq!(
            LineColumn::from((10, 10)),
            LineColumn {
                line: 10,
                column: 10
            }
        );
        assert_eq!(LineColumn::from((0, 0)), LineColumn { line: 1, column: 1 });

        assert_eq!(
            LineColumn::from(rope::Point::new(0, 1)),
            LineColumn { line: 1, column: 2 }
        );
        assert_eq!(
            LineColumn::from(rope::Point::new(10, 9)),
            LineColumn {
                line: 11,
                column: 10
            }
        );
    }

    #[test]
    fn test_line_column_display() {
        assert_eq!(LineColumn::from((1, 2)).to_string(), "1:2");
        assert_eq!(LineColumn::from((10, 10)).to_string(), "10:10");
        assert_eq!(LineColumn::from((0, 0)).to_string(), "1:1");
    }
}

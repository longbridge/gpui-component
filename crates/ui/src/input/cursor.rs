use std::ops::Range;

/// A selection in the text, represented by start and end byte indices.
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

/// Line and column position (0-based) in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Line number (0-based)
    pub line: usize,
    /// The character offset (0-based) in the line
    pub character: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        (line, column).into()
    }
}

impl From<(usize, usize)> for Position {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0,
            character: value.1,
        }
    }
}

impl From<lsp_types::Position> for Position {
    fn from(value: lsp_types::Position) -> Self {
        Self {
            line: value.line as usize,
            character: value.character as usize,
        }
    }
}

impl From<Position> for lsp_types::Position {
    fn from(value: Position) -> Self {
        Self {
            line: value.line as u32,
            character: value.character as u32,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.character + 1)
    }
}

#[cfg(test)]
mod tests {
    use crate::input::Position;

    #[test]
    fn test_line_column_from_to() {
        assert_eq!(
            Position::new(1, 2),
            Position {
                line: 1,
                character: 2
            }
        );

        assert_eq!(
            Position::from((1, 2)),
            Position {
                line: 1,
                character: 2
            }
        );
        assert_eq!(
            Position::from((10, 10)),
            Position {
                line: 10,
                character: 10
            }
        );
        assert_eq!(
            Position::from((0, 0)),
            Position {
                line: 0,
                character: 0
            }
        );
    }

    #[test]
    fn test_position_display() {
        let pos = Position::new(0, 0);
        assert_eq!(pos.to_string(), "1:1");

        let pos = Position::new(4, 10);
        assert_eq!(pos.to_string(), "5:11");
    }
}

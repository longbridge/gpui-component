use std::{
    cmp::Ordering,
    fmt,
    ops::{Add, Deref, Range, Sub},
};

/// Cursor of the text.
#[derive(Debug, Copy, Clone, Default)]
pub struct Cursor {
    /// The byte offset in the text (zero-based).
    pub(super) offset: usize,
}

impl Cursor {
    pub fn new(offset: usize) -> Self {
        Self { offset }
    }

    /// Returns the byte offset in the text (zero-based).
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl Eq for Cursor {}
impl PartialEq for Cursor {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}
impl PartialEq<usize> for Cursor {
    fn eq(&self, other: &usize) -> bool {
        self.offset == *other
    }
}
impl PartialEq<Cursor> for usize {
    fn eq(&self, other: &Cursor) -> bool {
        *self == other.offset
    }
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}
impl PartialOrd<usize> for Cursor {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        self.offset.partial_cmp(other)
    }
}
impl PartialOrd<Cursor> for usize {
    fn partial_cmp(&self, other: &Cursor) -> Option<Ordering> {
        self.partial_cmp(&other.offset)
    }
}

impl Add for Cursor {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self.offset += other.offset;
        self
    }
}
impl Add<usize> for Cursor {
    type Output = Self;

    fn add(mut self, other: usize) -> Self {
        self.offset += other;
        self
    }
}
impl Add<Cursor> for usize {
    type Output = Cursor;

    fn add(self, other: Cursor) -> Cursor {
        Cursor::new(self + other.offset)
    }
}

impl Sub for Cursor {
    type Output = Self;

    fn sub(mut self, other: Self) -> Self {
        self.offset -= other.offset;
        self
    }
}
impl Sub<usize> for Cursor {
    type Output = Self;

    fn sub(mut self, other: usize) -> Self {
        self.offset -= other;
        self
    }
}
impl Sub<Cursor> for usize {
    type Output = Cursor;

    fn sub(self, other: Cursor) -> Cursor {
        Cursor::new(self - other.offset)
    }
}

impl Deref for Cursor {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.offset
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub start: Cursor,
    pub end: Cursor,
}

impl Selection {
    pub fn new(start: Cursor, end: Cursor) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.offset.saturating_sub(self.start.offset)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl From<Range<Cursor>> for Selection {
    fn from(value: Range<Cursor>) -> Self {
        Self::new(value.start, value.end)
    }
}
impl From<Selection> for Range<Cursor> {
    fn from(value: Selection) -> Self {
        value.start..value.end
    }
}
impl From<Range<usize>> for Selection {
    fn from(value: Range<usize>) -> Self {
        Self::new(Cursor::new(value.start), Cursor::new(value.end))
    }
}
impl From<Selection> for Range<usize> {
    fn from(value: Selection) -> Self {
        value.start.offset..value.end.offset
    }
}
impl From<&Selection> for Range<usize> {
    fn from(value: &Selection) -> Self {
        value.start.offset..value.end.offset
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

impl From<(usize, usize)> for LineColumn {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0.max(1),
            column: value.1.max(1),
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
    fn test_line_column_display() {
        assert_eq!(LineColumn::from((1, 2)).to_string(), "1:2");
        assert_eq!(LineColumn::from((10, 10)).to_string(), "10:10");
        assert_eq!(LineColumn::from((0, 0)).to_string(), "1:1");
    }
}

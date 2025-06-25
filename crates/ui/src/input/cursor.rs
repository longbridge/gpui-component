use std::{
    cmp::Ordering,
    ops::{Add, Deref, Range, Sub},
};

/// Cursor of the text.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Cursor {
    /// The byte offset.
    pub(super) offset: usize,
    /// Whether the cursor is before or after the offset, default: false.
    pub(super) after: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            offset: 0,
            after: false,
        }
    }
}

impl Cursor {
    pub fn new(offset: usize) -> Self {
        Self {
            offset,
            after: false,
        }
    }

    pub fn after(mut self) -> Self {
        self.after = true;
        self
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
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

/// A rectangle structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Rect<T> {
    /// X coordinate of the rectangle.
    pub x: T,
    /// Y coordinate of the rectangle.
    pub y: T,
    /// Width of the rectangle.
    pub width: T,
    /// Height of the rectangle.
    pub height: T,
}

impl<T> Rect<T> {
    /// Creates a new rectangle with the specified coordinates and size.
    #[inline]
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the origin point of the rectangle.
    #[inline]
    pub fn origin(&self) -> Point<T>
    where
        T: Copy,
    {
        Point::new(self.x, self.y)
    }

    /// Returns the size of the rectangle.
    pub fn size(&self) -> Size<T>
    where
        T: Copy,
    {
        Size::new(self.width, self.height)
    }

    /// Maps the rectangle to a new type using the provided function.
    pub fn map<F, U>(&self, f: F) -> Rect<U>
    where
        T: Copy,
        F: Fn(T) -> U,
    {
        Rect {
            x: f(self.x),
            y: f(self.y),
            width: f(self.width),
            height: f(self.height),
        }
    }
}

/// A point structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Point<T> {
    /// X coordinate of the point.
    pub x: T,
    /// Y coordinate of the point.
    pub y: T,
}

impl<T> Point<T> {
    /// Creates a new point with the specified coordinates.
    #[inline]
    pub fn new(x: T, y: T) -> Self {
        Point { x, y }
    }

    /// Maps the point to a new type using the provided function.
    pub fn map<F, U>(&self, f: F) -> Point<U>
    where
        T: Copy,
        F: Fn(T) -> U,
    {
        Point {
            x: f(self.x),
            y: f(self.y),
        }
    }
}

/// A size structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Size<T> {
    /// Width of the size.
    pub width: T,
    /// Height of the size.
    pub height: T,
}

impl<T> Size<T> {
    /// Creates a new size with the specified width and height.
    #[inline]
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }

    /// Maps the size to a new type using the provided function.
    pub fn map<F, U>(&self, f: F) -> Size<U>
    where
        T: Copy,
        F: Fn(T) -> U,
    {
        Size {
            width: f(self.width),
            height: f(self.height),
        }
    }
}

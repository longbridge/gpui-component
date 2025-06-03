use num_enum::TryFromPrimitive;

use crate::{ImageBuffer, Point};

/// Cursor type values.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum CursorType {
    Pointer = 0,
    Cross = 1,
    Hand = 2,
    IBeam = 3,
    Wait = 4,
    Help = 5,
    EastResize = 6,
    NorthResize = 7,
    NorthEastResize = 8,
    NorthWestResize = 9,
    SouthResize = 10,
    SouthEastResize = 11,
    SouthWestResize = 12,
    WestResize = 13,
    NorthSouthResize = 14,
    EastWestResize = 15,
    NorthEastSouthWestResize = 16,
    NorthWestSouthEastResize = 17,
    ColumnResize = 18,
    RowResize = 19,
    MiddlePanning = 20,
    EastPanning = 21,
    NorthPanning = 22,
    NorthEastPanning = 23,
    NorthWestPanning = 24,
    SouthPanning = 25,
    SouthEastPanning = 26,
    SouthWestPanning = 27,
    WestPanning = 28,
    Move = 29,
    VerticalText = 30,
    Cell = 31,
    ContextMenu = 32,
    Alias = 33,
    Progress = 34,
    NoDrop = 35,
    Copy = 36,
    None = 37,
    NotAllowed = 38,
    ZoomIn = 39,
    ZoomOut = 40,
    Grab = 41,
    Grabbing = 42,
    MiddlePanningVertical = 43,
    MiddlePanningHorizontal = 44,
    Custom = 45,
    DndNone = 46,
    DndMove = 47,
    DndCopy = 48,
    DndLink = 49,
}

/// Representing cursor information.
#[derive(Debug)]
pub struct CursorInfo<'a> {
    /// The hotspot of the cursor, which is the point in the image that will be
    /// used as the actual cursor position.
    pub hotspot: Point<i32>,
    /// The scale factor of the cursor.
    pub scale_factor: f32,
    /// The image buffer of the cursor.
    pub image: ImageBuffer<'a>,
}

//! Presentation hook for an editor's existing viewport, without a second scroll system.
use crate::{ScrollbarAxis, ScrollbarHandle};
use gpui::{AnyElement, App, Bounds, Pixels, Point, ScrollHandle, Size, Window};
use std::rc::Rc;

/// Current-frame editor geometry and its retained scroll offset.
/// The editor applies caret scrolling before invoking the presentation callback.
#[derive(Clone)]
pub struct EditorScrollbarContext {
    pub(crate) handle: ScrollHandle,
    pub(crate) viewport: Bounds<Pixels>,
    pub(crate) scroll_viewport: Size<Pixels>,
    pub(crate) content: Size<Pixels>,
    pub(crate) axis: ScrollbarAxis,
}

impl EditorScrollbarContext {
    pub fn axis(&self) -> ScrollbarAxis {
        self.axis
    }

    /// The viewport without the editor's padding. The scroll range is the
    /// content size minus this size; `viewport_bounds` keeps the padding and
    /// is where the scrollbar's track belongs.
    pub fn scroll_viewport_size(&self) -> Size<Pixels> {
        self.scroll_viewport
    }
}

impl ScrollbarHandle for EditorScrollbarContext {
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        self.viewport
    }
    fn content_size(&self) -> Size<Pixels> {
        self.content
    }
    fn offset(&self) -> Point<Pixels> {
        self.handle.offset()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        self.handle.set_offset(offset);
    }
}

/// Supply a scrollbar from the consuming UI library. None retains Base's default.
pub type EditorScrollbarRenderer =
    Rc<dyn Fn(EditorScrollbarContext, &mut Window, &mut App) -> AnyElement>;

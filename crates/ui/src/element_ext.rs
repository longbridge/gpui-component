use gpui::{
    AnyElement, App, Bounds, IntoElement, ParentElement, Pixels, Styled as _, Window, canvas,
};

use crate::{Sizable, Size};

/// A type-erased element that can accept a [`Size`] before being rendered.
pub(crate) struct AnySizableElement(Box<dyn FnOnce(Size) -> AnyElement>);

impl AnySizableElement {
    pub fn new(element: impl IntoElement + Sizable + 'static) -> Self {
        Self(Box::new(|size| element.with_size(size).into_any_element()))
    }

    pub fn into_any(self, size: Size) -> AnyElement {
        (self.0)(size)
    }
}

/// A trait to extend [`gpui::Element`] with additional functionality.
pub trait ElementExt: ParentElement + Sized {
    /// Add a prepaint callback to the element.
    ///
    /// This is a helper method to get the bounds of the element after paint.
    ///
    /// The first argument is the bounds of the element in pixels.
    ///
    /// See also [`gpui::canvas`].
    fn on_prepaint<F>(self, f: F) -> Self
    where
        F: FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    {
        self.child(
            canvas(move |bounds, window, cx| f(bounds, window, cx), |_, _, _, _| {})
                .absolute()
                .size_full(),
        )
    }
}

impl<T: ParentElement> ElementExt for T {}

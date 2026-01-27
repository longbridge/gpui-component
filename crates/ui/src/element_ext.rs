use crate::focus_trap::FocusTrapElement;
use gpui::{
    App, Bounds, ElementId, FocusHandle, IntoElement, ParentElement, Pixels, Styled as _, Window,
    canvas,
};

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
            canvas(
                move |bounds, window, cx| f(bounds, window, cx),
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
    }

    /// Enable focus trap for this element.
    ///
    /// When enabled, focus will automatically cycle within this container
    /// instead of escaping to parent elements. This is useful for modal dialogs,
    /// sheets, and other overlay components.
    ///
    /// The focus trap works by:
    /// 1. Registering this element as a focus trap container
    /// 2. When Tab/Shift-Tab is pressed, Root intercepts the event
    /// 3. If focus would leave the container, it cycles back to the beginning/end
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for this focus trap
    /// * `cx` - The context to create a focus handle for the container
    ///
    /// # Example
    ///
    /// ```ignore
    /// v_flex()
    ///     .child(Button::new("btn1").label("Button 1"))
    ///     .child(Button::new("btn2").label("Button 2"))
    ///     .child(Button::new("btn3").label("Button 3"))
    ///     .focus_trap("my-trap", self.trap_handle.clone())
    /// // Pressing Tab will cycle: btn1 -> btn2 -> btn3 -> btn1
    /// // Focus will not escape to elements outside this container
    /// ```
    ///
    /// See also: <https://github.com/focus-trap/focus-trap-react>
    fn focus_trap(self, id: impl Into<ElementId>, handle: FocusHandle) -> FocusTrapElement
    where
        Self: IntoElement,
    {
        FocusTrapElement::new(id, handle, self)
    }
}

impl<T: ParentElement> ElementExt for T {}

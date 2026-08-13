use gpui::{App, ClickEvent, InteractiveElement, Stateful, StatefulInteractiveElement, Window};

pub trait InteractiveElementExt: InteractiveElement {
    /// Locks scrolling to the gesture's dominant axis, where the platform supports it.
    ///
    /// gpui delimits scroll gestures with `std::time::Instant`, which is
    /// unimplemented on wasm32: the first wheel event over an axis-locked
    /// scroll area panics with "time not implemented on this platform" and
    /// takes the whole application down, leaving the canvas unresponsive.
    /// Losing the axis lock in the browser is by far the lesser cost, so this
    /// is a no-op there.
    fn lock_scroll_axis(self) -> Self
    where
        Self: Sized + StatefulInteractiveElement,
    {
        #[cfg(target_family = "wasm")]
        {
            self
        }
        #[cfg(not(target_family = "wasm"))]
        {
            self.restrict_scroll_to_axis()
        }
    }

    /// Set the listener for a double click event.
    fn on_double_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_click(move |event, window, cx| {
            if event.click_count() == 2 {
                listener(event, window, cx);
            }
        });
        self
    }
}

impl<E: InteractiveElement> InteractiveElementExt for Stateful<E> {}
